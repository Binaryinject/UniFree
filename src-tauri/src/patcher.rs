use std::fs;
use std::path::{Path, PathBuf};

// === Editor PEM patch ===
const PEM_BEGIN: &[u8] = b"-----BEGIN CERTIFICATE-----";
const PEM_END: &[u8] = b"-----END CERTIFICATE-----";
const MOD_PEM_B64: &str = "TUlJRTd6Q0NBOWVnQXdJQkFnSVVVQ0FiVDVXVEcrTDR1VEVTNE5iYU93dmM0TDh3RFFZSktvWklodmNOQVFFRkJRQXdnZk14Q3pBSgpCZ05WQkFZVEFrUkxNUk13RVFZRFZRUUlEQXBEYjNCbGJtaGhaMlZ1TVJNd0VRWURWUVFIREFwRGIzQmxibWhoWjJWdU1SOHdIUVlEClZRUUtEQlpWYm1sMGVTQlVaV05vYm05c2IyZHBaWE1nUVhCek1Va3dSd1lEVlFRTERFQXVMaTR1TGk0dUxpNHVMaTR1TGk0dUxpNHUKTGk0dUxpNHVMaTR1TGk0dUxpNHVMaTR1TGk0dUxpNHVMaTR1TGk0dUxpNHVMaTR1TGk0dUxpNHVMaTR1TGk0dUxpNHVMaTR1TVFzd0NRWURWUVFEREFKSgpWREVnTUI0R0NTcUdTSWIzRFFFSkFSWVJZV1J0YVc1QWRXNXBkSGt6WkM1amIyMHhIekFkQmdOVkJDa01GaTR1TGk0dUxpNHVMaTR1CkxpNHVMaTR1TGk0dUxpNHdIaGNOTWpNd05ESTJNVEl4TURReldoY05NalF3TkRJMU1USXhNRFF6V2pDQjh6RUxNQWtHQTFVRUJoTUMKUkVzeEV6QVJCZ05WQkFnTUNrTnZjR1Z1YUdGblpXNHhFekFSQmdOVkJBY01Da052Y0dWdWFHRm5aVzR4SHpBZEJnTlZCQW9NRmxWdQphWFI1SUZSbFkyaHViMnh2WjJsbGN5QkJjSE14U1RCSEJnTlZCQXNNUUM0dUxpNHVMaTR1TGk0dUxpNHVMaTR1TGk0dUxpNHVMaTR1CkxpNHVMaTR1TGk0dUxpNHVMaTR1TGk0dUxpNHVMaTR1TGk0dUxpNHVMaTR1TGk0dUxpNHVMaTR1TGk0dUxpNHVMaTR1TGk0dUxpNHVMaTR4Q3pBSkJnTlZCQU1NQWtsVU1TQXdIZ1lKCktvWklodmNOQVFrQkZoRmhaRzFwYmtCMWJtbDBlVE5rTG1OdmJURWZNQjBHQTFVRUtRd1dMaTR1TGk0dUxpNHVMaTR1TGk0dUxpNHUKTGk0dUxqQ0NBU0l3RFFZSktvWklodmNOQVFFQkJRQURnZ0VQQURDQ0FRb0NnZ0VCQUwzdC8zb3NzaW9XYnpNZTZTSzEzWm43cnNrUAprTHROSmpsTlViRlJ4ejdJekpva29HMzg3TlNYT2xPQW4ycUVISVJwRE5zeERUSDIwZmtMellCd2UwVHI1YTJ1OWwrRE9XcFJudDdKCnpNSmFUQldpV3NyTG5aaHQ1ZVBSajdWbjdjMjVxbTdQZHE5aXVZcjB6S21xRVczK2VOSDdhNlBIZ1F5SlJMa2svenVFMGRyY3pCa24KUkVhek9tQUNpcjlvMWdqVS9VMzZGWU4rdjNyNHNFTEhES21KNUorUXJ4Zm14RnNYelh1WmM4d1RyRThweFdYSUlXTFpSamljL3pLeApWUWlmWVVROXdVaEZKdVRkQnl1SEtoaTM3dUc4alVOeWZQZTFQQ2ZmUWhMUmIycGJDdzVraktma0p3SmV4aGQrTUJzT0RGYm9DM29oCnJ5RDBWd3EzNjc4Q0F3RUFBYU41TUhjd0RnWURWUjBQQVFIL0JBUURBZ1dnTUIwR0ExVWRKUVFXTUJRR0NDc0dBUVVGQndNQ0JnZ3IKQmdFRkJRY0RCREFKQmdOVkhSTUVBakFBTUJ3R0ExVWRFUVFWTUJPQkVXRmtiV2x1UUhWdWFYUjVNMlF1WTI5dE1CMEdBMVVkRGdRVwpCQlE2MUc1WE13b2Q0dHJOVlVCWHpsYkZTZEEweXpBTkJna3Foa2lHOXcwQkFRVUZBQU9DQVFFQUltdmZ2QXZWNVlxcT2nakYxNlNiCnk5TUdJUWlZanlWQklEb0h4d1huZUhDQWF4UjZJS1MrNWVsd1VydUovRCtCSzZ2SzA5SnU5RmxCeStacEJQMkl0K3BaaGgKdzI0cStNdk5OdURyS3E3RkhQeEVGQnE3WDJvQ0JvT3ZJbCt2K2hBMG1mZTgyK2pKdllXTE9UWVhacm9FZE9EZW9QZVRqeApkamRtQ20xY1BsK0t0K2FyYnE0Q2E2d3JqTmdrV0xTR1VYZ2JnZ3J0Z2Z6ZnpSYkxPZWd0RklKbGpQc3JrZEJjK01iS2FqCnJ5RDBWd3EzNjc4Q0F3RUFBYU41TUhjd0RnWURWUjBQQVFIL0JBUURBZ1dnTUIwR0ExVWRKUVFXTUJRR0NDc0dBUVVGQndNQ0JnZ3IKQmdFRkJRY0RCREFKQmdOVkhSTUVBakFBTUJ3R0ExVWRFUVFWTUJPQkVXRmtiV2x1UUhWdWFYUjVNMlF1WTI5dE1CMEdBMVVkRGdRVwpCQlE2MUc1WE13b2Q0dHJOVlVCWHpsYkZTZEEweXpBTkJna3Foa2lHOXcwQkFRVUZBQU9DQVFFQUltdmZ2QXZWNVlxcT2nakYxNlNiCnk5TUdJUWlZanlWQklEb0h4d1huZUhDQWF4UjZJS1MrNWVsd1VydUovRCtCSzZ2SzA5SnU5RmxCeStacEJQMkl0K3BaaGgKdzI0cStNdk5OdURyS3E3RkhQeEVGQnE3WDJvQ0JvT3ZJbCt2K2hBMG1mZTgyK2pKdllXTE9UWVhacm9FZE9EZW9QZVRqeApkamRtQ20xY1BsK0t0K2FyYnE0Q2E2d3JqTmdrV0xTR1VYZ2JnZ3J0Z2Z6ZnpSYkxPZWd0RklKbGpQc3JrZEJjK01iS2FqCnJ5RDBWd3EzNjc4Q0F3RUFBYU41TUhjd0RnWURWUjBQQVFIL0JBUURBZ1dnTUIwR0ExVWRKUVFXTUJRR0NDc0dBUVVGQndNQ0JnZ3IKQmdFRkJRY0RCREFKQmdOVkhSTUVBakFBTUJ3R0ExVWRFUVFWTUJPQkVXRmtiV2x1UUhWdWFYUjVNMlF1WTI5dE1CMEdBMVVkRGdRVwpCQlE2MUc1WE13b2Q0dHJOVlVCWHpsYkZTZEEweXpBTkJna3Foa2lHOXcwQkFRVUZBQU9DQVFFQUltdmZ2QXZWNVlxcT2nakYxNlNiCnk5TUdJUWlZanlWQklEb0h4d1huZUhDQWF4UjZJS1MrNWVsd1VydUovRCtCSzZ2SzA5SnU5RmxCeStacEJQMkl0K3BaaGgKdzI0cStNdk5OdURyS3E3RkhQeEVGQnE3WDJvQ0JvT3ZJbCt2K2hBMG1mZTgyK2pKdllXTE9UWVhacm9FZE9EZW9QZVRqeApkamRtQ20xY1BsK0t0K2FyYnE0Q2E2d3JqTmdrV0xTR1VYZ2JnZ3J0Z2Z6ZnpSYkxPZWd0RklKbGpQc3JrZEJjK01iS2FqCnJ5RDBWd3EzNjc4Q0F3RUFBYU41TUhjd0RnWURWUjBQQVFIL0JBUURBZ1dnTUIwR0ExVWRKUVFXTUJRR0NDc0dBUVVGQndNQ0JnZ3IKQmdFRkJRY0RCREFKQmdOVkhSTUVBakFBTUJ3R0ExVWRFUVFWTUJPQkVXRmtiV2x1UUhWdWFYUjVNMlF1WTI5dE1CMEdBMVVkRGdRVwpCQlE2MUc1WE13b2Q0dHJOVlVCWHpsYkZTZEEweXpBTkJna3Foa2lHOXcwQkFRVUZBQU9DQVFFQUltdmZ2QXZWNVlxcT2nakYxNlNiCnk5TUdJUWlZanlWQklEb0h4d1huZUhDQWF4UjZJS1MrNWVsd1VydUovRCtCSzZ2SzA5SnU5RmxCeStacEJQMkl0K3BaaGgKdzI0cStNdk5OdURyS3E3RkhQeEVGQnE3WDJvQ0JvT3ZJbCt2K2hBMG1mZTgyK2pKdllXTE9UWVhacm9FZE9EZW9QZVRqeApkamRtQ20xY1BsK0t0K2FyYnE0Q2E2d3JqTmdrV0xTR1VYZ2JnZ3J0Z2Z6ZnpSYkxPZWd0RklKbGpQc3JrZEJjK01iS2Fq";

