use std::fs;
use std::io::{Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

// === Hub paths ===
// 复用 scanner 中唯一的 hub_resources_path 实现，避免重复
use crate::scanner::hub_resources_path;

fn hub_asar_path() -> PathBuf {
    hub_resources_path().join("app.asar")
}

/// 备份文件位置。
///
/// - Windows / Linux：与目标同目录的 `<file>.bak`（保持原有行为）。
/// - macOS：备份**不能**留在 `.app` 包内——包里多出任何未被封签的文件都会让
///   `codesign` 报 "unsealed contents" 并使签名失效。因此统一放到
///   `~/Library/Application Support/UniFree/backups/<路径转写>/<文件名>`。
pub fn backup_path(path: &Path) -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        let base = dirs::data_dir()
            .unwrap_or_else(std::env::temp_dir)
            .join("UniFree")
            .join("backups");
        let key: String = path
            .to_string_lossy()
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
            .collect();
        let dir = base.join(key);
        let _ = fs::create_dir_all(&dir);
        dir.join(path.file_name().unwrap_or_else(|| std::ffi::OsStr::new("backup")))
    }
    #[cfg(not(target_os = "macos"))]
    {
        PathBuf::from(format!("{}.bak", path.display()))
    }
}


/// ValidateSignature 包装函数的字节锚点（用于检测 Native AOT exe 是否已补丁）
const VALIDATE_SIG_PATTERN: &str = "57 56 53 48 83 EC 20 48 8B DA 48 85 DB 74 ?? 48 8B 71 10 40 0F B6 79 18 48 8D 15 ?? ?? ?? ?? 48 39 11 74 ?? 48 8D 15 ?? ?? ?? ?? 48 39 11 75 ?? 48 8B D3";

/// Unity 2019.4 编辑器中 `LicensingClient::ValidateServerProcess` 的决策点锚点。
/// 2019.x 的编辑器在启动 Licensing Client 前会对其做 Authenticode 签名校验
/// （WinVerifyTrust + 与内置 Unity 证书比对）。其签名证书在 2024 年过期后，
/// 校验失败 → 编辑器回退到 Legacy licensing → 原生 WinILicensingAdapter 校验 ULF 失败，
/// 最终 "Unity license information is invalid"。补丁把 `cmp edi, 9`（结果==9 成功）
/// 改为 `cmp edi, edi`（恒等，ZF 恒为 1），使紧随其后的 `je` 恒跳转到成功分支，
/// 从而无条件接受 Licensing Client（其 ULF 校验已由替换的 DLL 绕过）。
/// 锚点含 je 的相对偏移与错误字符串的 LEA 相对偏移（均用 ?? 通配），
/// 在同一条 2019.4 发布线内跨小版本兼容。
const VALIDATE_SERVER_PROCESS_PATTERN: &str = "83 FF 09 0F 84 ?? ?? ?? ?? 49 8B CF 48 8D 15 ?? ?? ?? ?? 80 3C 11 00";

/// Unity 2019.4 编辑器 `Licensing` 错误分发器（switch dispatcher）里
/// 读取当前错误码的 `call [rax+0xD8]`。编辑器原生 `WinILicensingAdapter` 独立校验 ULF
/// （原生 RSA 验签），失败时错误码 == 2 → "Unity license information is invalid" → 编辑器退出。
/// 补丁把 `call [rax+0xD8]`（FF 90 D8 00 00 00）改为 `xor eax, eax`（31 C0 + 4×nop），
/// 使错误码恒为 0（成功），从而不再产生 "license invalid" 错误。
const LICENSE_SYSTEM_ERROR_PATTERN: &str = "48 8B 01 FF 90 D8 00 00 00 83 F8 50";

/// Check EntitlementResolver DLL status
pub fn get_editor_dll_status(dll_path: &str) -> String {
    let path = Path::new(dll_path);
    if !path.exists() {
        return "not_found".into();
    }

    // 检查是否有备份文件（表示已补丁）
    let bak_path = backup_path(Path::new(dll_path));
    if Path::new(&bak_path).exists() {
        return "patched".into();
    }

    // 无备份时通过内容比对判断是否已补丁（不依赖文件大小，避免跨版本误判）
    let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or_default();
    let Ok(data) = fs::read(path) else {
        return "unknown".into();
    };

    // macOS：launcher shim 存在说明已执行过补丁（R2R 关闭 + IL 已改）
    #[cfg(target_os = "macos")]
    {
        if let Some(dir) = path.parent() {
            if dir.join("Unity.Licensing.Client.real").exists() {
                return "patched".into();
            }
        }
    }

    // 通用：Unity.Licensing.EntitlementResolver.dll 是托管程序集，与平台无关，
    // 用 IL 结构探测（ValidateSignature 的 4 条指令是否已被改写）判断补丁状态。
    // macOS 上不存在预编译替换 DLL，必须走这条路径。
    if file_name == "Unity.Licensing.EntitlementResolver.dll" {
        #[cfg(target_os = "macos")]
        let probe = crate::il_patch::patch_resolver_keep_r2r(&data);
        #[cfg(not(target_os = "macos"))]
        let probe = crate::il_patch::patch_resolver(&data);
        if let Ok((_, report)) = probe {
            // 先看原始错误块是否还在（原始文件），再看补丁窗口：
            // 未补丁的 DLL 里也可能出现形似补丁窗口的字节，顺序颠倒会误报 patched。
            if report.patched_sites > 0 {
                return "original".into();
            }
            if report.already_patched_sites > 0 {
                return "patched_no_backup".into();
            }
        }
    }

    let patched = match file_name {
        "System.Security.Cryptography.Xml.dll" => {
            data.as_slice() == include_bytes!("../resources/win/System.Security.Cryptography.Xml.dll").as_slice()
        }
        "Unity.Licensing.EntitlementResolver.dll" => {
            data.as_slice() == include_bytes!("../resources/win/Unity.Licensing.EntitlementResolver.dll").as_slice()
                || data.as_slice()
                    == include_bytes!("../resources/win/Unity.Licensing.EntitlementResolver.1.17.4.dll").as_slice()
        }
        "Unity.Licensing.Client.exe" => {
            // Native AOT：定位 ValidateSignature 包装函数，检查函数头是否为 mov eax,1; ret
            find_pattern(&data, VALIDATE_SIG_PATTERN)
                .map(|off| data.get(off as usize..off as usize + 6) == Some(&[0xB8, 0x01, 0x00, 0x00, 0x00, 0xC3][..]))
                .unwrap_or(false)
        }
        _ => false,
    };
    if patched { "patched_no_backup".into() } else { "original".into() }
}

/// 通配符字节搜索，返回文件偏移
fn find_pattern(data: &[u8], pattern: &str) -> Option<u64> {
    find_pattern_at(data, pattern).map(|o| o as u64)
}

/// 在切片中搜索通配符字节模式，返回切片内偏移
fn find_pattern_at(data: &[u8], pattern: &str) -> Option<usize> {
    let parts: Vec<&str> = pattern.split_whitespace().collect();
    let bytes: Vec<Option<u8>> = parts
        .iter()
        .map(|b| {
            if *b == "??" {
                None
            } else {
                Some(u8::from_str_radix(b, 16).unwrap())
            }
        })
        .collect();

    'outer: for i in 0..=data.len().saturating_sub(bytes.len()) {
        for (j, b) in bytes.iter().enumerate() {
            if let Some(expected) = b {
                if data[i + j] != *expected {
                    continue 'outer;
                }
            }
        }
        return Some(i);
    }
    None
}

/// 对 Native AOT 编译的 Unity.Licensing.Client.exe (Unity >= 6000.7) 做字节级补丁，
/// 绕过许可证 XML 数字签名验证。通过锚点+模式匹配定位，兼容小版本更新。
pub fn patch_licensing_client(exe_path: &str) -> Result<String, String> {
    let path = Path::new(exe_path);
    if !path.exists() {
        return Err("Unity.Licensing.Client.exe not found".into());
    }

    // 首次创建备份，之后每次从备份恢复确保干净状态
    let bak = backup_path(Path::new(exe_path));
    let bak_path = Path::new(&bak);
    if !bak_path.exists() {
        fs::copy(path, &bak).map_err(|e| e.to_string())?;
    }
    fs::copy(bak_path, path).map_err(|e| format!("Failed to restore from backup: {}", e))?;

    let data = fs::read(path).map_err(|e| format!("Failed to read: {}", e))?;
    let mut patches: Vec<(u64, Vec<u8>)> = Vec::new();

    // 唯一补丁：XmlExtensions.ValidateSignature 包装函数 (sub_1404F1C10) → 始终返回 1
    // Parse 流程调用此函数进行签名验证但忽略返回值，失败路径通过异常/__debugbreak 中断。
    // 将函数头改为 `mov eax,1; ret` 后，整个验证链路（门控、CheckSignature、公钥比较、
    // LABEL_14 证书信任检查）全部短路，签名 ULF 保持原样即被接受。
    // 锚点包含函数头 + 配置字段读取 ([rcx+10h] cert、[rcx+18h] checkEnabled) + 双重类型检查，
    // 对绝对地址（lea rel32）使用 ?? 通配，同发布线内跨小版本兼容。
    // 注：cmp [rcx],rdx 编码为 48 39 11（ModRM 0x11，rm=rcx），不要误写成 17（那是 [rdi]）。
    let p1 = find_pattern(&data, VALIDATE_SIG_PATTERN)
        .ok_or("P1: ValidateSignature wrapper not found")?;
    patches.push((p1, vec![0xB8, 0x01, 0x00, 0x00, 0x00, 0xC3]));

    // 注：早期补丁已由本单点补丁取代（见 git 历史 / docs）：
    //   - v2.5.0 起：P1 证书链 / P2 门控 / P3 LABEL_14 / P4 BCrypt-NCrypt（4 补丁）
    //   - 精简 v1：去掉冗余的 P1/P4（P2+P3 两补丁）
    //   - 精简 v2（当前）：直接短路 ValidateSignature 包装函数（1 补丁）

    // 写入
    let mut f = fs::OpenOptions::new().write(true).open(path).map_err(|e| e.to_string())?;
    for (off, bytes) in &patches {
        f.seek(SeekFrom::Start(*off)).map_err(|e| e.to_string())?;
        f.write_all(bytes).map_err(|e| e.to_string())?;
    }
    Ok(format!("Patched {} areas", patches.len()))
}

