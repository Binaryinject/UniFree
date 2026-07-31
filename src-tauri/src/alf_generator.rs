use std::fs;
use std::path::PathBuf;
use alf_generator::AlfGenerator;

/// 生成 ALF 文件内容
pub fn generate_alf_content(product: &str) -> String {
    let generator = AlfGenerator::new().with_product(product);
    generator.generate()
}

/// 获取 ALF 文件路径
pub fn get_alf_path() -> PathBuf {
    let local_app_data = std::env::var("LOCALAPPDATA")
        .unwrap_or_else(|_| {
            let home = std::env::var("USERPROFILE").unwrap_or_else(|_| r"C:\Users\Public".into());
            format!(r"{}\AppData\Local", home)
        });

    let alf_dir = PathBuf::from(local_app_data).join("UniFree");
    let _ = fs::create_dir_all(&alf_dir);

    alf_dir.join("Unity_lic.alf")
}

/// 生成并保存 ALF 文件
pub fn generate_alf_file(product: &str) -> Result<PathBuf, String> {
    let alf_path = get_alf_path();
    let alf_content = generate_alf_content(product);

    fs::write(&alf_path, alf_content)
        .map_err(|e| format!("Failed to write ALF file: {}", e))?;

    eprintln!("ALF file generated at: {}", alf_path.display());
    Ok(alf_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_alf() {
        let content = generate_alf_content("Unity Pro");
        // v1.0.0 format checks
        assert!(content.contains("<MachineBindings>"));
        assert!(content.contains("<NoHardwareCheck"));
        assert!(content.contains("<LicenseVersion"));
        assert!(!content.contains("<SystemInfo>"));
        assert!(!content.contains("<MachineID"));
    }
}