// === Hub paths ===
fn hub_resources_path() -> PathBuf {
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

fn hub_dll_path() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        let pf = std::env::var("PROGRAMFILES").unwrap_or_else(|_| r"C:\Program Files".into());
        PathBuf::from(pf)
            .join("Unity Hub")
            .join("UnityLicensingClient_V1")
            .join("Unity.Licensing.EntitlementResolver.dll")
    }
    #[cfg(target_os = "macos")]
    {
        PathBuf::from("/Applications/Unity Hub.app/Contents/Resources")
            .join("UnityLicensingClient_V1")
            .join("Unity.Licensing.EntitlementResolver.dll")
    }
    #[cfg(target_os = "linux")]
    {
        PathBuf::from("/usr/share/unityhub")
            .join("UnityLicensingClient_V1")
            .join("Unity.Licensing.EntitlementResolver.dll")
    }
}

fn restore_hub_resolver_if_backup_exists() -> Result<bool, String> {
    let dll_path = hub_dll_path();
    let bak_path = PathBuf::from(format!("{}.bak", dll_path.to_string_lossy()));
    if !dll_path.exists() || !bak_path.exists() {
        return Ok(false);
    }

    let current = fs::read(&dll_path).map_err(|e| e.to_string())?;
    let backup = fs::read(&bak_path).map_err(|e| e.to_string())?;
    if current == backup {
        return Ok(false);
    }

    fs::write(&dll_path, backup).map_err(|e| e.to_string())?;
    Ok(true)
}