/// Patch EntitlementResolver.dll by replacing with pre-patched version
/// The pre-patched DLL has ValidateSignature bypassed
pub fn patch_entitlement_resolver(dll_path: &str, display_version: Option<&str>) -> Result<String, String> {
    // 6000.7+: Native AOT exe，字节级 patch
    if dll_path.ends_with(".exe") {
        return patch_licensing_client(dll_path);
    }

    // macOS：仅支持 Unity 6（6000.0-6000.6）的就地 IL 补丁。
    // 该 DLL 是跨平台的托管程序集，IL 结构与 Windows 完全一致，补丁方法可直接复用。
    // 更老的版本（2019-2022）依赖 Windows 侧预编译 DLL 替换，macOS 暂不覆盖；
    // 6000.7+ 的 Native AOT 客户端是 arm64 Mach-O，需要独立锚点，暂不覆盖。
    #[cfg(target_os = "macos")]
    {
        let path = Path::new(dll_path);
        if !path.exists() {
            return Err("Licensing client not found".into());
        }
        let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or_default();
        if file_name == "Unity.Licensing.Client" {
            // Unity 6000.3.28+ / 6000.7+ 的 macOS licensing client 是 Native AOT（arm64 Mach-O，
            // 目录里没有托管 DLL），无法走 IL 补丁，需要独立的 arm64 原生锚点。
            return Err(
                "macOS: this editor ships a Native AOT licensing client (no managed DLLs) \
                 - native arm64 patch not implemented yet; managed clients (e.g. 6000.3.21-27) are supported"
                    .into(),
            );
        }
        if file_name != "Unity.Licensing.EntitlementResolver.dll" {
            return Err(format!("Unsupported Unity licensing target on macOS: {}", file_name));
        }
        let msg = patch_entitlement_resolver_inplace(dll_path)?;
        let shim_note = match ensure_r2r_env_shim(path.parent().unwrap_or(Path::new(""))) {
            Ok(true) => "，已为 licensing client 安装 DOTNET_ReadyToRun=0 启动器",
            Ok(false) => "",
            Err(e) => {
                eprintln!("⚠ macOS: launcher shim not installed: {}", e);
                "，⚠ 启动器 shim 未安装（补丁可能不生效）"
            }
        };
        resign_editor_licensing_client(path);
        return Ok(format!("{} (macOS in-place IL patch){}", msg, shim_note));
    }
    let path = Path::new(dll_path);
    if !path.exists() {
        return Err("DLL not found".into());
    }

    // Select the replacement by target filename, not by the install directory.
    let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or_default();

    // IL resolver：优先"就地字节级补丁"（保留程序集身份，1.17.x / 1.18+ / 未来任何
    // LocalIPC 线通用，无需按线挑选预编译 DLL）；命不中才回退到按发行线匹配的
    // 预编译补丁 DLL（原逻辑，见下）。
    if file_name == "Unity.Licensing.EntitlementResolver.dll" {
        match patch_entitlement_resolver_inplace(dll_path) {
            Ok(msg) => return Ok(msg),
            Err(e) => {
                eprintln!(
                    "⚠ In-place IL patch not applicable ({}); falling back to precompiled resolver",
                    e
                );
            }
        }
    }

    let target_version = display_version.unwrap_or("unknown");
    let (target_kind, patched_dll): (&str, &[u8]) = match file_name {
        "System.Security.Cryptography.Xml.dll" => (
            "2019-2022",
            include_bytes!("../resources/win/System.Security.Cryptography.Xml.dll"),
        ),
        "Unity.Licensing.EntitlementResolver.dll" => {
            // 预编译补丁 DLL 必须与 editor 自带 licensing client 的 LocalIPC 线匹配：
            // - LocalIPC 1.17.x（如 6000.3.10f1）：原版 EntitlementResolver 程序集版本为
            //   1.17.4.0（引用 v7 运行时），必须使用基于 1.17.4 原版生成的补丁 DLL
            //   （保留程序集版本 1.17.4.0），否则 client 启动即抛
            //   "Could not load file or assembly 'Unity.Licensing.EntitlementResolver,
            //    Version=1.17.4.0'"。
            // - LocalIPC 1.18+（如 6000.3.20f1/22f1）：原版解析器程序集版本为 0.0.0.0
            //   （引用 v8 运行时），使用当前捆绑的 0.0.0.0 补丁 DLL。
            // 版本线从 Unity.Licensing.Client.deps.json 中的 "Unity.Licensing.Client/<ver>" 读取。
            let client_dir = path.parent().map(|p| p.to_path_buf()).unwrap_or_default();
            if licensing_client_version(&client_dir)
                .map(|v| v.starts_with("1.17"))
                .unwrap_or(false)
            {
                (
                    "6000.0-6000.6 (LocalIPC 1.17.x)",
                    include_bytes!("../resources/win/Unity.Licensing.EntitlementResolver.1.17.4.dll"),
                )
            } else {
                (
                    "6000.0-6000.6 (LocalIPC 1.18+)",
                    include_bytes!("../resources/win/Unity.Licensing.EntitlementResolver.dll"),
                )
            }
        }
        _ => return Err(format!("Unsupported Unity licensing target: {}", file_name)),
    };

    // 2019.x 编辑器：Licensing Client 的 Authenticode 签名证书在 2024 年过期，
    // 编辑器原生 `ValidateServerProcess` 校验失败后回退到 Legacy licensing（原生），
    // 导致 ULF 校验失败。因此必须先补丁 Unity.exe 使其接受 Licensing Client，
    // 其 ULF 校验再由下面替换的 DLL 绕过。
    if file_name == "System.Security.Cryptography.Xml.dll" && major_version(target_version) == Some(2019) {
        let exe_path = unity_exe_from_dll(dll_path)
            .ok_or("Cannot derive Unity.exe path from licensing DLL path")?;
        patch_unity_exe_validate_server_process(exe_path.to_string_lossy().as_ref())?;
    }

    // 创建备份
    let bak_path = backup_path(Path::new(dll_path));
    if !Path::new(&bak_path).exists() {
        fs::copy(path, &bak_path).map_err(|e| e.to_string())?;
    }

    // 写入版本对应的补丁DLL
    fs::write(path, patched_dll).map_err(|e| format!("Failed to write patched DLL: {}", e))?;

    Ok(format!(
        "Patched: replaced with pre-patched DLL for Unity {} ({})",
        target_version, target_kind
    ))
}

/// 就地 IL 字节补丁 resolver：任意 LocalIPC 线通用（程序集身份、方法表、局部变量、
/// 异常处理子句全部原样保留，只改写 ValidateSignature 内 4 条指令）。
/// 幂等：已处于补丁状态（含已被预编译补丁 DLL 替换过的情况）直接返回成功。
fn patch_entitlement_resolver_inplace(dll_path: &str) -> Result<String, String> {
    let path = Path::new(dll_path);
    let data = fs::read(path).map_err(|e| format!("Failed to read resolver: {}", e))?;
    // macOS：必须保留 R2R 头（清掉会让 .NET 宿主完全不加载该程序集），
    // 改为让 licensing client 以 DOTNET_ReadyToRun=0 启动（见 ensure_r2r_env_shim）。
    #[cfg(target_os = "macos")]
    let patch_result = crate::il_patch::patch_resolver_keep_r2r(&data);
    #[cfg(not(target_os = "macos"))]
    let patch_result = crate::il_patch::patch_resolver(&data);
    match patch_result {
        Ok((patched, report)) if report.patched_sites > 0 => {
            let bak = backup_path(Path::new(dll_path));
            if !Path::new(&bak).exists() {
                fs::copy(path, &bak).map_err(|e| format!("Failed to backup resolver: {}", e))?;
            }
            fs::write(path, &patched)
                .map_err(|e| format!("Failed to write patched resolver: {}", e))?;
            Ok(format!(
                "Patched in-place (IL byte patch, {} site(s)); assembly identity preserved, works for any LocalIPC line",
                report.patched_sites
            ))
        }
        Ok((_, report)) => Ok(format!(
            "Already patched in-place ({} site(s) detected)",
            report.already_patched_sites
        )),
        Err(e) => Err(format!("in-place IL patch not applicable: {}", e)),
    }
}

/// 从 Unity.Licensing.Client.deps.json 读取 licensing client 发行线版本
/// （JSON 库键形如 "Unity.Licensing.Client/1.17.4"），用于选择匹配的补丁 DLL。
fn licensing_client_version(client_dir: &Path) -> Option<String> {
    const MARKER: &str = "Unity.Licensing.Client/";
    let content = fs::read_to_string(client_dir.join("Unity.Licensing.Client.deps.json")).ok()?;
    let start = content.find(MARKER)? + MARKER.len();
    let rest = &content[start..];
    let end = rest.find(|c: char| !(c.is_ascii_digit() || c == '.'))?;
    let ver = &rest[..end];
    (!ver.is_empty()).then(|| ver.to_string())
}

