//! # ALF Generator
//!
//! Unity ALF (Activation License File) generator compatible with Unity Licensing Client.
//!
//! This library generates ALF files that are compatible with Unity's licensing system.
//! Uses v1.0.0 format with NoHardwareCheck and real machine bindings.

use sha1::{Digest, Sha1};
use base64::{Engine as _, engine::general_purpose};

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

/// ALF 生成器
pub struct AlfGenerator {
    unity_version: String,
}

impl AlfGenerator {
    /// 创建新的 ALF 生成器
    pub fn new() -> Self {
        Self {
            unity_version: "2017.2.0".to_string(),
        }
    }

    /// 设置 Unity 版本
    pub fn with_unity_version(mut self, version: &str) -> Self {
        self.unity_version = version.to_string();
        self
    }

    /// 生成 ALF 内容 (v1.0.0 format with NoHardwareCheck + real machine bindings)
    pub fn generate(&self) -> String {
        let bindings = self.get_machine_bindings();
        let machine_id = self.generate_machine_id(&bindings);
        let serial_hash = self.generate_serial_hash(&machine_id);

        let mut alf_content = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<root>\n    <License id=\"Terms\">\n        <NoHardwareCheck Value=\"true\"/>\n        <MachineBindings>\n");

        // 写入真实机器绑定
        for (key, value) in &bindings {
            alf_content.push_str(&format!("            <Binding Key=\"{}\" Value=\"{}\" />\n", key, value));
        }

        alf_content.push_str("        </MachineBindings>\n");
        alf_content.push_str(&format!("        <SerialHash Value=\"{}\" />\n", serial_hash));
        
        // Features
        alf_content.push_str("        <Features>\n");
        let features = vec![0, 2, 4, 9, 13, 20, 21, 22, 30, 39, 40, 60, 65];
        for f in &features {
            alf_content.push_str(&format!("            <Feature Value=\"{}\" />\n", f));
        }
        alf_content.push_str("        </Features>\n");

        // DeveloperData
        let developer_data = self.generate_developer_data(&machine_id);
        alf_content.push_str(&format!("        <DeveloperData Value=\"{}\" />\n", developer_data));

        // SerialMasked
        let serial_masked = self.generate_serial_masked();
        alf_content.push_str(&format!("        <SerialMasked Value=\"{}\" />\n", serial_masked));

        alf_content.push_str(&format!("        <LicenseVersion Value=\"6.x\" />\n"));
        alf_content.push_str(&format!("        <ClientProvidedVersion Value=\"{}\" />\n", self.unity_version));
        alf_content.push_str("        <AlwaysOnline Value=\"false\" />\n");
        alf_content.push_str("    </License>\n</root>");

        alf_content
    }

    /// 获取机器绑定信息
    fn get_machine_bindings(&self) -> Vec<(i32, String)> {
        let mut bindings = Vec::new();

        #[cfg(target_os = "windows")]
        {
            // MachineBinding1 = Windows Product ID
            if let Some(product_id) = windows::get_windows_product_id() {
                if !product_id.is_empty() {
                    bindings.push((1, product_id));
                }
            }

            // MachineBinding2 = FlipAndCodeBytes(BootDriveSerialNumber)
            if let Some(serial) = windows::get_boot_drive_serial_number() {
                if !serial.is_empty() {
                    let encoded = flip_and_code_bytes(&serial);
                    if !encoded.is_empty() {
                        bindings.push((2, encoded));
                    }
                }
            }

            // MachineBinding4 = Base64Encode(BIOSIdentifier)
            if let Some(bios_id) = windows::get_bios_identifier() {
                if !bios_id.is_empty() {
                    let encoded = base64_encode(&bios_id);
                    bindings.push((4, encoded));
                }
            }

            // MachineBinding5 = MAC Address
            if let Some(mac) = windows::get_mac_address() {
                if !mac.is_empty() {
                    bindings.push((5, mac));
                }
            }
        }

        #[cfg(not(target_os = "windows"))]
        {
            bindings.push((1, "Non-Windows-Platform".to_string()));
        }

        bindings
    }

    /// 生成 Machine ID = Base64(SHA1(Binding1 + Binding2 + Binding4))
    fn generate_machine_id(&self, bindings: &[(i32, String)]) -> String {
        let binding1 = bindings.iter().find(|(k, _)| *k == 1).map(|(_, v)| v.as_str()).unwrap_or("");
        let binding2 = bindings.iter().find(|(k, _)| *k == 2).map(|(_, v)| v.as_str()).unwrap_or("");
        let binding4 = bindings.iter().find(|(k, _)| *k == 4).map(|(_, v)| v.as_str()).unwrap_or("");

        let part1 = binding1;
        let part2 = format!("{}{}", binding2, binding4);

        let mut hasher = Sha1::new();
        hasher.update(format!("{}{}", part1, part2).as_bytes());
        let result = hasher.finalize();

        general_purpose::STANDARD.encode(result)
    }

    /// 生成 SerialHash = SHA1(machine_id) as hex
    fn generate_serial_hash(&self, machine_id: &str) -> String {
        let mut hasher = Sha1::new();
        hasher.update(machine_id.as_bytes());
        let result = hasher.finalize();
        hex::encode(result)
    }

    /// 生成 DeveloperData = Base64(SerialMasked)
    fn generate_developer_data(&self, _machine_id: &str) -> String {
        let serial = "F4-I0SA-6VDD-WI39-41QP-62OQ";
        general_purpose::STANDARD.encode(serial.as_bytes())
    }

    /// 生成 SerialMasked
    fn generate_serial_masked(&self) -> String {
        "F4-I0SA-6VDD-WI39-41QP-XXXX".to_string()
    }
}