// === PEM replacement for Editor ===

fn find_pem_block(data: &[u8]) -> Option<(usize, usize)> {
    // 查找第一个 PEM 证书块
    let begin_pos = data.windows(PEM_BEGIN.len()).position(|w| w == PEM_BEGIN)?;
    let after_begin = &data[begin_pos + PEM_BEGIN.len()..];
    let end_pos = after_begin.windows(PEM_END.len()).position(|w| w == PEM_END)?;
    let end_abs = begin_pos + PEM_BEGIN.len() + end_pos + PEM_END.len();

    eprintln!("  Found PEM certificate at offset: {}", begin_pos);

    Some((begin_pos, end_abs))
}

fn detect_newline(data: &[u8], pem_start: usize) -> &'static [u8] {
    let after_begin = pem_start + PEM_BEGIN.len();
    if after_begin + 1 < data.len() && data[after_begin] == b'\r' && data[after_begin + 1] == b'\n' {
        b"\r\n"
    } else {
        b"\n"
    }
}

fn build_pem_block(newline: &[u8]) -> Vec<u8> {
    let mut pem = Vec::new();
    pem.extend_from_slice(PEM_BEGIN);
    pem.extend_from_slice(newline);
    let decoded = base64_decode(MOD_PEM_B64);
    let encoded = base64_encode(&decoded);
    for chunk in encoded.as_bytes().chunks(64) {
        pem.extend_from_slice(chunk);
        pem.extend_from_slice(newline);
    }
    pem.extend_from_slice(PEM_END);
    pem
}