/// 解析版本号的主版本号（"2019.4.40f1" / "2019.4.40f1_ffc62b691db5" → 2019）
fn major_version(version: &str) -> Option<u32> {
    version
        .split(|c: char| !c.is_ascii_digit() && c != '.')
        .find(|t| !t.is_empty() && t.chars().next().is_some_and(|c| c.is_ascii_digit()))
        .and_then(|t| t.split('.').next())
        .and_then(|s| s.parse::<u32>().ok())
}

/// 从 licensing DLL 路径推导 Unity.exe 路径
/// DLL: {EditorRoot}\Editor\Data\Resources\Licensing\Client\xxx.dll
/// EXE: {EditorRoot}\Editor\Unity.exe
fn unity_exe_from_dll(dll_path: &str) -> Option<PathBuf> {
    let path = Path::new(dll_path);
    let editor_dir = path.parent()?.parent()?.parent()?.parent()?.parent()?;
    let exe = editor_dir.join("Unity.exe");
    exe.exists().then_some(exe)
}

/// 补丁 Unity.exe 的 `LicensingClient::ValidateServerProcess`（Unity 2019.x 专用）。
/// 将 `cmp edi, 9`（校验结果==9 才算成功）改为 `cmp edi, edi`（恒等，ZF 恒为 1），
/// 使紧随其后的 `je` 恒跳转到成功分支，从而无条件接受 Licensing Client，
/// 跳过已失效的 Authenticode 签名校验（证书过期后 WinVerifyTrust 失败）。
pub fn patch_unity_exe_validate_server_process(exe_path: &str) -> Result<String, String> {
    let path = Path::new(exe_path);
    if !path.exists() {
        return Err("Unity.exe not found".into());
    }

    // 首次创建备份，之后每次从备份恢复确保干净状态
    let bak = backup_path(Path::new(exe_path));
    let bak_path = Path::new(&bak);
    if !bak_path.exists() {
        fs::copy(path, &bak).map_err(|e| e.to_string())?;
    }
    fs::copy(bak_path, path).map_err(|e| format!("Failed to restore from backup: {}", e))?;

    let data = fs::read(path).map_err(|e| format!("Failed to read: {}", e))?;

    // 补丁点 1：ValidateServerProcess 决策点（cmp edi,9 → cmp edi,edi）
    let p1 = find_pattern(&data, VALIDATE_SERVER_PROCESS_PATTERN)
        .ok_or("ValidateServerProcess anchor not found (editor may not be Unity 2019.4)")?;

    // 补丁点 2：Licensing 错误分发器（call [rax+0xD8] → xor eax,eax）
    let p2 = find_pattern(&data, LICENSE_SYSTEM_ERROR_PATTERN)
        .ok_or("LICENSE SYSTEM error dispatcher anchor not found (editor may not be Unity 2019.4)")?;

    let mut f = fs::OpenOptions::new().write(true).open(path).map_err(|e| e.to_string())?;
    // P1：cmp edi,9（83 FF 09）→ cmp edi,edi + nop（3B FF 90）
    f.seek(SeekFrom::Start(p1)).map_err(|e| e.to_string())?;
    f.write_all(&[0x3B, 0xFF, 0x90]).map_err(|e| e.to_string())?;
    // P2：call [rax+0xD8]（FF 90 D8 00 00 00）→ xor eax,eax + 4×nop（31 C0 90 90 90 90）
    f.seek(SeekFrom::Start(p2 + 3)).map_err(|e| e.to_string())?;
    f.write_all(&[0x31, 0xC0, 0x90, 0x90, 0x90, 0x90]).map_err(|e| e.to_string())?;
    Ok("Patched ValidateServerProcess + LICENSE SYSTEM dispatcher (2 areas)".into())
}

/// Restore DLL from backup
pub fn restore(dll_path: &str) -> Result<String, String> {
    let bak_path = backup_path(Path::new(dll_path));
    let bak = Path::new(&bak_path);
    if !bak.exists() {
        return Err("Backup not found".into());
    }
    fs::copy(bak, dll_path).map_err(|e| e.to_string())?;
    fs::remove_file(bak).map_err(|e| e.to_string())?;

    // 若曾补丁过 Unity.exe（2019.x），一并恢复
    if let Some(exe_path) = unity_exe_from_dll(dll_path) {
        let exe_bak = backup_path(&exe_path);
        let exe_bak_path = Path::new(&exe_bak);
        if exe_bak_path.exists() {
            fs::copy(exe_bak_path, &exe_path).map_err(|e| format!("Failed to restore Unity.exe: {}", e))?;
            fs::remove_file(exe_bak_path).map_err(|e| e.to_string())?;
            eprintln!("✓ Restored Unity.exe");
        }
    }

    // macOS：撤销 launcher shim，并重签 licensing client 所在的 app
    #[cfg(target_os = "macos")]
    {
        if let Some(dir) = Path::new(dll_path).parent() {
            remove_r2r_env_shim(dir);
            resign_editor_licensing_client(Path::new(dll_path));
        }
    }

    // 恢复后结束接管用的 licensing client，让 Hub 的 client 重新接管全局管道
    kill_process("Unity.Licensing.Client.exe").ok();

    Ok(format!("Restored: {}", dll_path))
}



/// 检查 Hub 是否已被补丁。
///
/// **按内容判定，不再以 `.bak` 备份是否存在为依据。** 备份文件在 Hub 升级/重装后
/// 不会被安装器清掉（安装器只覆盖它自己的文件，卸载器也只删已知文件），于是会出现
/// "重新装了 Hub，UniFree 仍显示已打补丁"——而 asar 其实已被安装器还原成原版。
/// 反过来，用陈旧备份覆盖新安装的 asar 还会把旧版本的 JS 塞进新版本 Hub。
///
/// 三个补丁点独立检测，全命中 = "patched"，部分命中 = "partial"，全未命中 = "original"。
fn hub_patch_state() -> String {
    let asar_path = hub_asar_path();
    if !asar_path.exists() {
        return "error".into();
    }

    let licensing_dir = crate::scanner::hub_licensing_dir();

    let mut total = 0usize;
    let mut hits = 0usize;

    // 补丁点 1：app.asar 内的 JS（getLicense / isLicenseValid / entitlement verdict）
    total += 1;
    if asar_has_patch_marker(&asar_path) {
        hits += 1;
    }

    // 补丁点 2：Hub 自带 licensing client 的 ULF 签名校验（resolver IL）
    total += 1;
    if hub_resolver_patched(&licensing_dir) {
        hits += 1;
    }

    // 补丁点 3：Hub exe 的 asar 完整性 fuse（Electron 43+ 才有，缺失则不计入分母）
    if let Some(exe) = hub_exe_path(&asar_path) {
        if exe.exists() && hub_exe_fuse4(&exe).is_ok() {
            total += 1;
            if hub_exe_fuse_disabled(&exe) {
                hits += 1;
            }
        }
    }

    match hits {
        0 => "original".into(),
        h if h == total => "patched".into(),
        _ => "partial".into(),
    }
}

/// app.asar 数据区里是否含 UniFree 的 JS 补丁标记（重建 asar 时写入）
fn asar_has_patch_marker(asar_path: &Path) -> bool {
    match fs::read(asar_path) {
        Ok(data) => find_bytes(&data, b"patched by unifree").is_some(),
        Err(_) => false,
    }
}

/// Hub licensing client 的 `Unity.Licensing.EntitlementResolver.dll` 是否已补丁。
/// 首选就地 IL 结构判定（不写入，只看 ValidateSignature 的 4 条指令是否已被改写）；
/// resolver 缺失或结构不认识时，退化为老式 XML DLL 备份是否存在。
fn hub_resolver_patched(licensing_dir: &Path) -> bool {
    let resolver = licensing_dir.join("Unity.Licensing.EntitlementResolver.dll");
    if let Ok(data) = fs::read(&resolver) {
        #[cfg(target_os = "macos")]
        let probe = crate::il_patch::patch_resolver_keep_r2r(&data);
        #[cfg(not(target_os = "macos"))]
        let probe = crate::il_patch::patch_resolver(&data);
        if let Ok((_, report)) = probe {
            if report.already_patched_sites > 0 {
                return true;
            }
            if report.patched_sites > 0 {
                return false;
            }
        }
    }
    backup_path(&licensing_dir.join("System.Security.Cryptography.Xml.dll")).exists()
}

/// Check Hub status: "patched" / "partial" / "original" / "error"
pub fn get_hub_status() -> String {
    hub_patch_state()
}

/// 查找JS方法体并替换
fn replace_method_body(content: &str, method_signature: &str, new_body: &str) -> Option<String> {
    let idx = content.find(method_signature)?;
    let after = &content[idx..];
    let brace_start = after.find('{')?;
    let mut depth = 0;
    let mut end_pos = 0;
    for (i, c) in after[brace_start..].char_indices() {
        if c == '{' { depth += 1; }
        if c == '}' {
            depth -= 1;
            if depth == 0 {
                end_pos = brace_start + i + 1;
                break;
            }
        }
    }
    if end_pos == 0 { return None; }
    let old_body = &after[..end_pos];
    Some(content.replace(old_body, new_body))
}

