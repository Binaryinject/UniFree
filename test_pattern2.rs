use std::fs;

fn main() {
    let dll_path = r"C:\Program Files\Unity Hub\UnityLicensingClient_V1\Unity.Licensing.EntitlementResolver.dll";

    println!("Testing IL Patcher on: {}", dll_path);

    let data = match fs::read(dll_path) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Failed to read DLL: {}", e);
            return;
        }
    };

    println!("DLL size: {} bytes\n", data.len());

    // 尝试多种编码搜索
    let searches = vec![
        ("UTF-8", b"The digital signature is invalid.".to_vec()),
        ("UTF-16 LE", "The digital signature is invalid.".encode_utf16()
            .flat_map(|c| c.to_le_bytes()).collect()),
        ("Short", b"digital signature".to_vec()),
        ("Short UTF-16", "digital signature".encode_utf16()
            .flat_map(|c| c.to_le_bytes()).collect()),
    ];

    for (name, pattern) in &searches {
        let mut count = 0;
        for (i, window) in data.windows(pattern.len()).enumerate() {
            if window == pattern.as_slice() {
                println!("Found '{}' at offset {:#x}", name, i);
                count += 1;
                if count >= 5 {
                    break;
                }
            }
        }
        if count == 0 {
            println!("Not found: {}", name);
        }
        println!();
    }

    // 搜索 IL 模式：callvirt + brtrue.s
    println!("Searching for IL pattern: callvirt (0x6F) + brtrue.s (0x2D)...");
    let mut found_count = 0;
    for i in 0..data.len().saturating_sub(7) {
        if data[i] == 0x6F && data[i + 5] == 0x2D {
            found_count += 1;
            if found_count <= 10 {
                println!("Pattern at {:#x}: callvirt...brtrue.s (byte {:#x})", i, data[i + 5]);
            }
        }
    }
    println!("Total patterns found: {}", found_count);
}