fn base64_decode(input: &str) -> Vec<u8> {
    const TABLE: [u8; 128] = [
        0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
        0,0,0,0,0,0,0,0,0,0,0,62,0,0,0,63,52,53,54,55,56,57,58,59,60,61,0,0,0,0,0,0,
        0,0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,21,22,23,24,25,0,0,0,0,0,
        0,26,27,28,29,30,31,32,33,34,35,36,37,38,39,40,41,42,43,44,45,46,47,48,49,50,51,0,0,0,0,0,
    ];
    let mut output = Vec::new();
    let mut buf: u32 = 0;
    let mut bits: u32 = 0;
    for &b in input.as_bytes() {
        if b == b'=' || b > 127 { continue; }
        let val = TABLE[b as usize] as u32;
        buf = (buf << 6) | val;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            output.push((buf >> bits) as u8);
            buf &= (1 << bits) - 1;
        }
    }
    output
}

fn base64_encode(input: &[u8]) -> String {
    const TABLE: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut output = String::new();
    for chunk in input.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let triple = (b0 << 16) | (b1 << 8) | b2;
        output.push(TABLE[(triple >> 18 & 0x3F) as usize] as char);
        output.push(TABLE[(triple >> 12 & 0x3F) as usize] as char);
        if chunk.len() > 1 {
            output.push(TABLE[(triple >> 6 & 0x3F) as usize] as char);
        } else {
            output.push('=');
        }
        if chunk.len() > 2 {
            output.push(TABLE[(triple & 0x3F) as usize] as char);
        } else {
            output.push('=');
        }
    }
    output
}

// === Asar patching for Hub ===

struct AsarHeader {
    json_start: usize,
    json_size: usize,
    data_start: usize,
}

fn parse_asar_header(data: &[u8]) -> Option<AsarHeader> {
    if data.len() < 16 {
        return None;
    }
    let header_size = u32::from_le_bytes([data[4], data[5], data[6], data[7]]) as usize;
    // Asar header format:
    // 4 bytes: size of size (always 4)
    // 4 bytes: header size
    // 4 bytes: header size (duplicate)
    // 4 bytes: header size (duplicate)
    // JSON starts at offset 16
    let json_start = 16;
    let json_size = header_size - 12; // Subtract the 12 bytes before JSON
    let data_start = json_start + json_size;
    Some(AsarHeader { json_start, json_size, data_start })
}

fn find_file_in_asar(header_json: &str, filename: &str) -> Option<(usize, usize)> {
    // Find the file entry in the header JSON
    let pattern = format!("\"{}\":", filename);
    let pos = header_json.find(&pattern)?;
    let after = &header_json[pos + pattern.len()..];
    // Find "size" value (can be number or string)
    let size_start = after.find("\"size\":")? + 7;
    let size_str = after[size_start..].trim_start();
    let size_end = if size_str.starts_with('"') {
        // String value: "1234"
        let end = size_str[1..].find('"')? + 1;
        size_str[1..end].parse::<usize>().ok()?
    } else {
        // Number value: 1234
        let end = size_str.find(|c: char| !c.is_ascii_digit())?;
        size_str[..end].trim().parse::<usize>().ok()?
    };
    // Find "offset" value (can be number or string)
    let offset_start = after.find("\"offset\":")? + 9;
    let offset_str = after[offset_start..].trim_start();
    let offset = if offset_str.starts_with('"') {
        // String value: "1234"
        let end = offset_str[1..].find('"')? + 1;
        offset_str[1..end].parse::<usize>().ok()?
    } else {
        // Number value: 1234
        let end = offset_str.find(|c: char| !c.is_ascii_digit())?;
        offset_str[..end].trim().parse::<usize>().ok()?
    };
    Some((offset, size_end))
}