/// Patch Hub: 只修改 app.asar 内的 JS，就地重写 asar（不替换 DLL、不解包到 app/）。
///
/// Hub 3.20.0+（Electron 43，带 OnlyLoadAppFromAsar fuse）要求 `app.asar` 文件必须存在；
/// 旧式"解包到 app/ + 改名为 .bak"会导致 Hub 启动即退出（exit code 1）。
/// 这里用 asar crate 重建 app.asar：只把 licenseService / licenseQueryService 的
/// `isLicenseValid()` 改成 `return true;`，其余文件原样保留；unpacked 的 native
/// 模块仍留在 `app.asar.unpacked/`，不写入新 asar。
pub fn patch_hub(disable_signin: bool, disable_update: bool) -> Result<String, String> {
    let asar_path = hub_asar_path();
    if !asar_path.exists() {
        return Err("app.asar not found".into());
    }

    eprintln!("Starting Hub patch (JS-only, in-place asar rewrite)...");

    // 清理旧式补丁的遗留物（app/ 解包目录 + 被替换的 XML DLL），保证干净状态
    if let Some(resources) = asar_path.parent() {
        let app_folder = resources.join("app");
        if app_folder.exists() {
            fs::remove_dir_all(&app_folder).map_err(|e| format!("Failed to remove old app folder: {}", e))?;
        }
        {
            let licensing_dir = crate::scanner::hub_licensing_dir();
            let xml_dll = licensing_dir.join("System.Security.Cryptography.Xml.dll");
            let xml_bak = backup_path(&licensing_dir.join("System.Security.Cryptography.Xml.dll"));
            if xml_bak.exists() {
                if xml_dll.exists() {
                    fs::remove_file(&xml_dll).map_err(|e| format!("Failed to remove replaced XML DLL: {}", e))?;
                }
                fs::rename(&xml_bak, &xml_dll).map_err(|e| format!("Failed to restore original XML DLL: {}", e))?;
            }
        }
    }

    // 首次创建备份，之后每次从备份恢复，确保在干净副本上补丁。
    // 升级/重装 Hub 后安装器会把 asar 还原成原版，但旧 `.bak` 会残留：
    // 若拿陈旧备份去覆盖新安装的 asar，会把上一版本的 JS/文件清单塞进新版本 Hub。
    // 因此先核对备份所属的 Hub 版本，陈旧则丢弃重建。
    let bak = backup_path(&asar_path);
    let bak_str = bak.to_string_lossy().to_string();
    let bak_path = Path::new(&bak_str);
    if bak_path.exists() && is_stale_hub_backup(bak_path, &asar_path) {
        eprintln!("⚠ app.asar.bak 属于旧版本 Hub（陈旧备份），丢弃并以当前安装为干净基线");
        fs::remove_file(bak_path).map_err(|e| format!("Failed to remove stale app.asar.bak: {}", e))?;
    }
    if !bak_path.exists() {
        fs::copy(&asar_path, bak_path).map_err(|e| format!("Failed to backup app.asar: {}", e))?;
    }
    fs::copy(bak_path, &asar_path).map_err(|e| format!("Failed to restore app.asar from backup: {}", e))?;

    // Hub 3.20.0+ 的 exe 带 EnableEmbeddedAsarIntegrityValidation fuse：
    // 不翻转的话，改动 app.asar 会在启动时触发 "Integrity check failed" FATAL。
    if let Some(hub_exe) = hub_exe_path(&asar_path) {
        match flip_hub_exe_fuses(&hub_exe) {
            Ok(flipped) => {
                if flipped {
                    eprintln!("✓ Disabled EnableEmbeddedAsarIntegrityValidation fuse in {}", hub_exe.display());
                }
            }
            Err(e) => eprintln!("⚠ Warning: failed to flip Hub exe fuse: {}", e),
        }
    }

    let patched_files = rewrite_hub_asar(&asar_path, disable_signin, disable_update)?;

    // Hub 自带 licensing client（UnityLicensingClient_V1）的 EntitlementResolver 一并打补丁
    // （LocalIPC 1.17.x 线＝Hub 3.20.0，与编辑器 6000.3.10f1 同一版本族；
    //   LocalIPC 1.18+ 线＝Hub 3.21.1 起，与编辑器 6000.3.20f1/22f1/23f1 同线）。
    // 这样"从 Hub 启动的编辑器"经由 Hub 的 licensing client 也能通过 ULF 签名校验——
    // 否则只要 Hub client 与编辑器 client 版本相同（协议匹配），编辑器会优先使用
    // Hub 的（未补丁）client → "The digital signature is invalid." → 0 entitlement → 退出 198。
    // **无需额外启动独立 IPC 进程**：Hub 启动/重启时自动拉起其 client（补丁一次永久生效）。
    let resolver_note = match patch_hub_licensing_client() {
        Ok(n) => format!("; {}", n),
        Err(e) => format!("; ⚠ Hub licensing client resolver 未打补丁: {}", e),
    };

    // 更新本地 hubConfig.json
    eprintln!("Updating local hubConfig.json...");
    if let Err(e) = crate::config_patcher::update_hub_config(disable_signin, disable_update) {
        eprintln!("⚠ Warning: Failed to update hubConfig.json: {}", e);
    } else {
        eprintln!("✓ hubConfig.json updated");
    }

    // macOS：改动 app.asar 与 Electron Framework 后原签名即失效；arm64 内核会拒绝
    // 加载签名无效的 Mach-O，因此必须对整个 Hub.app 做 ad-hoc 重签（不需要开发者证书）。
    #[cfg(target_os = "macos")]
    {
        resign_app_bundle(&crate::scanner::hub_install_dir(), "Unity Hub.app")?;
        eprintln!("✓ Re-signed Unity Hub.app (ad-hoc)");
    }

    Ok(format!(
        "Hub patched: {} JS files modified in-place in app.asar (asar kept intact);{}",
        patched_files, resolver_note
    ))
}

