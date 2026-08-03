use std::fs;
use std::io::{Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

// === Hub paths ===
// 复用 scanner 中唯一的 hub_resources_path 实现，避免重复
use crate::scanner::hub_resources_path;

fn hub_asar_path() -> PathBuf {
    hub_resources_path().join("app.asar")
}

/// Check EntitlementResolver DLL status
pub fn get_editor_dll_status(dll_path: &str) -> String {
    let path = Path::new(dll_path);
    if !path.exists() {
        return "not_found".into();
    }

    // 检查是否有备份文件（表示已补丁）
    let bak_path = format!("{}.bak", dll_path);
    if Path::new(&bak_path).exists() {
        return "patched".into();
    }

    // 检查DLL大小来判断是否已补丁
    // 原始DLL约514KB，补丁后约341KB
    if let Ok(metadata) = fs::metadata(path) {
        let size = metadata.len();
        if size < 400_000 {
            return "patched_no_backup".into();
        }
    }

    "original".into()
}

/// Extract Unity version year from DLL path
/// e.g., "C:\...\2022.3.1f1\Editor\...\dll" -> "2022"
/// e.g., "C:\...\6000.0.0f1\Editor\...\dll" -> "6000"
fn extract_unity_version(dll_path: &str) -> Option<String> {
    let path = Path::new(dll_path);
    // Walk up parent directories to find version folder
    let mut current = path.parent();
    while let Some(dir) = current {
        let folder_name = dir.file_name()?.to_string_lossy();
        // Check if it starts with a version number
        if folder_name.starts_with("20") || folder_name.starts_with("6") {
            // Extract year prefix (e.g., "2022" from "2022.3.1f1" or "6000" from "6000.0.0f1")
            let prefix = if folder_name.starts_with("6000") {
                "6000".to_string()
            } else {
                folder_name.chars().take(4).collect::<String>()
            };
            return Some(prefix);
        }
        current = dir.parent();
    }
    None
}

/// Get patched DLL for specific Unity version
/// >= 6000: Unity.Licensing.EntitlementResolver.dll
/// < 6000: System.Security.Cryptography.Xml.dll
fn get_patched_dll_for_version(version: &str) -> Option<&'static [u8]> {
    match version {
        "2019" | "2020" | "2021" | "2022" => {
            Some(include_bytes!("../resources/win/System.Security.Cryptography.Xml.dll"))
        }
        "6000" => {
            Some(include_bytes!("../resources/win/Unity.Licensing.EntitlementResolver.dll"))
        }
        _ => None,
    }
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
    let bak = format!("{}.bak", exe_path);
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
    let p1 = find_pattern(&data,
        "57 56 53 48 83 EC 20 48 8B DA 48 85 DB 74 ?? 48 8B 71 10 40 0F B6 79 18 48 8D 15 ?? ?? ?? ?? 48 39 11 74 ?? 48 8D 15 ?? ?? ?? ?? 48 39 11 75 ?? 48 8B D3"
    ).ok_or("P1: ValidateSignature wrapper not found")?;
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
pub fn patch_entitlement_resolver(dll_path: &str) -> Result<String, String> {
    // 6000.7+: Native AOT exe，字节级 patch
    if dll_path.ends_with(".exe") {
        return patch_licensing_client(dll_path);
    }
    let path = Path::new(dll_path);
    if !path.exists() {
        return Err("DLL not found".into());
    }

    // Extract Unity version from path
    let version = extract_unity_version(dll_path)
        .ok_or("Cannot detect Unity version from path")?;

    // Get version-specific patched DLL
    let patched_dll = get_patched_dll_for_version(&version)
        .ok_or(format!("Unity {} is not supported. Supported versions: 2019, 2020, 2021, 2022, 6000", version))?;

    // 创建备份
    let bak_path = format!("{}.bak", dll_path);
    if !Path::new(&bak_path).exists() {
        fs::copy(path, &bak_path).map_err(|e| e.to_string())?;
    }

    // 写入版本对应的补丁DLL
    fs::write(path, patched_dll).map_err(|e| format!("Failed to write patched DLL: {}", e))?;
    Ok(format!("Patched: replaced with pre-patched DLL for Unity {}", version))
}

/// Restore DLL from backup
pub fn restore(dll_path: &str) -> Result<String, String> {
    let bak_path = format!("{}.bak", dll_path);
    let bak = Path::new(&bak_path);
    if !bak.exists() {
        return Err("Backup not found".into());
    }
    fs::copy(bak, dll_path).map_err(|e| e.to_string())?;
    fs::remove_file(bak).map_err(|e| e.to_string())?;
    Ok(format!("Restored: {}", dll_path))
}



/// 检查 Hub 是否已被补丁（XML DLL 备份 / asar 备份两者任一存在即视为 patched）
/// 返回 "patched" / "original" / "error"
fn hub_patch_state() -> String {
    let asar_path = hub_asar_path();
    let resources_path = match asar_path.parent() {
        Some(p) => p,
        None => return "error".into(),
    };

    let hub_dir = resources_path.parent().unwrap_or(resources_path);
    let xml_dll_bak = hub_dir
        .join("UnityLicensingClient_V1")
        .join("System.Security.Cryptography.Xml.dll.bak");
    if xml_dll_bak.exists() {
        return "patched".into();
    }

    // 兼容旧版本：检查 asar 备份
    let asar_bak = resources_path.join("app.asar.bak");
    if asar_bak.exists() {
        return "patched".into();
    }

    "original".into()
}

/// Check Hub status: "patched", "original", "error"
pub fn get_hub_status() -> String {
    hub_patch_state()
}

pub fn get_hub_config_status() -> String {
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
        if let Some(hub_dir) = resources.parent() {
            let licensing_dir = hub_dir.join("UnityLicensingClient_V1");
            let xml_dll = licensing_dir.join("System.Security.Cryptography.Xml.dll");
            let xml_bak = licensing_dir.join("System.Security.Cryptography.Xml.dll.bak");
            if xml_bak.exists() {
                if xml_dll.exists() {
                    fs::remove_file(&xml_dll).map_err(|e| format!("Failed to remove replaced XML DLL: {}", e))?;
                }
                fs::rename(&xml_bak, &xml_dll).map_err(|e| format!("Failed to restore original XML DLL: {}", e))?;
            }
        }
    }

    // 首次创建备份，之后每次从备份恢复，确保在干净副本上补丁
    let bak_str = format!("{}.bak", asar_path.display());
    let bak_path = Path::new(&bak_str);
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

    // 更新本地 hubConfig.json
    eprintln!("Updating local hubConfig.json...");
    if let Err(e) = crate::config_patcher::update_hub_config(disable_signin, disable_update) {
        eprintln!("⚠ Warning: Failed to update hubConfig.json: {}", e);
    } else {
        eprintln!("✓ hubConfig.json updated");
    }

    Ok(format!("Hub patched: {} JS files modified in-place in app.asar (asar kept intact, no DLL replaced)", patched_files))
}