fn patch_asar_file(data: &mut Vec<u8>, filename: &str, replacements: &[(&str, &str)]) -> Result<bool, String> {
    let header = parse_asar_header(data).ok_or("Invalid asar header")?;
    let header_json = String::from_utf8_lossy(&data[header.json_start..header.json_start + header.json_size]).to_string();
    let (file_offset, file_size) = find_file_in_asar(&header_json, filename)
        .ok_or(format!("File not found in asar: {}", filename))?;
    let abs_offset = header.data_start + file_offset;
    if abs_offset + file_size > data.len() {
        return Err("File extends beyond asar".into());
    }
    let file_content = String::from_utf8_lossy(&data[abs_offset..abs_offset + file_size]).to_string();
    let mut modified = file_content.clone();
    for (from, to) in replacements {
        // Pad replacement with spaces to maintain same file size
        let from_len = from.len();
        let to_len = to.len();
        let padded_to = if to_len < from_len {
            format!("{}{}", to, " ".repeat(from_len - to_len))
        } else {
            to.to_string()
        };
        modified = modified.replace(from, &padded_to);
    }
    if modified == file_content {
        return Ok(false);
    }
    let new_bytes = modified.as_bytes();
    if new_bytes.len() != file_size {
        return Err(format!("Replacement changed file size ({} -> {}), not supported", file_size, new_bytes.len()));
    }
    data[abs_offset..abs_offset + file_size].copy_from_slice(new_bytes);
    Ok(true)
}

fn contains_bytes(data: &[u8], needle: &[u8]) -> bool {
    data.windows(needle.len()).any(|w| w == needle)
}

/// Check Editor DLL status: Always return "original" since we don't patch it anymore
pub fn get_editor_dll_status(dll_path: &str) -> String {
    let path = Path::new(dll_path);
    if !path.exists() {
        return "not_found".into();
    }
    // Editor DLL 不再需要补丁
    "original".into()
}

/// Patch Editor DLL: No longer patches EntitlementResolver.dll
/// Editor should connect to Hub's Licensing Client which already has cracked CryptoXml.dll
pub fn patch_editor(dll_path: &str) -> Result<String, String> {
    // Editor 不需要单独补丁，因为它会使用 Hub 的 Licensing Client
    // Hub 的 Licensing Client 已经替换了 System.Security.Cryptography.Xml.dll
    return Err("Editor patching is not needed. Hub's Licensing Client is already patched.".into());
}

/// Restore Editor DLL from backup
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
    let config_status = get_hub_config_status();
    let resolver_status = get_hub_cert_status();

    if config_status == "not_found" {
        return "not_found".into();
    }
    if config_status == "error" || resolver_status == "error" {
        return "error".into();
    }
    if resolver_status == "patched" || resolver_status == "patched_no_backup" {
        return "partial".into();
    }
    config_status
}

pub fn get_hub_config_status() -> String {
    let asar_path = hub_asar_path();
    if !asar_path.exists() {
        return "not_found".into();
    }

    let data = match fs::read(&asar_path) {
        Ok(d) => d,
        Err(_) => return "error".into(),
    };

    let bak_path = asar_path.with_extension("asar.bak");
    let is_patched = contains_bytes(
        &data,
        b"[LOCAL_CONFIG_SETTINGS.DisableSignInRequired]: true",
    ) || contains_bytes(
        &data,
        b"[LOCAL_CONFIG_SETTINGS.DisableAutoUpdate]: true",
    );

    if is_patched {
        if bak_path.exists() {
            return "patched".into();
        }
        return "patched_no_backup".into();
    }

    "original".into()
}

pub fn get_hub_cert_status() -> String {
    get_editor_dll_status(&hub_dll_path().to_string_lossy())
}