/// macOS：对 `.app` 做 ad-hoc 重签（`codesign --force --deep --sign -`），
/// 并清除 quarantine 标记。改动包内任何文件后必须执行，否则签名校验会让 app 无法启动。
#[cfg(target_os = "macos")]
pub fn resign_app_bundle(app: &Path, label: &str) -> Result<(), String> {
    let _ = std::process::Command::new("xattr")
        .args(["-dr", "com.apple.quarantine"])
        .arg(app)
        .status();
    let out = std::process::Command::new("codesign")
        .args(["--force", "--deep", "--sign", "-"])
        .arg(app)
        .output()
        .map_err(|e| format!("failed to run codesign: {}", e))?;
    if !out.status.success() {
        return Err(format!(
            "codesign failed for {}: {}",
            label,
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(())
}

/// macOS：把 licensing client 的可执行文件包一层 shell shim，注入
/// `DOTNET_ReadyToRun=0`（原二进制改名 `*.real` 保留）。
///
/// 原因：resolver 是 ReadyToRun 镜像，运行时会执行 R2R 原生代码而忽略 IL 补丁；
/// 而 .NET 宿主又不接受被清掉 `ManagedNativeHeader` 的镜像（直接报
/// `FileNotFoundException: Could not load file or assembly ...`）。
/// 用环境变量禁用 R2R 是唯一不改原生代码、且跨架构（arm64 / x86_64）通用的办法。
///
/// 返回是否新安装了 shim（已安装返回 false）。
#[cfg(target_os = "macos")]
pub fn ensure_r2r_env_shim(client_dir: &Path) -> Result<bool, String> {
    let shim = client_dir.join("Unity.Licensing.Client");
    let real = client_dir.join("Unity.Licensing.Client.real");
    if real.exists() {
        return Ok(false);
    }
    let data = fs::read(&shim).map_err(|e| format!("licensing client not found: {}", e))?;
    // Mach-O magic（64 位小端 / universal）——已经是脚本说明装过了
    let is_macho = data.starts_with(&[0xCF, 0xFA, 0xED, 0xFE]) || data.starts_with(&[0xCA, 0xFE, 0xBA, 0xBE]);
    if !is_macho {
        return Ok(false);
    }
    fs::rename(&shim, &real).map_err(|e| format!("Failed to rename licensing client: {}", e))?;
    let script = "#!/bin/sh\n# patched by unifree: disable ReadyToRun so the IL patch in\n# Unity.Licensing.EntitlementResolver.dll takes effect\nexport DOTNET_ReadyToRun=0\nexec \"$(dirname \"$0\")/Unity.Licensing.Client.real\" \"$@\"\n";
    fs::write(&shim, script).map_err(|e| format!("Failed to write launcher shim: {}", e))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&shim, fs::Permissions::from_mode(0o755))
            .map_err(|e| format!("Failed to chmod launcher shim: {}", e))?;
    }
    Ok(true)
}

/// 撤销 launcher shim（还原原始 licensing client 可执行文件）。
#[cfg(target_os = "macos")]
pub fn remove_r2r_env_shim(client_dir: &Path) {
    let shim = client_dir.join("Unity.Licensing.Client");
    let real = client_dir.join("Unity.Licensing.Client.real");
    if real.exists() {
        let _ = fs::remove_file(&shim);
        let _ = fs::rename(&real, &shim);
    }
}

/// macOS：补丁编辑器内置 licensing client 的 DLL 后，重签它所在的
/// `UnityLicensingClient.app`（资源封印失效会让签名校验失败）。
#[cfg(target_os = "macos")]
fn resign_editor_licensing_client(target: &Path) {
    let mut current = target.parent();
    while let Some(p) = current {
        if p.extension().map(|e| e == "app").unwrap_or(false) {
            if let Err(e) = resign_app_bundle(p, "UnityLicensingClient.app") {
                eprintln!("⚠ macOS: failed to re-sign {}: {}", p.display(), e);
            }
            return;
        }
        current = p.parent();
    }
}

/// 补丁 Hub 自带 licensing client 的 `Unity.Licensing.EntitlementResolver.dll`：
/// 优先"就地 IL 字节补丁"（任意 LocalIPC 线通用，保留程序集身份；1.17.x / 1.18+ /
/// 未来新线都能命中，只要 ValidateSignature 的 IL 结构未改型）；命不中时按 LocalIPC
/// 发行线回退到预编译补丁 DLL（1.17.x = Hub 3.20.0 时代 / 1.18+ = Hub 3.21.1 起）。
/// 备份为 `<dll>.bak`；已补丁（内容一致）时跳过。需要管理员权限（Program Files）。
fn patch_hub_licensing_client() -> Result<String, String> {
    let licensing_dir = crate::scanner::hub_licensing_dir();
    let resolver = licensing_dir.join("Unity.Licensing.EntitlementResolver.dll");
    if !resolver.exists() {
        return Err(format!("not found: {}", resolver.display()));
    }
    let ver = licensing_client_version(&licensing_dir).unwrap_or_else(|| "unknown".into());

    // 就地 IL 补丁（首选，与 LocalIPC 线无关）
    let data = fs::read(&resolver).map_err(|e| format!("Failed to read hub resolver: {}", e))?;
    #[cfg(target_os = "macos")]
    let patch_result = crate::il_patch::patch_resolver_keep_r2r(&data);
    #[cfg(not(target_os = "macos"))]
    let patch_result = crate::il_patch::patch_resolver(&data);
    match patch_result {
        Ok((patched, report)) => {
            // 只结束 Hub 目录下的 licensing client（另一个进程会随后重新拉起并加载补丁后的 DLL）；
            // 不碰编辑器自己的 client（1.18+ 编辑器走独立目录/独立进程/版本化管道，互不影响）
            kill_hub_licensing_clients(&licensing_dir);
            if report.patched_sites > 0 {
                let bak = backup_path(&licensing_dir.join("Unity.Licensing.EntitlementResolver.dll"));
                if !bak.exists() {
                    fs::copy(&resolver, &bak)
                        .map_err(|e| format!("Failed to backup hub resolver: {}", e))?;
                }
                fs::write(&resolver, &patched)
                    .map_err(|e| format!("Failed to write patched hub resolver: {}", e))?;
                #[cfg(target_os = "macos")]
                let _ = ensure_r2r_env_shim(&licensing_dir);
                Ok(format!(
                    "hub licensing client resolver patched in-place (LocalIPC v{}, {} site(s)); editors launched from Hub now bypass ULF signature check",
                    ver, report.patched_sites
                ))
            } else {
                #[cfg(target_os = "macos")]
                let _ = ensure_r2r_env_shim(&licensing_dir);
                Ok(format!(
                    "hub licensing client resolver already patched in-place (LocalIPC v{}, {} site(s) detected)",
                    ver, report.already_patched_sites
                ))
            }
        }
        Err(e) => {
            eprintln!(
                "⚠ Hub resolver in-place IL patch not applicable ({}); falling back by LocalIPC line",
                e
            );
            // 回退按发行线选择预编译补丁 DLL：
            // - 1.17.x（Hub 3.20.0 时代）：EntitlementResolver 程序集版本 1.17.4.0（v7 运行时），
            //   必须用基于 1.17.4 原版生成的补丁 DLL（保留程序集版本），否则 client 启动即崩。
            // - 1.18+（Hub 3.21.1 起，如 6000.3.20f1/22f1/23f1 同线）：程序集版本 0.0.0.0
            //   （v8 运行时），用与编辑器 1.18 线相同的补丁 DLL 即可。
            let (line, patched): (&str, &[u8]) = if ver.starts_with("1.17") {
                (
                    "LocalIPC 1.17.x",
                    include_bytes!("../resources/win/Unity.Licensing.EntitlementResolver.hub.1.17.4.dll"),
                )
            } else if ver.starts_with("1.18") {
                (
                    "LocalIPC 1.18+",
                    include_bytes!("../resources/win/Unity.Licensing.EntitlementResolver.dll"),
                )
            } else {
                return Err(format!(
                    "licensing client 版本 {} 非 1.17.x/1.18+ 且就地 IL 补丁未命中（{}），暂未适配",
                    ver, e
                ));
            };
            kill_hub_licensing_clients(&licensing_dir);

            // 幂等：已是对应线的补丁版本则直接返回
            if fs::read(&resolver).map(|d| d.as_slice() == patched).unwrap_or(false) {
                return Ok(format!("hub licensing client resolver already patched ({})", line));
            }

            // 备份 + 替换
            let bak = backup_path(&licensing_dir.join("Unity.Licensing.EntitlementResolver.dll"));
            if !bak.exists() {
                fs::copy(&resolver, &bak)
                    .map_err(|e| format!("Failed to backup hub resolver: {}", e))?;
            }
            fs::write(&resolver, patched)
                .map_err(|e| format!("Failed to write patched hub resolver: {}", e))?;
            Ok(format!(
                "hub licensing client resolver patched ({} v{}); editors launched from Hub now bypass ULF signature check",
                line, ver
            ))
        }
    }
}

/// 根据 app.asar 路径推导 Hub 可执行文件路径（fuse 存在 exe 里）
/// fuse 所在的可执行文件：
/// - Windows：`<Hub>/Unity Hub.exe`
/// - macOS：Electron fuses 嵌在 `Electron Framework` 里，不在 `Contents/MacOS/Unity Hub`
fn hub_exe_path(asar_path: &Path) -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        let hub_dir = asar_path.parent()?.parent()?;
        Some(hub_dir.join("Unity Hub.exe"))
    }
    #[cfg(target_os = "macos")]
    {
        let _ = asar_path;
        Some(
            crate::scanner::hub_install_dir()
                .join("Contents")
                .join("Frameworks")
                .join("Electron Framework.framework")
                .join("Electron Framework"),
        )
    }
    #[cfg(target_os = "linux")]
    {
        let hub_dir = asar_path.parent()?.parent()?;
        Some(hub_dir.join("unityhub"))
    }
}

const HUB_FUSE_MAGIC: &[u8] = b"dL7pKGdnNz796PbbjQWNKmHXBZaB9tsX";
const HUB_FUSE4: usize = 4; // EnableEmbeddedAsarIntegrityValidation

/// 在已读入的 exe 字节里定位 fuse 4 的偏移
fn hub_fuse4_offset(data: &[u8]) -> Result<usize, String> {
    let pos = find_bytes(data, HUB_FUSE_MAGIC)
        .ok_or("Hub exe 中未找到 Electron fuses magic（可能不是 3.20.0+）")?;
    let wire_start = pos + HUB_FUSE_MAGIC.len();
    if data.len() < wire_start + 2 + HUB_FUSE4 {
        return Err("fuse 段过短".into());
    }
    let version = data[wire_start];
    let length = data[wire_start + 1] as usize;
    if version != 1 {
        return Err(format!("未知的 fuse 版本: {}", version));
    }
    if HUB_FUSE4 >= length {
        return Err("fuse 段长度不足（无 fuse4）".into());
    }
    let off = wire_start + 2 + HUB_FUSE4;
    let cur = data[off];
    if cur != b'1' && cur != b'0' {
        return Err(format!("fuse4 字节异常: 0x{:02X}", cur));
    }
    Ok(off)
}

/// 读取 Hub exe 里 fuse 4（`EnableEmbeddedAsarIntegrityValidation`）的当前字节（'0'/'1'）。
/// 与补丁写入解耦，供状态检测复用。
fn hub_exe_fuse4(exe_path: &Path) -> Result<u8, String> {
    let data = fs::read(exe_path).map_err(|e| format!("Failed to read Hub exe: {}", e))?;
    let off = hub_fuse4_offset(&data)?;
    Ok(data[off])
}

/// fuse 4 是否已被翻转为 '0'（即 asar 完整性校验已关闭）
fn hub_exe_fuse_disabled(exe_path: &Path) -> bool {
    matches!(hub_exe_fuse4(exe_path), Ok(b'0'))
}

/// Hub 3.20.0+（Electron 43）的 exe 带 Electron fuses。把
/// `EnableEmbeddedAsarIntegrityValidation`（fuse 4）从 '1' 翻转为 '0'，
/// 使 Electron 不再校验 app.asar 头 JSON 的 SHA256（否则补丁后启动即 FATAL）。
/// 返回是否发生了修改。首次修改前会备份 exe 为 `<exe>.bak`。
fn flip_hub_exe_fuses(exe_path: &Path) -> Result<bool, String> {
    let mut data = fs::read(exe_path).map_err(|e| format!("Failed to read Hub exe: {}", e))?;
    let fuse4_off = hub_fuse4_offset(&data)?;
    if data[fuse4_off] == b'0' {
        return Ok(false); // 已关闭
    }

    // 翻转前备份 exe（旧版本残留的 `<exe>.bak` 会被换成本次安装的原版，避免恢复时写回旧 exe）
    let bak = backup_path(exe_path);
    let bak_path = Path::new(&bak);
    let stale_bak = bak_path.exists() && !same_installed_version(exe_path, bak_path);
    if !bak_path.exists() || stale_bak {
        fs::copy(exe_path, bak_path).map_err(|e| format!("Failed to backup Hub exe: {}", e))?;
    }
    data[fuse4_off] = b'0';
    fs::write(exe_path, &data).map_err(|e| format!("Failed to write Hub exe: {}", e))?;
    Ok(true)
}

/// 读取 asar 内 `package.json` 的 Hub 版本（原版 / 补丁重建版都保留该文件）
fn hub_version_of_asar(asar_path: &Path) -> Option<String> {
    let data = fs::read(asar_path).ok()?;
    let reader = asar::AsarReader::new(&data, None).ok()?;
    for (path, file) in reader.files() {
        if path.to_string_lossy() == "package.json" {
            let json: serde_json::Value = serde_json::from_slice(file.data()).ok()?;
            return json
                .get("version")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
        }
    }
    None
}

