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

/// Patch EntitlementResolver.dll by replacing with pre-patched version
/// The pre-patched DLL has ValidateSignature bypassed
pub fn patch_entitlement_resolver(dll_path: &str) -> Result<String, String> {
    let path = Path::new(dll_path);
    if !path.exists() {
        return Err("DLL not found".into());
    }

    // 创建备份
    let bak_path = format!("{}.bak", dll_path);
    if !Path::new(&bak_path).exists() {
        fs::copy(path, &bak_path).map_err(|e| e.to_string())?;
    }

    // 使用预编译的补丁DLL
    let patched_dll = include_bytes!("../resources/win/Unity.Licensing.EntitlementResolver.dll");
    fs::write(path, patched_dll).map_err(|e| format!("Failed to write patched DLL: {}", e))?;
    Ok("Patched: replaced with pre-patched DLL".into())
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

    // 检查是否存在备份文件（表示已补丁）
    let asar_bak = resources_path.join("app.asar.bak");
    if asar_bak.exists() {
        return "patched".into();
    }

    // 检查 asar 中是否已补丁
    let config_status = get_hub_config_status();
    if config_status == "not_found" {
        return "not_found".into();
    }
    if config_status == "error" {
        return "error".into();
    }
    if config_status == "patched" || config_status == "patched_no_backup" {
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
    let app_folder = resources_path.join("app");
    let bak_path = asar_path.with_extension("asar.bak");

    // 如果 app 目录存在且 asar 已备份，检查 app 目录中的补丁
    if app_folder.exists() && bak_path.exists() {
        // 检查 app 目录中的 JS 文件
        return match check_patched_in_dir(&app_folder) {
            true => "patched".into(),
            false => "patched_no_backup".into(), // app存在但没bak（不应该发生）
        };
    }

    // 如果 asar 文件存在，检查其中的内容
    if asar_path.exists() {
        let asar_data = match fs::read(&asar_path) {
            Ok(d) => d,
            Err(_) => return "error".into(),
        };

        let asar = match asar::AsarReader::new(&asar_data, None) {
            Ok(a) => a,
            Err(_) => return "error".into(),
        };

        for (path, file) in asar.files() {
            let path_str = path.to_string_lossy();
            if path_str.ends_with(".js") {
                let content = String::from_utf8_lossy(file.data());
                if content.contains("return true; // patched by unifree") ||
                   content.contains("return; // patched by unifree") ||
                   content.contains("DisableSignInRequired]: true,") ||
                   content.contains("DisableAutoUpdate]: true,") {
                    return "patched".into();
                }
            }
        }
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

/// 递归检查目录中的JS文件是否包含补丁标记
fn check_patched_in_dir(dir: &Path) -> bool {
    let Ok(entries) = fs::read_dir(dir) else { return false };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if check_patched_in_dir(&path) {
                return true;
            }
        } else if path.extension().map_or(false, |e| e == "js") {
            if let Ok(content) = fs::read_to_string(&path) {
                if content.contains("return true; // patched by unifree") ||
                   content.contains("return; // patched by unifree") ||
                   content.contains("DisableSignInRequired]: true,") ||
                   content.contains("DisableAutoUpdate]: true,") {
                    return true;
                }
            }
        }
    }
    false
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
                // 补丁 getLicense: 返回假的 Unity Pro 许可证
                if let Some(patched) = replace_method_body(&modified, "async getLicense()", "async getLicense() {\n\t\t\t\treturn [{ id: 'unifree-license', product: 'Unity Pro', licenseType: 'ULF', valid: true, label: 'Unity Pro', startDate: '2024-01-01', expirationDate: '2099-12-31' }];\n\t\t\t}") {
                    modified = patched;
                    eprintln!("  ✓ Patched licenseQueryService.getLicense in {}", path_str);
                    patched_files += 1;
                }
            }

            // 补丁 licensingSdk: 禁用初始化
            if file_name.starts_with("licensingSdk") {
                if let Some(patched) = replace_method_body(&modified, "async init()", "async init() {\n\t\treturn; // patched by unifree\n\t}") {
                    modified = patched;
                    eprintln!("  ✓ Patched init in {}", path_str);
                    patched_files += 1;
                }
            }

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

    // 6. 禁用 Licensing Client（防止无限网络请求）
    eprintln!("Disabling Licensing Client...");
    let hub_dir = resources_path.parent().ok_or("Cannot get Hub directory")?;
    let licensing_client_exe = hub_dir.join("UnityLicensingClient_V1").join("Unity.Licensing.Client.exe");
    let licensing_client_bak = hub_dir.join("UnityLicensingClient_V1").join("Unity.Licensing.Client.exe.bak");

    if licensing_client_exe.exists() {
        if licensing_client_bak.exists() {
            fs::remove_file(&licensing_client_bak).map_err(|e| format!("Failed to remove old backup: {}", e))?;
        }
        fs::rename(&licensing_client_exe, &licensing_client_bak).map_err(|e| format!("Failed to rename Licensing Client: {}", e))?;
        eprintln!("✓ Disabled Licensing Client (renamed to .bak)");
    } else if licensing_client_bak.exists() {
        eprintln!("✓ Licensing Client already disabled");
    }

    // 7. 更新本地 hubConfig.json
    eprintln!("Updating local hubConfig.json...");
    if let Err(e) = crate::config_patcher::update_hub_config(disable_signin, disable_update) {
        eprintln!("⚠ Warning: Failed to update hubConfig.json: {}", e);
    } else {
        eprintln!("✓ hubConfig.json updated");
    }

    Ok(format!("Hub patched: {} files modified, app folder created, Licensing Client disabled", patched_files))
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

    // 恢复 Licensing Client
    let hub_dir = resources_path.parent().ok_or("Cannot get Hub directory")?;
    let licensing_client_exe = hub_dir.join("UnityLicensingClient_V1").join("Unity.Licensing.Client.exe");
    let licensing_client_bak = hub_dir.join("UnityLicensingClient_V1").join("Unity.Licensing.Client.exe.bak");

    if licensing_client_bak.exists() {
        if licensing_client_exe.exists() {
            fs::remove_file(&licensing_client_exe).map_err(|e| format!("Failed to remove backup: {}", e))?;
        }
        fs::rename(&licensing_client_bak, &licensing_client_exe).map_err(|e| format!("Failed to restore Licensing Client: {}", e))?;
    }

    Ok("Restored: app.asar, Licensing Client".into())
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
