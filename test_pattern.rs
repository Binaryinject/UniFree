use std::fs;

fn main() {
    let dll_path = r"C:\Program Files\Unity Hub\UnityLicensingClient_V1\Unity.Licensing.EntitlementResolver.dll";

    println!("Testing IL Patcher on: {}", dll_path);

    // 读取 DLL
    let data = match fs::read(dll_path) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Failed to read DLL: {}", e);
            return;
        }
    };

    println!("DLL size: {} bytes", data.len());

    // 搜索错误字符串
    let error_str = b"The digital signature is invalid.";
    let mut error_positions = Vec::new();

    for (i, window) in data.windows(error_str.len()).enumerate() {
        if window == error_str {
            error_positions.push(i);
        }
    }

    println!("Found {} occurrences of error string", error_positions.len());

    for (idx, &error_pos) in error_positions.iter().enumerate() {
        println!("\n=== Occurrence {} at offset {:#x} ===", idx + 1, error_pos);

        let search_start = error_pos.saturating_sub(100);
        let search_end = error_pos;

        for i in (search_start..search_end).rev() {
            if i + 12 >= data.len() {
                continue;
            }

            if data[i] == 0x6F && data[i + 5] == 0x2D && data[i + 7] == 0x72 {
                println!("✓ Found pattern at offset {:#x}:", i);
                println!("  callvirt: {:#x} {:#x} {:#x} {:#x} {:#x}",
                    data[i], data[i+1], data[i+2], data[i+3], data[i+4]);
                println!("  Current opcode: {:#x} ({})",
                    data[i + 5],
                    if data[i + 5] == 0x2D { "brtrue.s" } else if data[i + 5] == 0x2B { "br.s (already patched)" } else { "unknown" }
                );
                println!("  ldstr: {:#x} {:#x} {:#x} {:#x} {:#x}",
                    data[i+7], data[i+8], data[i+9], data[i+10], data[i+11]);
                break;
            }
        }
    }
}
