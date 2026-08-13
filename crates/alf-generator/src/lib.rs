//! # ALF Generator
//!
//! Unity ALF (Activation License File) generator compatible with Unity Licensing Client.
//!
//! This library generates ALF files that are compatible with Unity's licensing system.
//! Uses v1.0.0 format with NoHardwareCheck and real machine bindings.

use sha1::{Digest, Sha1};
use base64::{Engine as _, engine::general_purpose};

/// ALF 生成器
pub struct AlfGenerator {
    unity_version: String,
    product: String,
}

/// Get features for a given Unity product type
fn features_for_product(product: &str) -> Vec<u32> {
    match product {
        "Unity Pro" => vec![0, 2, 4, 9, 13, 20, 21, 22, 30, 39, 40, 60, 65],
        "Unity Plus" => vec![0, 2, 4, 9, 13, 22, 39, 40, 60],
        "Unity Enterprise" => vec![0, 2, 4, 9, 13, 20, 21, 22, 30, 39, 40, 60, 65, 70],
        "Unity Industrial" => vec![0, 2, 4, 9, 13, 20, 21, 22, 30, 39, 40, 60, 65, 70, 80],
        _ => vec![0, 2, 4, 9, 13, 20, 21, 22, 30, 39, 40, 60, 65],
    }
}

impl AlfGenerator {
    /// 创建新的 ALF 生成器
    pub fn new() -> Self {
        Self {
            unity_version: "2017.2.0".to_string(),
            product: "Unity Pro".to_string(),
        }
    }

    /// 设置 Unity 版本
    pub fn with_unity_version(mut self, version: &str) -> Self {
        self.unity_version = version.to_string();
        self
    }

    /// 设置产品类型
    pub fn with_product(mut self, product: &str) -> Self {
        self.product = product.to_string();
        self
    }

    /// 生成 ALF 内容 (v1.0.0 format with NoHardwareCheck + real machine bindings)
    pub fn generate(&self) -> String {
        let bindings = self.get_machine_bindings();
        let machine_id = self.generate_machine_id(&bindings);
        let serial_hash = self.generate_serial_hash(&machine_id);

        let mut alf_content = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<root>\n    <License id=\"Terms\">\n        <NoHardwareCheck Value=\"true\"/>\n        <MachineBindings>\n");

        for (key, value) in &bindings {
            alf_content.push_str(&format!("            <Binding Key=\"{}\" Value=\"{}\" />\n", key, value));
        }

        alf_content.push_str("        </MachineBindings>\n");
        alf_content.push_str(&format!("        <SerialHash Value=\"{}\" />\n", serial_hash));

        // Features (based on product type)
        alf_content.push_str("        <Features>\n");
        for f in features_for_product(&self.product) {
            alf_content.push_str(&format!("            <Feature Value=\"{}\" />\n", f));
        }
        alf_content.push_str("        </Features>\n");

        let developer_data = self.generate_developer_data(&machine_id);
        alf_content.push_str(&format!("        <DeveloperData Value=\"{}\" />\n", developer_data));

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

    /// 获取 C: 盘所在卷的序列号（用 Win32 API 替代 PowerShell，避免外部进程）
    pub fn get_boot_drive_serial_number() -> Option<String> {
        use ::windows::core::PCWSTR;
        use ::windows::Win32::Storage::FileSystem::GetVolumeInformationW;

        let root: Vec<u16> = "C:\\".encode_utf16().chain(std::iter::once(0)).collect();
        let mut serial = 0u32;
        unsafe {
            let ok = GetVolumeInformationW(
                PCWSTR(root.as_ptr()),
                None,
                Some(&mut serial),
                None,
                None,
                None,
            );
            if ok.is_ok() && serial != 0 {
                return Some(format!("{serial:08X}"));
            }
        }
        None
    }

    /// 获取 BIOS 标识（用注册表替代已废弃的 wmic）
    pub fn get_bios_identifier() -> Option<String> {
        use winreg::enums::*;
        use winreg::RegKey;

        let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
        if let Ok(key) = hklm.open_subkey("HARDWARE\\DESCRIPTION\\System\\BIOS") {
            for value in ["BIOSVersion", "SystemProductName"] {
                if let Ok(v) = key.get_value::<String, _>(value) {
                    if !v.is_empty() {
                        return Some(v);
                    }
                }
            }
        }
        None
    }

    /// 获取 MAC 地址（用 GetAdaptersInfo 替代 getmac 外部命令）
    pub fn get_mac_address() -> Option<String> {
        use ::windows::Win32::NetworkManagement::IpHelper::{GetAdaptersInfo, IP_ADAPTER_INFO};

        unsafe {
            let mut size: u32 = 0;
            GetAdaptersInfo(None, &mut size);
            if size == 0 {
                return None;
            }
            let mut buf = vec![0u8; size as usize];
            let info = buf.as_mut_ptr() as *mut IP_ADAPTER_INFO;
            if GetAdaptersInfo(Some(info), &mut size) != 0 {
                return None;
            }
            let mut cur = info;
            while !cur.is_null() {
                let adapter = &*cur;
                if adapter.AddressLength > 0 {
                    let mac = adapter.Address[..adapter.AddressLength as usize]
                        .iter()
                        .map(|b| format!("{b:02x}"))
                        .collect::<Vec<_>>()
                        .join(":");
                    if !mac.is_empty() {
                        return Some(mac);
                    }
                }
                cur = adapter.Next;
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
