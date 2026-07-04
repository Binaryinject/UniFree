use std::fs;
use std::path::PathBuf;

fn ulf_dir() -> PathBuf {
    #[cfg(target_os = "windows")]
    { PathBuf::from(r"C:\ProgramData\Unity") }
    #[cfg(target_os = "macos")]
    { PathBuf::from("/Library/Application Support/Unity") }
    #[cfg(target_os = "linux")]
    { PathBuf::from("/usr/share/unity3d") }
}

fn ulf_path() -> PathBuf {
    ulf_dir().join("Unity_lic.ulf")
}

pub fn copy_ulf() -> Result<String, String> {
    let path = ulf_path();
    if path.exists() {
        return Ok(format!("license_exists:{}", path.display()));
    }
    Err("License file not found. Use generate_license_direct command.".to_string())
}

pub fn get_ulf_status() -> String {
    let path = ulf_path();
    if !path.exists() {
        return "unauthorized".into();
    }

    // 检查是否有签名节点
    match fs::read(&path) {
        Ok(bytes) => {
            let content = String::from_utf8_lossy(&bytes);
            if content.contains("<Signature") {
                "authorized".into()
            } else {
                "missing_signature".into()
            }
        }
        Err(_) => "unknown".into(),
    }
}