/// 两个文件是否属于同一版本（用 ProductVersion 的首段比对，"3.22.5" 与 "3.22.5.0" 视为一致）。
/// 读不出任一侧版本时按"一致"处理，避免误判导致反复重写备份。
fn same_installed_version(a: &Path, b: &Path) -> bool {
    let va = crate::scanner::read_file_product_version(a);
    let vb = crate::scanner::read_file_product_version(b);
    match (va, vb) {
        (Some(x), Some(y)) => x.starts_with(&y) || y.starts_with(&x),
        _ => true,
    }
}

/// `app.asar.bak` 是否是"旧版本 Hub"的遗留备份。
///
/// 判定依据：备份内 `package.json` 的版本 vs 当前安装 Hub 的版本
/// （优先取 exe 的 ProductVersion，读不到时退化为当前 asar 内的版本）。
/// 任一版本读不出时按"非陈旧"处理，避免误删可用的干净基线。
fn is_stale_hub_backup(bak: &Path, asar: &Path) -> bool {
    let Some(bak_version) = hub_version_of_asar(bak) else {
        return false;
    };
    let installed = hub_exe_path(asar)
        .and_then(|exe| crate::scanner::read_file_product_version(&exe))
        .or_else(|| hub_version_of_asar(asar));
    let Some(installed) = installed else {
        return false;
    };
    // "3.22.5" 与 "3.22.5.0" 视为同一版本
    !(installed.starts_with(&bak_version) || bak_version.starts_with(&installed))
}

/// 在字节切片中查找子序列，返回偏移
fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

/// 结束可执行文件位于指定 licensing client 目录下的 Unity.Licensing.Client 进程
/// （仅限 Windows；其他平台无此必要）。只按 Hub 目录过滤，避免误杀
/// 1.18+ 编辑器自己目录下的 licensing client 进程。
fn kill_hub_licensing_clients(licensing_dir: &Path) {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        let prefix = licensing_dir.to_string_lossy().replace('\'', "''");
        let script = format!(
            "Get-CimInstance Win32_Process -Filter \"Name='Unity.Licensing.Client.exe'\" | \
             Where-Object {{ $_.ExecutablePath -like '{0}*' }} | \
             ForEach-Object {{ Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue }}",
            prefix
        );
        let _ = std::process::Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-Command", &script])
            .creation_flags(0x08000000)
            .output();
    }
    #[cfg(target_os = "macos")]
    {
        // 只结束 Hub 自带 client（路径过滤），保留编辑器自己的 client 进程
        let _ = std::process::Command::new("pkill")
            .arg("-f")
            .arg(licensing_dir.join("Unity.Licensing.Client").to_string_lossy().as_ref())
            .output();
    }
    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    {
        let _ = licensing_dir;
        kill_process("Unity.Licensing.Client").ok();
    }
}

/// 读取 app.asar，完整重建：保留 app.asar.unpacked/ 里 native 模块的 unpacked 标记，
/// 其余文件重写进新数据区（允许任意改 JS 内容，不受等长限制）。
/// 补丁目标：
/// - `licenseQueryService.getLicense()` → 返回假的 Unity Pro ULF 许可证（让 Hub UI 显示许可证）
/// - `licenseService` / `licenseQueryService` 的 `isLicenseValid()` → return true
/// - `DefaultLocalConfig` 的 DisableSignIn / DisableAutoUpdate
/// 不替换任何 DLL。返回补丁的文件数。
///
/// 注：Hub 3.20.0 的 exe 带 EnableEmbeddedAsarIntegrityValidation fuse（校验 header JSON 的
/// SHA256），补丁前需先把该 fuse 翻转为 0（见 flip_hub_exe_fuses），否则改动 asar 会触发
/// "Integrity check failed" FATAL。
fn rewrite_hub_asar(asar_path: &Path, disable_signin: bool, disable_update: bool) -> Result<usize, String> {
    let data = fs::read(asar_path).map_err(|e| format!("Failed to read app.asar: {}", e))?;

    // 收集每个文件的元信息（大小、executable、是否 unpacked）
    let (header, _) = asar::Header::read(&mut &data[..]).map_err(|e| format!("Failed to read asar header: {}", e))?;
    let mut metas: std::collections::HashMap<PathBuf, AsarFileMeta> = std::collections::HashMap::new();
    collect_asar_meta(&header, Path::new(""), &mut metas);

    // 读取全部文件内容（unpacked 文件内容不需要——只保留标记，Electron 从 .unpacked/ 加载）
    let asar = asar::AsarReader::new(&data, None).map_err(|e| format!("Failed to parse asar: {}", e))?;

    // 重建数据区 + 收集新 header 条目
    let mut new_data: Vec<u8> = Vec::new();
    let mut entries: Vec<(PathBuf, serde_json::Value)> = Vec::new();
    let mut patched = 0usize;

    for (path, file) in asar.files() {
        let meta = metas.get(path).cloned().unwrap_or(AsarFileMeta {
            size: file.data().len(),
            executable: false,
            unpacked: false,
        });

        if meta.unpacked {
            // native 模块：保留 unpacked 标记，不写入数据区
            entries.push((path.to_path_buf(), serde_json::json!({ "size": meta.size, "unpacked": true })));
            continue;
        }

        let mut content = file.data().to_vec();
        let path_str = path.to_string_lossy();
        let mut changed = false;
        if path_str.ends_with(".js") {
            let new_text = patch_license_js(&content, &path, disable_signin, disable_update);
            if new_text.as_bytes() != content {
                content = new_text.into_bytes();
                changed = true;
            }
        }

        let offset = new_data.len();
        new_data.extend_from_slice(&content);
        let entry = if meta.executable {
            serde_json::json!({ "size": content.len(), "offset": offset.to_string(), "executable": true })
        } else {
            serde_json::json!({ "size": content.len(), "offset": offset.to_string() })
        };
        entries.push((path.to_path_buf(), entry));

        if changed {
            eprintln!("  ✓ Patched {}", path.display());
            patched += 1;
        }
    }

    // 构建嵌套 header JSON：{"files": {...}}
    let mut root_files = serde_json::Map::new();
    for (path, entry) in entries {
        let comps: Vec<String> = path
            .components()
            .map(|c| c.as_os_str().to_string_lossy().into_owned())
            .collect();
        insert_into_header(&mut root_files, &comps, entry);
    }
    let mut root = serde_json::Map::new();
    root.insert("files".to_string(), serde_json::Value::Object(root_files));
    let json_bytes = serde_json::to_vec(&serde_json::Value::Object(root)).map_err(|e| e.to_string())?;

    // 组装 asar（pickle 头 + JSON(4 对齐) + 数据区）
    let json_size = json_bytes.len();
    let aligned = json_size + (4 - (json_size % 4)) % 4;
    let mut json_padded = json_bytes;
    json_padded.resize(aligned, 0);

    let mut out: Vec<u8> = Vec::with_capacity(16 + aligned + new_data.len());
    out.extend_from_slice(&4u32.to_le_bytes());
    out.extend_from_slice(&((aligned + 8) as u32).to_le_bytes());
    out.extend_from_slice(&((aligned + 4) as u32).to_le_bytes());
    out.extend_from_slice(&(json_size as u32).to_le_bytes());
    out.extend_from_slice(&json_padded);
    out.extend_from_slice(&new_data);

    fs::write(asar_path, &out).map_err(|e| format!("Failed to write app.asar: {}", e))?;
    Ok(patched)
}

/// asar 文件元信息
#[derive(Clone)]
struct AsarFileMeta {
    size: usize,
    executable: bool,
    unpacked: bool,
}

/// 遍历 asar header，收集每个文件的元信息
fn collect_asar_meta(
    header: &asar::Header,
    prefix: &Path,
    out: &mut std::collections::HashMap<PathBuf, AsarFileMeta>,
) {
    match header {
        asar::Header::File(f) => {
            out.insert(prefix.to_path_buf(), AsarFileMeta {
                size: f.size(),
                executable: f.executable(),
                unpacked: f.unpacked(),
            });
        }
        asar::Header::Directory { files } => {
            for (name, child) in files {
                collect_asar_meta(child, &prefix.join(name), out);
            }
        }
        asar::Header::Link { .. } => {}
    }
}

/// 把文件条目插入嵌套的 header files 结构（自动建目录）
fn insert_into_header(
    files_map: &mut serde_json::Map<String, serde_json::Value>,
    comps: &[String],
    entry: serde_json::Value,
) {
    if comps.len() == 1 {
        files_map.insert(comps[0].clone(), entry);
        return;
    }
    let child = files_map
        .entry(comps[0].clone())
        .or_insert_with(|| serde_json::json!({ "files": {} }));
    if let Some(obj) = child.as_object_mut() {
        if let Some(cf) = obj.get_mut("files").and_then(|v| v.as_object_mut()) {
            insert_into_header(cf, &comps[1..], entry);
        }
    }
}