impl Default for AlfGenerator {
    fn default() -> Self {
        Self::new()
    }
}

/// FlipAndCodeBytes 算法
fn flip_and_code_bytes(uncoded: &str) -> String {
    if uncoded.chars().any(|c| c.is_control()) {
        return String::new();
    }

    let text = get_printable_string_from_hex(uncoded)
        .unwrap_or_else(|| uncoded.to_string());

    let text = if text.len() % 2 == 1 {
        &text[..text.len() - 1]
    } else {
        &text
    };

    let mut chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if i + 1 < chars.len() {
            chars.swap(i, i + 1);
        }
        i += 2;
    }

    format_xml_escape(chars.into_iter().collect::<String>().trim())
}

/// 从十六进制字符串获取可打印字符
fn get_printable_string_from_hex(uncoded: &str) -> Option<String> {
    if uncoded.len() % 2 == 1 {
        return None;
    }

    let cleaned: String = uncoded.chars().map(|c| if c.is_whitespace() { '0' } else { c }).collect();

    if !cleaned.chars().all(|c| c.is_ascii_digit() || (c.to_ascii_lowercase() >= 'a' && c.to_ascii_lowercase() <= 'f')) {
        return None;
    }

    let mut result = String::new();
    for i in (0..cleaned.len()).step_by(2) {
        if let Ok(byte) = u8::from_str_radix(&cleaned[i..i+2], 16) {
            if byte >= 0x20 && byte <= 0x7E {
                result.push(byte as char);
            } else {
                return None;
            }
        } else {
            return None;
        }
    }

    Some(result)
}

/// XML 转义
fn format_xml_escape(s: &str) -> String {
    s.replace("&", "&amp;")
        .replace("<", "&lt;")
        .replace(">", "&gt;")
        .replace("'", "&apos;")
        .replace("\"", "&quot;")
}

/// Base64 编码
fn base64_encode(s: &str) -> String {
    general_purpose::STANDARD.encode(s.as_bytes())
}

/// Windows 特定实现
#[cfg(target_os = "windows")]
pub mod windows {
    use super::*;

    /// 获取 Windows Product ID
    pub fn get_windows_product_id() -> Option<String> {
        use winreg::enums::*;
        use winreg::RegKey;

        let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
        if let Ok(key) = hklm.open_subkey("SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion") {
            if let Ok(product_id) = key.get_value::<String, _>("ProductId") {
                return Some(product_id);
            }
        }
        None
    }

    /// 获取 C: 盘所在的物理磁盘序列号
    pub fn get_boot_drive_serial_number() -> Option<String> {
        let ps_command = r#"
            $disk = Get-Partition -DriveLetter C | Get-Disk
            $disk.SerialNumber
        "#;

        if let Ok(output) = std::process::Command::new("powershell")
            .args(&["-NoProfile", "-NonInteractive", "-Command", ps_command])
            .creation_flags(0x08000000)
            .output()
        {
            let serial = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !serial.is_empty() {
                return Some(serial);
            }
        }
        None
    }

    /// 获取 BIOS 序列号
    pub fn get_bios_identifier() -> Option<String> {
        if let Ok(output) = std::process::Command::new("wmic")
            .args(&["bios", "get", "SerialNumber"])
            .creation_flags(0x08000000)
            .output()
        {
            let serial = String::from_utf8_lossy(&output.stdout)
                .lines()
                .nth(1)
                .unwrap_or("")
                .trim()
                .to_string();
            if !serial.is_empty() {
                return Some(serial);
            }
        }
        None
    }

    /// 获取 MAC 地址
    pub fn get_mac_address() -> Option<String> {
        if let Ok(output) = std::process::Command::new("getmac")
            .args(&["/fo", "csv", "/nh"])
            .creation_flags(0x08000000)
            .output()
        {
            let mac = String::from_utf8_lossy(&output.stdout)
                .lines()
                .next()
                .and_then(|line| line.split(',').next())
                .map(|s| s.trim_matches('"').replace('-', ":").to_lowercase())
                .unwrap_or_default();
            if !mac.is_empty() {
                return Some(mac);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_alf() {
        let generator = AlfGenerator::new();
        let content = generator.generate();
        assert!(content.contains("<MachineBindings>"));
        assert!(content.contains("<NoHardwareCheck"));
        assert!(content.contains("<LicenseVersion"));
        assert!(content.contains("<Features>"));
        assert!(!content.contains("<SystemInfo>"));
    }

    #[test]
    fn test_custom_unity_version() {
        let generator = AlfGenerator::new().with_unity_version("6000.3.19f1");
        let content = generator.generate();
        assert!(content.contains("6000.3.19f1"));
    }
}
