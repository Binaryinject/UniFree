use std::fs;
use std::path::PathBuf;

const BUNDLED_ULF_BYTES: &[u8] = include_bytes!("../resources/Unity_lic.ulf");
const SIGNATURE_BEGIN: &[u8] = b"<Signature";
const SIGNATURE_END: &[u8] = b"</Signature>";

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

fn bundled_ulf_bytes() -> Vec<u8> {
    #[cfg(debug_assertions)]
    {
        let resource_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("resources")
            .join("Unity_lic.ulf");
        if let Ok(bytes) = fs::read(resource_path) {
            return bytes;
        }
    }

    BUNDLED_ULF_BYTES.to_vec()
}

pub fn copy_ulf() -> Result<String, String> {
    let dir = ulf_dir();
    let path = ulf_path();
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let ulf_bytes = bundled_ulf_bytes();

    if !has_signature_node(&ulf_bytes) {
        if let Ok(existing) = fs::read(&path) {
            if has_signature_node(&existing) {
                return Ok(format!("preserved_signed:{}", path.display()));
            }
        }

        return Ok(format!("skipped_missing_signature:{}", path.display()));
    }

    fs::write(&path, ulf_bytes).map_err(|e| e.to_string())?;
    Ok(format!("copied:{}", path.display()))
}

fn contains_bytes(data: &[u8], needle: &[u8]) -> bool {
    data.windows(needle.len()).any(|w| w == needle)
}

fn has_signature_node(bytes: &[u8]) -> bool {
    contains_bytes(bytes, SIGNATURE_BEGIN) && contains_bytes(bytes, SIGNATURE_END)
}

pub fn get_ulf_status() -> String {
    let path = ulf_path();
    if !path.exists() {
        return "unauthorized".into();
    }
    let bundled = bundled_ulf_bytes();

    match fs::read(&path) {
        Ok(bytes) if !has_signature_node(&bytes) => "missing_signature".into(),
        Ok(bytes) if bytes == bundled => "authorized".into(),
        Ok(_) => "mismatch".into(),
        Err(_) => "unknown".into(),
    }
}
