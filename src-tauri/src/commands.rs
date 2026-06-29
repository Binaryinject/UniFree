use serde::Serialize;
use tauri::command;

use crate::config;
use crate::i18n;
use crate::il_patcher;
use crate::license;
use crate::patcher;
use crate::scanner;

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
pub fn get_system_lang() -> String {
    i18n::get_system_lang()
}

#[command]
pub fn scan_unity_editors() -> Vec<EditorInfo> {
    scanner::scan_installed_editors()
}

#[command]
pub fn check_editor_dll_status(dll_path: String) -> String {
    patcher::get_editor_dll_status(&dll_path)
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
pub fn check_hub_cert_status() -> String {
    patcher::get_hub_cert_status()
}

#[command]
pub fn patch_editor_dll(dll_path: String) -> Result<String, String> {
    patcher::patch_editor(&dll_path)
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
pub fn write_sign_in_config() -> Result<String, String> {
    config::write_config()
}

#[command]
pub fn delete_sign_in_config() -> Result<String, String> {
    config::delete_config()
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
        std::process::Command::new("net")
            .arg("session")
            .creation_flags(0x08000000)
            .status()
            .map(|status| status.success())
            .unwrap_or(false)
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
pub fn get_hub_dll_path() -> String {
    scanner::get_hub_dll_path()
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
        std::process::Command::new(&hub_exe)
            .creation_flags(0x00000008)
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
pub fn patch_dll_il(dll_path: String) -> Result<String, String> {
    il_patcher::patch_signature_validation(&dll_path)
}
