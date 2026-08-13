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

pub fn ulf_path() -> PathBuf {
    ulf_dir().join("Unity_lic.ulf")
}

pub fn copy_ulf() -> Result<String, String> {
    let path = ulf_path();
    if !path.exists() {
        return Err("License file not found. Use generate_license_direct command.".to_string());
    }
    // 返回带前缀的状态，供前端区分「保留已签名 / 跳过缺签名 / 已就绪」三种情况
    match get_ulf_status().as_str() {
        "authorized" => Ok(format!("preserved_signed:{}", path.display())),
        "missing_signature" => Ok(format!("skipped_missing_signature:{}", path.display())),
        _ => Ok(format!("license_exists:{}", path.display())),
    }
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