/// 根据 app.asar 路径推导 Hub 可执行文件路径（fuse 存在 exe 里）
fn hub_exe_path(asar_path: &Path) -> Option<PathBuf> {
    let resources = asar_path.parent()?;
    let hub_dir = resources.parent()?;
    #[cfg(target_os = "windows")]
    { Some(hub_dir.join("Unity Hub.exe")) }
    #[cfg(target_os = "macos")]
    { Some(hub_dir.join("Unity Hub.app").join("Contents").join("MacOS").join("Unity Hub")) }
    #[cfg(target_os = "linux")]
    { Some(hub_dir.join("unityhub")) }
}

/// Hub 3.20.0+（Electron 43）的 exe 带 Electron fuses。把
/// `EnableEmbeddedAsarIntegrityValidation`（fuse 4）从 '1' 翻转为 '0'，
/// 使 Electron 不再校验 app.asar 头 JSON 的 SHA256（否则补丁后启动即 FATAL）。
/// 返回是否发生了修改。首次修改前会备份 exe 为 `<exe>.bak`。
fn flip_hub_exe_fuses(exe_path: &Path) -> Result<bool, String> {
    const MAGIC: &[u8] = b"dL7pKGdnNz796PbbjQWNKmHXBZaB9tsX";
    const FUSE4: usize = 4; // EnableEmbeddedAsarIntegrityValidation

    let data = fs::read(exe_path).map_err(|e| format!("Failed to read Hub exe: {}", e))?;
    let pos = find_bytes(&data, MAGIC)
        .ok_or("Hub exe 中未找到 Electron fuses magic（可能不是 3.20.0+）")?;
    let wire_start = pos + MAGIC.len();
    if data.len() < wire_start + 2 + FUSE4 {
        return Err("fuse 段过短".into());
    }
    let version = data[wire_start];
    let length = data[wire_start + 1] as usize;
    if version != 1 {
        return Err(format!("未知的 fuse 版本: {}", version));
    }
    if FUSE4 >= length {
        return Err("fuse 段长度不足（无 fuse4）".into());
    }
    let fuse4_off = wire_start + 2 + FUSE4;
    let cur = data[fuse4_off];
    if cur != b'1' && cur != b'0' {
        return Err(format!("fuse4 字节异常: 0x{:02X}", cur));
    }
    if cur == b'0' {
        return Ok(false); // 已关闭
    }

    // 翻转前备份 exe
    let bak = format!("{}.bak", exe_path.display());
    if !Path::new(&bak).exists() {
        fs::copy(exe_path, &bak).map_err(|e| format!("Failed to backup Hub exe: {}", e))?;
    }
    let mut out = data;
    out[fuse4_off] = b'0';
    fs::write(exe_path, &out).map_err(|e| format!("Failed to write Hub exe: {}", e))?;
    Ok(true)
}

