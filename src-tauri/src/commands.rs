use serde::Serialize;
use tauri::command;

use crate::alf_generator;
use crate::license;
use crate::patcher;
use crate::scanner;
use crate::ulf_signer;

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
pub fn generate_alf() -> Result<String, String> {
    let alf_path = alf_generator::generate_alf_file()?;
    Ok(format!("ALF generated: {}", alf_path.display()))
}

#[command]
pub fn generate_license_direct() -> Result<String, String> {
    // 1. 生成 ALF 文件
    let alf_path = alf_generator::generate_alf_file()?;
    eprintln!("ALF generated at: {}", alf_path.display());

    // 2. 转为 ULF（添加空签名，DLL已绕过验证）
    let ulf_path = std::path::PathBuf::from(r"C:\ProgramData\Unity\Unity_lic.ulf");
    let result = ulf_signer::sign_alf_to_ulf(&alf_path, &ulf_path)?;

    Ok(format!("License generated: {}", result))
}
