use std::fs;

fn main() {
    let dll_path = r"C:\Program Files\Unity Hub\UnityLicensingClient_V1\Unity.Licensing.EntitlementResolver.dll";

    println!("Reading DLL...");
    let data = fs::read(dll_path).expect("Failed to read DLL");

    // UTF-16 LE 编码
    let error_str: Vec<u8> = "The digital signature is invalid."
        .encode_utf16()
        .flat_map(|c| c.to_le_bytes())
        .collect();

    let mut error_positions = Vec::new();
    for (i, window) in data.windows(error_str.len()).enumerate() {
        if window == error_str.as_slice() {
            error_positions.push(i);
        }
    }

    println!("Found {} error strings at:", error_positions.len());
    for &pos in &error_positions {
        println!("  - {:#x}", pos);
    }

    let mut patched = 0;
    for &error_pos in &error_positions {
        println!("\nSearching backwards from {:#x}:", error_pos);

        let search_start = error_pos.saturating_sub(200);
        let search_end = error_pos;

        let mut found = false;
        for i in (search_start..search_end).rev() {
            if i + 12 >= data.len() {
                continue;
            }

            // 检查 callvirt (0x6F) 指令
            if data[i] == 0x6F {
                // 检查后面是否有 brtrue.s (0x2D) 或 br.s (0x2B)
                if i + 5 < data.len() {
                    let opcode = data[i + 5];
                    if opcode == 0x2D || opcode == 0x2B {
                        println!("  Found callvirt at {:#x}, opcode at +5: {:#x} ({})",
                            i, opcode,
                            if opcode == 0x2D { "brtrue.s - NEEDS PATCH" }
                            else { "br.s - ALREADY PATCHED" });

                        // 检查是否后面还有 ldstr
                        if i + 7 < data.len() && data[i + 7] == 0x72 {
                            println!("  ✓ Complete pattern found with ldstr at +7");
                            if opcode == 0x2D {
                                patched += 1;
                            }
                            found = true;
                            break;
                        }
                    }
                }
            }
        }

        if !found {
            println!("  ✗ No matching pattern found");
        }
    }

    println!("\n✅ Would patch {} locations", patched);
}
