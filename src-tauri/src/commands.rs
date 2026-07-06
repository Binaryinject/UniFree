use serde::Serialize;
use tauri::command;
use tauri::AppHandle;

use crate::alf_generator;
use crate::app_config;
use crate::license;
use crate::patcher;
use crate::scanner;
use crate::ulf_signer;
use crate::updater;

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[derive(Serialize)]
pub struct EditorInfo {
    pub version: String,
    pub path: String,
    pub dll_path: String,
    pub dll_status: String,
    pub product_name: String,
    pub architecture: String,
}

#[command]
pub fn scan_unity_editors() -> Vec<EditorInfo> {
    scanner::scan_installed_editors()
}

#[command]
pub fn check_hub_dll_status() -> String {
    let res_path = scanner::hub_resources_path();
    patcher::get_hub_status(&res_path.to_string_lossy())
}

#[command]
pub fn check_hub_config_status() -> String {
    patcher::get_hub_config_status()
}

#[command]
pub fn patch_editor_dll(dll_path: String) -> Result<String, String> {
    patcher::patch_entitlement_resolver(&dll_path)
}

#[command]
pub fn patch_hub(disable_signin: bool, disable_update: bool) -> Result<String, String> {
    let res_path = scanner::hub_resources_path();
    patcher::patch_hub(&res_path.to_string_lossy(), disable_signin, disable_update)
}

#[command]
pub fn restore_hub() -> Result<String, String> {
    let res_path = scanner::hub_resources_path();
    patcher::restore_hub(&res_path.to_string_lossy())
}

#[command]
pub fn restore_dll(dll_path: String) -> Result<String, String> {
    patcher::restore(&dll_path)
}



#[command]
pub fn copy_license() -> Result<String, String> {
    license::copy_ulf()
}

#[command]
pub fn check_license_status() -> String {
    license::get_ulf_status()
}

#[command]
pub fn check_admin() -> bool {
    #[cfg(target_os = "windows")]
    {
        // 使用 Windows API 直接检测管理员权限，无需启动外部进程
        use windows::Win32::Security::{GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY};
        use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};
        
        unsafe {
            let mut token_handle = Default::default();
            if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token_handle).is_ok() {
                let mut elevation: TOKEN_ELEVATION = Default::default();
                let mut return_length = 0u32;
                if GetTokenInformation(
                    token_handle,
                    TokenElevation,
                    Some(&mut elevation as *mut _ as *mut _),
                    std::mem::size_of::<TOKEN_ELEVATION>() as u32,
                    &mut return_length,
                ).is_ok()
                {
                    return elevation.TokenIsElevated != 0;
                }
            }
            false
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        unsafe { libc::geteuid() == 0 }
    }
}

#[command]
pub fn relaunch_as_admin() -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        std::process::Command::new("powershell")
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-ExecutionPolicy",
                "Bypass",
                "-Command",
                "Start-Process -FilePath $args[0] -Verb RunAs",
            ])
            .arg(exe)
            .creation_flags(0x08000000)
            .spawn()
            .map_err(|e| e.to_string())?;
        Ok(())
    }
    #[cfg(not(target_os = "windows"))]
    {
        Err("Administrator relaunch is only supported on Windows".into())
    }
}

#[command]
pub fn check_process(name: String) -> bool {
    patcher::check_process_running(&name)
}

#[command]
pub fn kill_process(name: String) -> Result<(), String> {
    patcher::kill_process(&name)
}