/// 对单个 JS 文件应用补丁。返回补丁后的文本（重建模式下允许变长）。
fn patch_license_js(content: &[u8], path: &Path, disable_signin: bool, disable_update: bool) -> String {
    let text = String::from_utf8_lossy(content);
    let mut modified = text.to_string();
    let file_name = path.file_name().unwrap_or_default().to_string_lossy();

    // licenseQueryService：getLicense → 返回假的 Unity Pro ULF 许可证（让 Hub UI 显示许可证）
    // + isLicenseValid → true
    if file_name.starts_with("licenseQueryService") {
        if let Some(patched) = replace_method_body(
            &modified,
            "async getLicense()",
            "async getLicense() {\n\t\t\t\treturn [{startDate:'2020-01-01T00:00:00Z',stopDate:'2099-12-31T23:59:59Z',updateDate:'2020-01-01T00:00:00Z',valid:true,activated:true,canExpire:false,error:false,returned:false,maintenance:false,initialized:true,mustReactivate:false,beta:false,isLicenseActionInProgress:false,label:'Unity Pro',flow:'Unity Pro',entitlementGroupId:'F4-I0SA-6VDD-WI39-41QP-XXXX',isEGL:false,transactionId:'',licenseType:'ULF',subLabel:'',entitlementGroupType:'organization'}];\n\t\t\t}",
        ) {
            modified = patched;
        }
        if let Some(patched) = replace_method_body(&modified, "async isLicenseValid()", "async isLicenseValid() {\n\t\t\t\treturn true; // patched by unifree\n\t\t\t}") {
            modified = patched;
        }
        // Hub 3.22.0+（HUB-7120）：授权校验从布尔值改成三态 verdict
        // （ENTITLED / NOT_ENTITLED / UNVERIFIED）。UI 的阻塞点和项目打开门禁
        // 走 checkEditorEntitlement / checkUlfLicense，不再走 isLicenseValid；
        // 这两个方法直连 licensing client 的 checkEntitlements/getAllEntitlementGroups，
        // 不打补丁的话 Hub 会按客户端真实答复显示"没有许可证"。
        if let Some(patched) = replace_method_body(
            &modified,
            "async checkEditorEntitlement()",
            "async checkEditorEntitlement() {\n\t\t\t\treturn \"ENTITLED\"; // patched by unifree\n\t\t\t}",
        ) {
            modified = patched;
        }
        if let Some(patched) = replace_method_body(
            &modified,
            "async checkUlfLicense()",
            "async checkUlfLicense() {\n\t\t\t\treturn \"ENTITLED\"; // patched by unifree\n\t\t\t}",
        ) {
            modified = patched;
        }
    }

    // licenseService.isLicenseValid → 始终返回 true
    if file_name.starts_with("licenseService") && modified.contains("isLicenseValid") {
        let search = "isLicenseValid() {\n\t\tif (await this.#licenseQueryService.isLicenseValid()) return true;";
        if modified.contains(search) {
            modified = modified.replace(search, "isLicenseValid() {\n\t\treturn true; // patched by unifree");
        } else if let Some(patched) = replace_method_body(&modified, "async isLicenseValid()", "async isLicenseValid() {\n\t\t\t\treturn true; // patched by unifree\n\t\t\t}") {
            modified = patched;
        }
    }

    // DefaultLocalConfig 默认值（强制登录 / 自动更新）
    //
    // 不按文件名过滤：Hub 3.22 起除了 `DefaultLocalConfig-*.js`，`installerContainerSetup-*.js`
    // 里还有一份独立的 DefaultLocalConfig 副本（编辑器安装/下载容器的窗口用它决定要不要登录）。
    // 只改 `DefaultLocalConfig-*.js` 会在"从 Hub 装编辑器"时重新要求登录。
    if disable_signin {
        modified = modified.replace("DisableSignInRequired]: false,", "DisableSignInRequired]: true,");
    }
    if disable_update {
        modified = modified.replace("DisableAutoUpdate]: false,", "DisableAutoUpdate]: true,");
    }

    modified
}

/// Restore Hub from backup
pub fn restore_hub() -> Result<String, String> {
    let asar_path = hub_asar_path();
    let resources_path = asar_path.parent().ok_or("Cannot get resources path")?;
    let app_folder = resources_path.join("app");
    let asar_bak = backup_path(&asar_path);

    // 删除提取的 app 目录
    if app_folder.exists() {
        fs::remove_dir_all(&app_folder).map_err(|e| format!("Failed to remove app folder: {}", e))?;
    }

    // 恢复原始 asar
    if asar_bak.exists() {
        if asar_path.exists() {
            fs::remove_file(&asar_path).map_err(|e| format!("Failed to remove patched asar: {}", e))?;
        }
        fs::rename(&asar_bak, &asar_path).map_err(|e| format!("Failed to restore asar: {}", e))?;
    }

    // 恢复 Hub XML DLL
    let licensing_dir = crate::scanner::hub_licensing_dir();
    let xml_dll_path = licensing_dir.join("System.Security.Cryptography.Xml.dll");
    let xml_dll_bak = backup_path(&licensing_dir.join("System.Security.Cryptography.Xml.dll"));

    if xml_dll_bak.exists() {
        if xml_dll_path.exists() {
            fs::remove_file(&xml_dll_path).map_err(|e| format!("Failed to remove patched XML DLL: {}", e))?;
        }
        fs::rename(&xml_dll_bak, &xml_dll_path).map_err(|e| format!("Failed to restore XML DLL: {}", e))?;
        eprintln!("✓ Restored original XML DLL");
    }

    // 恢复 Hub licensing client 的 EntitlementResolver（若被补丁过）
    let hub_resolver = licensing_dir.join("Unity.Licensing.EntitlementResolver.dll");
    let hub_resolver_bak = backup_path(&hub_resolver);
    if hub_resolver_bak.exists() {
        if hub_resolver.exists() {
            fs::remove_file(&hub_resolver).map_err(|e| format!("Failed to remove patched hub resolver: {}", e))?;
        }
        fs::rename(&hub_resolver_bak, &hub_resolver)
            .map_err(|e| format!("Failed to restore hub resolver: {}", e))?;
        eprintln!("✓ Restored hub licensing client resolver");
    }

    // 恢复 Licensing Client（如果被禁用）
    let licensing_client_exe = licensing_dir.join("Unity.Licensing.Client.exe");
    let licensing_client_bak = backup_path(&licensing_client_exe);

    if !licensing_client_exe.exists() && licensing_client_bak.exists() {
        fs::rename(&licensing_client_bak, &licensing_client_exe).map_err(|e| format!("Failed to restore Licensing Client: {}", e))?;
        eprintln!("✓ Restored Licensing Client");
    }

    // 恢复 Hub exe（撤销 fuse 翻转）
    if let Some(hub_exe) = hub_exe_path(&asar_path) {
        let exe_bak = backup_path(&hub_exe);
        if exe_bak.exists() {
            // macOS 上 hub_exe 是 framework 里的符号链接：必须"写穿"到目标文件，
            // 先删后改名会把符号链接换成真实文件，破坏 framework 结构（codesign 报
            // "bundle format is ambiguous"）。copy 对两种结构都安全。
            fs::copy(&exe_bak, &hub_exe).map_err(|e| format!("Failed to restore exe: {}", e))?;
            fs::remove_file(&exe_bak).map_err(|e| format!("Failed to remove backup exe: {}", e))?;
            eprintln!("✓ Restored Hub exe");
        }
    }

    // macOS 上 licensing client 的可执行文件没有 .exe 后缀
    #[cfg(target_os = "macos")]
    {
        remove_r2r_env_shim(&licensing_dir);
        let client = licensing_dir.join("Unity.Licensing.Client");
        let client_bak = backup_path(&licensing_dir.join("Unity.Licensing.Client"));
        if !client.exists() && client_bak.exists() {
            fs::rename(&client_bak, &client).map_err(|e| format!("Failed to restore Licensing Client: {}", e))?;
            eprintln!("✓ Restored Licensing Client");
        }
        // 恢复原文件后签名仍是补丁期间的 ad-hoc 签名，重新签一次保证可启动
        resign_app_bundle(&crate::scanner::hub_install_dir(), "Unity Hub.app")?;
        eprintln!("✓ Re-signed Unity Hub.app (ad-hoc)");
    }

    Ok("Restored: app.asar, XML DLL, Licensing Client, Hub exe".into())
}

/// Check if a process is running by name
pub fn check_process_running(name: &str) -> bool {
    #[cfg(target_os = "windows")]
    {
        use windows::Win32::Foundation::CloseHandle;
        use windows::Win32::System::Diagnostics::ToolHelp::{
            CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
        };

        unsafe {
            let snapshot = match CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) {
                Ok(h) => h,
                Err(_) => return false,
            };
            let mut entry = PROCESSENTRY32W {
                dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
                ..Default::default()
            };
            let mut found = false;
            if Process32FirstW(snapshot, &mut entry).is_ok() {
                loop {
                    let end = entry.szExeFile.iter().position(|&c| c == 0).unwrap_or(entry.szExeFile.len());
                    let exe_name = String::from_utf16_lossy(&entry.szExeFile[..end]);
                    if exe_name.eq_ignore_ascii_case(name) {
                        found = true;
                        break;
                    }
                    if Process32NextW(snapshot, &mut entry).is_err() {
                        break;
                    }
                }
            }
            let _ = CloseHandle(snapshot);
            found
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        let output = std::process::Command::new("pgrep")
            .arg(name)
            .output();
        match output {
            Ok(o) => o.status.success(),
            Err(_) => false,
        }
    }
}

