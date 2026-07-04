use std::process::Command;

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

/// Add "Open CMD here" to Windows Explorer context menu
#[tauri::command]
pub fn add_cmd_context_menu() -> Result<String, String> {
    #[cfg(target_os = "windows")]
    {
        // Registry keys for folder background context menu
        let commands = vec![
            // Directory background (right-click on empty space in folder)
            r#"reg add "HKEY_CLASSES_ROOT\Directory\Background\shell\OpenCMDHere" /ve /d "在此处打开命令提示符" /f"#,
            r#"reg add "HKEY_CLASSES_ROOT\Directory\Background\shell\OpenCMDHere" /v "Icon" /d "cmd.exe" /f"#,
            r#"reg add "HKEY_CLASSES_ROOT\Directory\Background\shell\OpenCMDHere\command" /ve /d "cmd.exe /s /k pushd \"%%V\"" /f"#,

            // Directory (right-click on folder itself)
            r#"reg add "HKEY_CLASSES_ROOT\Directory\shell\OpenCMDHere" /ve /d "在此处打开命令提示符" /f"#,
            r#"reg add "HKEY_CLASSES_ROOT\Directory\shell\OpenCMDHere" /v "Icon" /d "cmd.exe" /f"#,
            r#"reg add "HKEY_CLASSES_ROOT\Directory\shell\OpenCMDHere\command" /ve /d "cmd.exe /s /k pushd \"%%V\"" /f"#,

            // Drive (right-click on drives)
            r#"reg add "HKEY_CLASSES_ROOT\Drive\shell\OpenCMDHere" /ve /d "在此处打开命令提示符" /f"#,
            r#"reg add "HKEY_CLASSES_ROOT\Drive\shell\OpenCMDHere" /v "Icon" /d "cmd.exe" /f"#,
            r#"reg add "HKEY_CLASSES_ROOT\Drive\shell\OpenCMDHere\command" /ve /d "cmd.exe /s /k pushd \"%%V\"" /f"#,
        ];

        for cmd in commands {
            let output = Command::new("cmd")
                .args(["/C", cmd])
                .creation_flags(0x08000000) // CREATE_NO_WINDOW
                .output()
                .map_err(|e| format!("Failed to execute registry command: {}", e))?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(format!("Registry command failed: {}", stderr));
            }
        }

        Ok("成功添加到右键菜单".to_string())
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err("此功能仅支持 Windows 系统".to_string())
    }
}

/// Remove "Open CMD here" from Windows Explorer context menu
#[tauri::command]
pub fn remove_cmd_context_menu() -> Result<String, String> {
    #[cfg(target_os = "windows")]
    {
        let commands = vec![
            r#"reg delete "HKEY_CLASSES_ROOT\Directory\Background\shell\OpenCMDHere" /f"#,
            r#"reg delete "HKEY_CLASSES_ROOT\Directory\shell\OpenCMDHere" /f"#,
            r#"reg delete "HKEY_CLASSES_ROOT\Drive\shell\OpenCMDHere" /f"#,
        ];

        for cmd in commands {
            let output = Command::new("cmd")
                .args(["/C", cmd])
                .creation_flags(0x08000000) // CREATE_NO_WINDOW
                .output()
                .map_err(|e| format!("Failed to execute registry command: {}", e))?;

            // Ignore errors if key doesn't exist
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                if !stderr.contains("无法找到") && !stderr.contains("cannot find") {
                    return Err(format!("Registry command failed: {}", stderr));
                }
            }
        }

        Ok("成功从右键菜单移除".to_string())
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err("此功能仅支持 Windows 系统".to_string())
    }
}

/// Check if "Open CMD here" context menu is installed
#[tauri::command]
pub fn check_cmd_context_menu() -> bool {
    #[cfg(target_os = "windows")]
    {
        let output = Command::new("cmd")
            .args([
                "/C",
                r#"reg query "HKEY_CLASSES_ROOT\Directory\Background\shell\OpenCMDHere" 2>nul"#,
            ])
            .creation_flags(0x08000000) // CREATE_NO_WINDOW
            .output();

        match output {
            Ok(output) => output.status.success(),
            Err(_) => false,
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        false
    }
}