/// 在字节切片中查找子序列，返回偏移
fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
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

    // DefaultLocalConfig 配置
    if file_name.starts_with("DefaultLocalConfig") {
        if disable_signin {
            // 只设置 DisableSignInRequired（登录非必须），不设置 DisableSignIn（保留登录 UI）
            modified = modified.replace("DisableSignInRequired]: false,", "DisableSignInRequired]: true,");
        }
        if disable_update {
            modified = modified.replace("DisableAutoUpdate]: false,", "DisableAutoUpdate]: true,");
        }
    }

    modified
}

/// Restore Hub from backup
pub fn restore_hub() -> Result<String, String> {
    let asar_path = hub_asar_path();
    let resources_path = asar_path.parent().ok_or("Cannot get resources path")?;
    let app_folder = resources_path.join("app");
    let asar_bak = resources_path.join("app.asar.bak");

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

    // 恢复 Hub XML DLL 和 Licensing Client
    let hub_dir = resources_path.parent().ok_or("Cannot get Hub directory")?;
    let licensing_dir = hub_dir.join("UnityLicensingClient_V1");
    let xml_dll_path = licensing_dir.join("System.Security.Cryptography.Xml.dll");
    let xml_dll_bak = licensing_dir.join("System.Security.Cryptography.Xml.dll.bak");

    if xml_dll_bak.exists() {
        if xml_dll_path.exists() {
            fs::remove_file(&xml_dll_path).map_err(|e| format!("Failed to remove patched XML DLL: {}", e))?;
        }
        fs::rename(&xml_dll_bak, &xml_dll_path).map_err(|e| format!("Failed to restore XML DLL: {}", e))?;
        eprintln!("✓ Restored original XML DLL");
    }

    // 恢复 Licensing Client（如果被禁用）
    let licensing_client_exe = licensing_dir.join("Unity.Licensing.Client.exe");
    let licensing_client_bak = licensing_dir.join("Unity.Licensing.Client.exe.bak");

    if !licensing_client_exe.exists() && licensing_client_bak.exists() {
        fs::rename(&licensing_client_bak, &licensing_client_exe).map_err(|e| format!("Failed to restore Licensing Client: {}", e))?;
        eprintln!("✓ Restored Licensing Client");
    }

    // 恢复 Hub exe（撤销 fuse 翻转）
    if let Some(hub_exe) = hub_exe_path(&asar_path) {
        let exe_bak = hub_exe.with_extension("exe.bak");
        if exe_bak.exists() {
            if hub_exe.exists() {
                fs::remove_file(&hub_exe).map_err(|e| format!("Failed to remove patched exe: {}", e))?;
            }
            fs::rename(&exe_bak, &hub_exe).map_err(|e| format!("Failed to restore exe: {}", e))?;
            eprintln!("✓ Restored Hub exe");
        }
    }

    Ok("Restored: app.asar, XML DLL, Licensing Client, Hub exe".into())
}

/// Check if a process is running by name
pub fn check_process_running(name: &str) -> bool {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        let output = std::process::Command::new("tasklist")
            .args(["/FI", &format!("IMAGENAME eq {}", name), "/NH"])
            .creation_flags(0x08000000)
            .output();
        match output {
            Ok(o) => {
                let stdout = String::from_utf8_lossy(&o.stdout);
                stdout.contains(name)
            }
            Err(_) => false,
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

    /// 收集 asar 头里所有文件的元信息（测试辅助）
    fn collect_asar_meta_map(header: &asar::Header, prefix: &Path) -> std::collections::HashMap<PathBuf, AsarFileMeta> {
        let mut out = std::collections::HashMap::new();
        collect_asar_meta(header, prefix, &mut out);
        out
    }
}