/// Kill a process by name
pub fn kill_process(name: &str) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        std::process::Command::new("taskkill")
            .args(["/F", "/IM", name])
            .creation_flags(0x08000000)
            .output()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(not(target_os = "windows"))]
    {
        std::process::Command::new("pkill")
            .arg(name)
            .output()
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 对 6000.7.0a3 原始二进制的副本应用单点补丁
    /// （ValidateSignature 包装函数 → mov eax,1; ret），
    /// 验证恰好命中 1 个锚点、1 个写入点，且偏移与文档一致。
    #[test]
    fn simplified_patch_applies_to_real_binary() {
        let src = r"C:/Program Files/Unity/Hub/Editor/6000.7.0a3/Editor/Data/Resources/Licensing/Client/Unity.Licensing.Client.exe.bak";
        if !Path::new(src).exists() {
            eprintln!("SKIP: {} not found", src);
            return;
        }

        let tmp = std::env::temp_dir().join(format!("unifree_patch_test_{}", std::process::id()));
        std::fs::create_dir_all(&tmp).unwrap();
        let exe = tmp.join("Unity.Licensing.Client.exe");
        std::fs::copy(src, &exe).unwrap();

        let result = patch_licensing_client(exe.to_string_lossy().as_ref()).unwrap();
        assert_eq!(result, "Patched 1 areas", "应恰好应用 1 个补丁");

        // 对比补丁后的 exe 与其自身 .bak，收集所有差异区间
        let patched = std::fs::read(&exe).unwrap();
        let original = std::fs::read(format!("{}.bak", exe.to_string_lossy())).unwrap();
        assert_eq!(patched.len(), original.len(), "补丁不应改变文件大小");

        let mut regions: Vec<(u64, u64)> = Vec::new();
        let mut i = 0usize;
        while i < patched.len() {
            if patched[i] != original[i] {
                let start = i as u64;
                let mut end = i;
                while end < patched.len() && patched[end] != original[end] {
                    end += 1;
                }
                regions.push((start, (end - i) as u64));
                i = end;
            } else {
                i += 1;
            }
        }

        // 期望恰好 1 个差异区：
        //   ValidateSignature 包装函数 @ 0x4F1010 (6B: B8 01 00 00 00 C3 = mov eax,1; ret)
        assert_eq!(regions, vec![(0x4F1010, 6)],
            "差异区不符，实际: {:?}", regions);

        // 校验补丁字节：mov eax,1; ret
        let p1 = &patched[0x4F1010..0x4F1010 + 6];
        assert_eq!(p1, &[0xB8, 0x01, 0x00, 0x00, 0x00, 0xC3], "包装函数补丁字节不符");

        // 清理临时目录
        std::fs::remove_dir_all(&tmp).ok();
    }

    /// 对 Hub 3.20.0 的真实 app.asar 副本应用 JS-only 就地重写（rewrite_hub_asar），
    /// 验证：asar 仍可解析、unpacked native 模块被排除（保留在 .unpacked/）、
    /// licenseService/licenseQueryService 的 isLicenseValid 被打成 return true。
    #[test]
    fn hub_asar_rewrite_applies_to_real_binary() {
        // 优先用原始备份（已补丁的机器上 app.asar 已被修改，.bak 才是干净副本）
        let src_bak = r"D:/Unity/Hub/Unity Hub/resources/app.asar.bak";
        let src = if Path::new(src_bak).exists() { src_bak } else { r"D:/Unity/Hub/Unity Hub/resources/app.asar" };
        if !Path::new(src).exists() {
            eprintln!("SKIP: {} not found", src);
            return;
        }

        let tmp = std::env::temp_dir().join(format!("unifree_hub_test_{}", std::process::id()));
        std::fs::create_dir_all(&tmp).unwrap();
        let asar_path = tmp.join("app.asar");
        std::fs::copy(src, &asar_path).unwrap();

        // 统计原 asar 的文件数与 unpacked 数
        let orig_data = std::fs::read(&asar_path).unwrap();
        let orig_reader = asar::AsarReader::new(&orig_data, None).unwrap();
        let orig_count = orig_reader.files().len();
        let (header, _) = asar::Header::read(&mut &orig_data[..]).unwrap();
        let orig_metas = collect_asar_meta_map(&header, Path::new(""));
        let unpacked_count = orig_metas.values().filter(|m| m.unpacked).count();
        assert!(unpacked_count > 0, "应存在 unpacked native 模块");

        // 执行完整重建（在临时副本上）
        let patched_count = rewrite_hub_asar(&asar_path, false, false).unwrap();
        assert!(patched_count >= 2, "应补丁 licenseService + licenseQueryService，实际 {}", patched_count);

        // 重读补丁后的 asar，确认格式仍有效、文件数不变
        let patched_data = std::fs::read(&asar_path).unwrap();
        let reader = asar::AsarReader::new(&patched_data, None).unwrap();
        assert_eq!(reader.files().len(), orig_count,
            "重建不应改变文件数");

        // unpacked 标记必须原样保留（Electron 依赖它重定向到 app.asar.unpacked/）
        let (patched_header, _) = asar::Header::read(&mut &patched_data[..]).unwrap();
        let patched_metas = collect_asar_meta_map(&patched_header, Path::new(""));
        let patched_unpacked: std::collections::HashSet<PathBuf> = patched_metas
            .iter()
            .filter(|(_, m)| m.unpacked)
            .map(|(p, _)| p.clone())
            .collect();
        let orig_unpacked: std::collections::HashSet<PathBuf> = orig_metas
            .iter()
            .filter(|(_, m)| m.unpacked)
            .map(|(p, _)| p.clone())
            .collect();
        assert_eq!(patched_unpacked, orig_unpacked,
            "unpacked 标记应原样保留");

        // 校验 JS 补丁已生效（isLicenseValid + getLicense 假许可证）
        let mut patched_js = 0;
        let mut fake_license = false;
        for (path, file) in reader.files() {
            let path_str = path.to_string_lossy();
            if path_str.ends_with(".js") {
                let content = String::from_utf8_lossy(file.data());
                if content.contains("// patched by unifree") && content.contains("return true;") {
                    patched_js += 1;
                }
                if content.contains("label:'Unity Pro'") && content.contains("licenseType:'ULF'") {
                    fake_license = true;
                }
            }
        }
        assert!(patched_js >= 2, "应至少 2 个 JS 含补丁标记，实际 {}", patched_js);
        assert!(fake_license, "licenseQueryService.getLicense 应返回假 Pro 许可证");

        std::fs::remove_dir_all(&tmp).ok();
    }

    /// 对 Unity 2019.4.40 原始 Unity.exe 的副本应用 ValidateServerProcess 补丁，
    /// 验证恰好命中 1 个锚点、1 个写入点（3 字节），且补丁字节为 cmp edi,edi; nop。
    #[test]
    fn validate_server_process_patch_applies_to_real_binary() {
        let exe = r"D:/Unity/Editor/2019.4.40f1/Editor/Unity.exe";
        let src_bak = r"D:/Unity/Editor/2019.4.40f1/Editor/Unity.exe.bak";
        let src = if Path::new(src_bak).exists() { src_bak } else { exe };
        if !Path::new(src).exists() {
            eprintln!("SKIP: {} not found", src);
            return;
        }

        let tmp = std::env::temp_dir().join(format!("unifree_unityexe_test_{}", std::process::id()));
        std::fs::create_dir_all(&tmp).unwrap();
        let unity_exe = tmp.join("Unity.exe");
        std::fs::copy(src, &unity_exe).unwrap();

        let result = patch_unity_exe_validate_server_process(unity_exe.to_string_lossy().as_ref()).unwrap();
        assert_eq!(result, "Patched ValidateServerProcess + LICENSE SYSTEM dispatcher (2 areas)");

        // 对比补丁后的 exe 与其自身 .bak，收集所有差异区间
        let patched = std::fs::read(&unity_exe).unwrap();
        let original = std::fs::read(format!("{}.bak", unity_exe.to_string_lossy())).unwrap();
        assert_eq!(patched.len(), original.len(), "补丁不应改变文件大小");

        let mut regions: Vec<(u64, u64)> = Vec::new();
        let mut i = 0usize;
        while i < patched.len() {
            if patched[i] != original[i] {
                let start = i as u64;
                let mut end = i;
                while end < patched.len() && patched[end] != original[end] {
                    end += 1;
                }
                regions.push((start, (end - i) as u64));
                i = end;
            } else {
                i += 1;
            }
        }
        // P1: cmp edi,9（83 FF 09）→ cmp edi,edi + nop（3B FF 90）：中间字节 FF 不变，
        //     因此是两个相距 2 字节的单字节区。
        // P2: call [rax+0xD8]（FF 90 D8 00 00 00）→ xor eax,eax + 4×nop（31 C0 90 90 90 90）：
        //     6 字节全部改变，是一个 6 字节区。
        assert_eq!(regions.len(), 3, "应恰好 3 个差异区（P1 两个单字节 + P2 一个 6 字节），实际: {:?}", regions);
        let (off0, len0) = regions[0];
        let (off1, len1) = regions[1];
        let (off2, len2) = regions[2];
        assert_eq!((len0, len1, len2), (1, 1, 6), "差异区长度应为 1,1,6");
        assert_eq!(off1 - off0, 2, "P1 的两个差异字节应相距 2 字节（中间 FF 不变）");
        assert_eq!(original[off0 as usize], 0x83, "P1 第 1 字节原应为 0x83");
        assert_eq!(patched[off0 as usize], 0x3B, "P1 第 1 字节补丁应为 0x3B");
        assert_eq!(original[off1 as usize], 0x09, "P1 第 2 字节原应为 0x09");
        assert_eq!(patched[off1 as usize], 0x90, "P1 第 2 字节补丁应为 0x90");
        assert_eq!(&original[off2 as usize..off2 as usize + 6], &[0xFF, 0x90, 0xD8, 0x00, 0x00, 0x00], "P2 原应为 call [rax+0xD8]");
        assert_eq!(&patched[off2 as usize..off2 as usize + 6], &[0x31, 0xC0, 0x90, 0x90, 0x90, 0x90], "P2 补丁应为 xor eax,eax + 4×nop");

        std::fs::remove_dir_all(&tmp).ok();
    }

    /// 收集 asar 头里所有文件的元信息（测试辅助）
    fn collect_asar_meta_map(header: &asar::Header, prefix: &Path) -> std::collections::HashMap<PathBuf, AsarFileMeta> {
        let mut out = std::collections::HashMap::new();
        collect_asar_meta(header, prefix, &mut out);
        out
    }
}
