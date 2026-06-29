use std::fs;
use std::path::PathBuf;

fn config_dir() -> PathBuf {
    #[cfg(target_os = "windows")]
    { PathBuf::from(r"C:\ProgramData\Unity\config") }
    #[cfg(target_os = "macos")]
    { PathBuf::from("/Library/Application Support/Unity/config") }
    #[cfg(target_os = "linux")]
    { PathBuf::from("/usr/share/unity3d/config") }
}

fn config_file() -> PathBuf {
    config_dir().join("services-config.json")
}

const CONFIG_CONTENT: &str = r#"{"hubDisableSignInRequired": true}"#;

pub fn write_config() -> Result<String, String> {
    let dir = config_dir();
    let path = config_file();
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    if path.exists() {
        let mut perms = fs::metadata(&path).map_err(|e| e.to_string())?.permissions();
        perms.set_readonly(false);
        fs::set_permissions(&path, perms).map_err(|e| e.to_string())?;
    }
    fs::write(&path, CONFIG_CONTENT).map_err(|e| e.to_string())?;
    Ok(format!("Created: {}", path.display()))
}

pub fn delete_config() -> Result<String, String> {
    let path = config_file();
    if !path.exists() {
        return Ok("Config not found, skipped".to_string());
    }
    let mut perms = fs::metadata(&path).map_err(|e| e.to_string())?.permissions();
    perms.set_readonly(false);
    fs::set_permissions(&path, perms).map_err(|e| e.to_string())?;
    fs::remove_file(&path).map_err(|e| e.to_string())?;
    Ok(format!("Deleted: {}", path.display()))
}
