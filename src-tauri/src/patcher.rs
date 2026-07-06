use std::fs;
use std::path::{Path, PathBuf};

// === Hub paths ===
fn hub_resources_path() -> PathBuf {
    // Check saved config first
    if let Some(custom_path) = crate::app_config::get_hub_install_path() {
        return custom_path.join("resources");
    }

    // Default paths
    #[cfg(target_os = "windows")]
    {
        let pf = std::env::var("PROGRAMFILES").unwrap_or_else(|_| r"C:\Program Files".into());
        PathBuf::from(pf).join("Unity Hub").join("resources")
    }
    #[cfg(target_os = "macos")]
    {
        PathBuf::from("/Applications/Unity Hub.app/Contents/Resources")
    }
    #[cfg(target_os = "linux")]
    {
        PathBuf::from("/usr/share/unityhub/resources")
    }
}

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

/// Patch EntitlementResolver.dll by replacing with pre-patched version
/// The pre-patched DLL has ValidateSignature bypassed
pub fn patch_entitlement_resolver(dll_path: &str) -> Result<String, String> {
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



/// Check Hub status: "patched", "original", "not_found", "error"
pub fn get_hub_status(_resources_path: &str) -> String {
    let asar_path = hub_asar_path();
    let resources_path = match asar_path.parent() {
        Some(p) => p,
        None => return "error".into(),
    };

    // 检查 Hub 的 XML DLL 是否已补丁
    let hub_dir = resources_path.parent().unwrap_or(resources_path);
    let xml_dll_bak = hub_dir.join("UnityLicensingClient_V1").join("System.Security.Cryptography.Xml.dll.bak");
    if xml_dll_bak.exists() {
        return "patched".into();
    }

    // 检查 asar 是否已备份（兼容旧版本）
    let asar_bak = resources_path.join("app.asar.bak");
    if asar_bak.exists() {
        return "patched".into();
    }

    "original".into()
}

pub fn get_hub_config_status() -> String {
    let asar_path = hub_asar_path();
    let resources_path = match asar_path.parent() {
        Some(p) => p,
        None => return "error".into(),
    };

    // 检查 Hub 的 XML DLL 是否已补丁
    let hub_dir = resources_path.parent().unwrap_or(resources_path);
    let xml_dll_bak = hub_dir.join("UnityLicensingClient_V1").join("System.Security.Cryptography.Xml.dll.bak");
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

/// Patch Hub: 修改asar中的JS代码绕过许可证验证 (UniHacker方法)
/// 提取asar到app目录，补丁JS文件，然后重命名asar为.bak
/// Electron会在找不到app.asar时自动从app目录加载
pub fn patch_hub(_resources_path: &str, disable_signin: bool, disable_update: bool) -> Result<String, String> {
    let asar_path = hub_asar_path();
    if !asar_path.exists() {
        return Err("app.asar not found".into());
    }

    eprintln!("Starting Hub patch (JS patching method - extract to folder)...");

    let resources_path = asar_path.parent().ok_or("Cannot get resources path")?;
    let app_folder = resources_path.join("app");
    let asar_bak = resources_path.join("app.asar.bak");
    let unpacked_dir = resources_path.join("app.asar.unpacked");

    // 1. 清理旧的 app 目录
    if app_folder.exists() {
        eprintln!("Removing old app folder...");
        fs::remove_dir_all(&app_folder).map_err(|e| format!("Failed to remove old app folder: {}", e))?;
    }

    // 2. 读取 asar 文件并提取到 app 目录
    eprintln!("Reading app.asar...");
    let asar_data = fs::read(&asar_path).map_err(|e| format!("Failed to read asar: {}", e))?;
    let asar = asar::AsarReader::new(&asar_data, None).map_err(|e| format!("Failed to parse asar: {}", e))?;
    eprintln!("✓ Parsed app.asar ({} files)", asar.files().len());

    let mut patched_files = 0;

    // 3. 提取所有文件到 app 目录，同时补丁 JS 文件
    eprintln!("Extracting files to app/...");
    for (path, file) in asar.files() {
        let full_path = app_folder.join(path);

        // 创建父目录
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("Failed to create dir: {}", e))?;
        }

        let path_str = path.to_string_lossy();

        // 检查是否是需要补丁的 JS 文件
        if path_str.ends_with(".js") {
            let content = String::from_utf8_lossy(file.data());
            let mut modified = content.to_string();
            let file_name = path.file_name().unwrap_or_default().to_string_lossy();

            // 补丁 isLicenseValid: 始终返回 true
            if file_name.starts_with("licenseService") && modified.contains("isLicenseValid") {
                let search = "isLicenseValid() {\n\t\tif (await this.#licenseQueryService.isLicenseValid()) return true;";
                if modified.contains(search) {
                    modified = modified.replace(search, "isLicenseValid() {\n\t\treturn true; // patched by unifree");
                    eprintln!("  ✓ Patched isLicenseValid in {}", path_str);
                    patched_files += 1;
                }
            }

            // 补丁 licenseQueryService.isLicenseValid: 始终返回 true
            if file_name.starts_with("licenseQueryService") {
                if let Some(patched) = replace_method_body(&modified, "async isLicenseValid()", "async isLicenseValid() {\n\t\t\t\treturn true; // patched by unifree\n\t\t\t}") {
                    modified = patched;
                    eprintln!("  ✓ Patched licenseQueryService.isLicenseValid in {}", path_str);
                    patched_files += 1;
                }
                // 不再补丁 getLicense，让 Hub 自己读取 ULF 文件
            }

            // 不再补丁 licensingSdk.init()，让 Hub 正常初始化

            // 补丁翻译键: NO_LICENSE_TEXT -> NO_LICENSE_ACTIVATED
            if modified.contains("NO_LICENSE_TEXT") {
                modified = modified.replace("NO_LICENSE_TEXT", "NO_LICENSE_ACTIVATED");
                eprintln!("  ✓ Fixed NO_LICENSE_TEXT translation key in {}", path_str);
                patched_files += 1;
            }

            // 补丁配置文件
            if path_str.contains("DefaultLocalConfig") {
                if disable_signin {
                    // 只设置 DisableSignInRequired（登录非必须），不设置 DisableSignIn（保留登录 UI）
                    modified = modified.replace("DisableSignInRequired]: false,", "DisableSignInRequired]: true,");
                }
                if disable_update {
                    modified = modified.replace("DisableAutoUpdate]: false,", "DisableAutoUpdate]: true,");
                }
            }

            // 写入修改后的文件
            fs::write(&full_path, modified.as_bytes()).map_err(|e| format!("Failed to write file: {}", e))?;
        } else {
            // 非JS文件直接写入
            fs::write(&full_path, file.data()).map_err(|e| format!("Failed to write file: {}", e))?;
        }
    }
    eprintln!("✓ Extracted {} files to app/", asar.files().len());

    // 4. 复制 app.asar.unpacked 中的原生模块到 app 目录
    if unpacked_dir.exists() {
        eprintln!("Copying native modules from app.asar.unpacked...");
        copy_dir_recursive(&unpacked_dir, &app_folder)?;
        eprintln!("✓ Copied native modules");
    }

    // 5. 备份原始 asar
    if !asar_bak.exists() {
        fs::rename(&asar_path, &asar_bak).map_err(|e| format!("Failed to rename asar: {}", e))?;
        eprintln!("✓ Renamed app.asar to app.asar.bak");
    } else {
        // 如果备份已存在，删除原文件
        fs::remove_file(&asar_path).map_err(|e| format!("Failed to remove original asar: {}", e))?;
        eprintln!("✓ Removed app.asar (backup already exists)");
    }

    // 6. 恢复 Licensing Client（如果被禁用）
    eprintln!("Checking Licensing Client...");
    let hub_dir = resources_path.parent().ok_or("Cannot get Hub directory")?;
    let licensing_dir = hub_dir.join("UnityLicensingClient_V1");
    let licensing_client_exe = licensing_dir.join("Unity.Licensing.Client.exe");
    let licensing_client_bak = licensing_dir.join("Unity.Licensing.Client.exe.bak");

    if !licensing_client_exe.exists() && licensing_client_bak.exists() {
        // 恢复 Licensing Client
        fs::rename(&licensing_client_bak, &licensing_client_exe).map_err(|e| format!("Failed to restore Licensing Client: {}", e))?;
        eprintln!("✓ Restored Licensing Client");
    } else if licensing_client_exe.exists() {
        eprintln!("✓ Licensing Client already exists");
    } else {
        eprintln!("⚠ Licensing Client not found");
    }

    // 7. 替换 Hub 的 XML DLL 跳过签名验证
    eprintln!("Replacing Hub XML DLL...");
    let xml_dll_path = licensing_dir.join("System.Security.Cryptography.Xml.dll");
    let xml_dll_bak = licensing_dir.join("System.Security.Cryptography.Xml.dll.bak");

    if xml_dll_path.exists() {
        if !xml_dll_bak.exists() {
            fs::copy(&xml_dll_path, &xml_dll_bak).map_err(|e| format!("Failed to backup XML DLL: {}", e))?;
            eprintln!("✓ Backed up original XML DLL");
        }
        let patched_dll = include_bytes!("../resources/win/System.Security.Cryptography.Xml.dll");
        fs::write(&xml_dll_path, patched_dll).map_err(|e| format!("Failed to write patched XML DLL: {}", e))?;
        eprintln!("✓ Replaced System.Security.Cryptography.Xml.dll");
    } else {
        eprintln!("⚠ Hub XML DLL not found at: {}", xml_dll_path.display());
    }

    // 8. 更新本地 hubConfig.json
    eprintln!("Updating local hubConfig.json...");
    if let Err(e) = crate::config_patcher::update_hub_config(disable_signin, disable_update) {
        eprintln!("⚠ Warning: Failed to update hubConfig.json: {}", e);
    } else {
        eprintln!("✓ hubConfig.json updated");
    }

    Ok(format!("Hub patched: {} files modified, app folder created, XML DLL replaced", patched_files))
}

/// 递归复制目录
fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), String> {
    for entry in fs::read_dir(src).map_err(|e| format!("Failed to read dir: {}", e))? {
        let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if src_path.is_dir() {
            fs::create_dir_all(&dst_path).map_err(|e| format!("Failed to create dir: {}", e))?;
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path).map_err(|e| format!("Failed to copy file: {}", e))?;
        }
    }
    Ok(())
}

/// Restore Hub from backup
pub fn restore_hub(_resources_path: &str) -> Result<String, String> {
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

    Ok("Restored: app.asar, XML DLL, Licensing Client".into())
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