#[command]
pub fn open_browser(url: String) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/c", "start", &url])
            .creation_flags(0x08000000)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(not(target_os = "windows"))]
    {
        std::process::Command::new("xdg-open")
            .arg(&url)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[command]
pub fn launch_hub() -> Result<(), String> {
    let hub_exe = scanner::hub_resources_path()
        .parent()
        .ok_or("Cannot find Unity Hub root")?
        .join("Unity Hub.exe");
    if !hub_exe.exists() {
        return Err("Unity Hub.exe not found".into());
    }
    #[cfg(target_os = "windows")]
    {
        // Use explorer.exe to launch without inheriting admin privileges
        std::process::Command::new("explorer.exe")
            .arg(&hub_exe)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(not(target_os = "windows"))]
    {
        std::process::Command::new(&hub_exe)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[command]
pub fn generate_alf() -> Result<String, String> {
    let alf_path = alf_generator::generate_alf_file("Unity Pro")?;
    Ok(format!("ALF generated: {}", alf_path.display()))
}

#[command]
pub fn generate_license_direct(product: String, private_key_pem: Option<String>) -> Result<String, String> {
    // 1. 生成 ALF 文件（根据产品类型选择不同 feature 集）
    let alf_path = alf_generator::generate_alf_file(&product)?;
    eprintln!("ALF generated at: {}", alf_path.display());

    // 2. 转为 ULF（用RSA密钥签名）
    let ulf_path = std::path::PathBuf::from(r"C:\ProgramData\Unity\Unity_lic.ulf");
    let result = ulf_signer::sign_alf_to_ulf(&alf_path, &ulf_path, private_key_pem.as_deref())?;

    Ok(format!("License generated: {}", result))
}

#[command]
pub fn get_hub_path() -> String {
    let resources = scanner::hub_resources_path();
    resources.parent()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default()
}

#[command]
pub async fn select_hub_path(app: tauri::AppHandle) -> Result<String, String> {
    use tauri_plugin_dialog::{DialogExt, FilePath};

    let file = app.dialog().file()
        .add_filter("Unity Hub", &["exe"])
        .set_title("Select Unity Hub.exe")
        .blocking_pick_file()
        .ok_or("No file selected")?;

    let path = match file {
        FilePath::Path(p) => p,
        _ => return Err("Invalid file path".into()),
    };

    let exe_path = path;

    // Verify it's Unity Hub.exe
    let exe_name = exe_path.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    if !exe_name.eq_ignore_ascii_case("Unity Hub.exe") {
        return Err("Please select Unity Hub.exe".into());
    }

    let install_dir = exe_path.parent()
        .ok_or("Cannot get install directory")?;

    // Verify resources folder exists
    let resources_dir = install_dir.join("resources");
    if !resources_dir.exists() {
        return Err("Invalid Unity Hub installation: resources folder not found".into());
    }

    // Save to config
    app_config::set_hub_install_path(&install_dir.to_string_lossy())?;

    Ok(install_dir.to_string_lossy().to_string())
}

#[command]
pub fn reset_hub_path() -> Result<(), String> {
    app_config::reset_hub_install_path()
}

#[command]
pub fn get_editor_scan_paths() -> Vec<String> {
    app_config::get_editor_scan_paths()
}

#[command]
pub async fn add_editor_scan_path(app: tauri::AppHandle) -> Result<String, String> {
    use tauri_plugin_dialog::{DialogExt, FilePath};

    let folder = app.dialog().file()
        .set_title("Select Unity Editor Directory")
        .blocking_pick_folder()
        .ok_or("No folder selected")?;

    let path = match folder {
        FilePath::Path(p) => p,
        _ => return Err("Invalid path".into()),
    };

    let path_str = path.to_string_lossy().to_string();
    app_config::add_editor_scan_path(&path_str)?;
    Ok(path_str)
}

#[command]
pub fn remove_editor_scan_path(path: String) -> Result<(), String> {
    app_config::remove_editor_scan_path(&path)
}

#[command]
pub async fn check_update() -> Result<Option<updater::UpdateInfo>, String> {
    let version = env!("CARGO_PKG_VERSION");
    updater::check_for_update(version).await
}

#[command]
pub async fn download_update(app: AppHandle, download_url: String, file_name: String) -> Result<(), String> {
    updater::download_and_install(app, &download_url, &file_name).await
}

#[command]
pub fn cancel_update_download() {
    updater::cancel_download();
}