/// Patch Hub: UniHacker TRUE method - Replace System.Security.Cryptography.Xml.dll
pub fn patch_hub(_resources_path: &str, disable_signin: bool, disable_update: bool) -> Result<String, String> {
    let asar_path = hub_asar_path();
    if !asar_path.exists() {
        return Err("app.asar not found".into());
    }

    eprintln!("Starting Hub patch (UniHacker method)...");

    // 1. 替换 System.Security.Cryptography.Xml.dll (真正的 UniHacker 方法)
    eprintln!("Replacing System.Security.Cryptography.Xml.dll...");
    let target_dll = if cfg!(target_os = "windows") {
        PathBuf::from(r"C:\Program Files\Unity Hub\UnityLicensingClient_V1\System.Security.Cryptography.Xml.dll")
    } else {
        return Err("This method only works on Windows".into());
    };

    if !target_dll.exists() {
        return Err(format!("Target DLL not found: {}", target_dll.display()));
    }

    // 备份原始 DLL
    let backup_dll = target_dll.with_extension("dll.bak");
    if !backup_dll.exists() {
        eprintln!("  Creating backup...");
        fs::copy(&target_dll, &backup_dll)
            .map_err(|e| format!("Failed to backup DLL: {}", e))?;
        eprintln!("  ✓ Backup created");
    }

    // 获取 cracked DLL 路径
    let cracked_dll_data = include_bytes!("../resources/cracked_CryptoXml.dll");

    eprintln!("  Original DLL: {} bytes", fs::metadata(&target_dll).unwrap().len());
    eprintln!("  Cracked DLL: {} bytes", cracked_dll_data.len());

    // 替换 DLL
    fs::write(&target_dll, cracked_dll_data)
        .map_err(|e| format!("Failed to replace DLL: {}", e))?;
    eprintln!("✓ System.Security.Cryptography.Xml.dll replaced with cracked version");

    restore_hub_resolver_if_backup_exists()?;

    // 2. 修改 Hub 配置
    let bak_path = asar_path.with_extension("asar.bak");
    if !bak_path.exists() {
        fs::copy(&asar_path, &bak_path).map_err(|e| e.to_string())?;
    }

    let mut data = fs::read(&asar_path).map_err(|e| e.to_string())?;
    let mut replacements = Vec::new();
    if disable_signin {
        replacements.push(("[LOCAL_CONFIG_SETTINGS.DisableSignInRequired]: false", "[LOCAL_CONFIG_SETTINGS.DisableSignInRequired]: true"));
    }
    if disable_update {
        replacements.push(("[LOCAL_CONFIG_SETTINGS.DisableAutoUpdate]: false", "[LOCAL_CONFIG_SETTINGS.DisableAutoUpdate]: true"));
    }
    if !replacements.is_empty() {
        eprintln!("Patching Hub config...");
        patch_asar_file(&mut data, "LocalConfig-DXicEBJ4.js", &replacements)?;
        fs::write(&asar_path, &data).map_err(|e| e.to_string())?;
        eprintln!("✓ Hub config patched");
    }

    // 3. 确认授权文件存在
    let license_path = r"C:\ProgramData\Unity\Unity_lic.ulf";
    if Path::new(license_path).exists() {
        eprintln!("✓ License file exists");
    } else {
        eprintln!("⚠ Warning: License file not found, please copy it manually");
    }

    Ok("Patched: Cracked CryptoXml.dll + Hub config (UniHacker method)".into())
}

/// Restore Hub from backup
pub fn restore_hub(_resources_path: &str) -> Result<String, String> {
    let dll_path = hub_dll_path();
    let dll_path_str = dll_path.to_string_lossy();
    let dll_bak = format!("{}.bak", dll_path_str);
    if Path::new(&dll_bak).exists() {
        restore(&dll_path_str)?;
    }

    let asar_path = hub_asar_path();
    let bak_path = asar_path.with_extension("asar.bak");
    if bak_path.exists() {
        fs::copy(&bak_path, &asar_path).map_err(|e| e.to_string())?;
        fs::remove_file(&bak_path).map_err(|e| e.to_string())?;
    }
    Ok("Restored: app.asar".into())
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
