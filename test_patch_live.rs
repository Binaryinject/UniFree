use std::fs;

fn main() {
    let dll_path = r"C:\Program Files\Unity Hub\UnityLicensingClient_V1\Unity.Licensing.EntitlementResolver.dll";

    println!("Reading DLL...");
    let mut data = fs::read(dll_path).expect("Failed to read DLL");

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

    println!("Found {} error strings", error_positions.len());

    let mut patched = 0;
    for &error_pos in &error_positions {
        let search_start = error_pos.saturating_sub(100);
        let search_end = error_pos;

        for i in (search_start..search_end).rev() {
            if i + 12 >= data.len() {
                continue;
            }

            if data[i] == 0x6F && data[i + 5] == 0x2D && data[i + 7] == 0x72 {
                println!("\n✓ Found target at {:#x}:", i);
                println!("  Current: callvirt...{:#x}(brtrue.s)...ldstr", data[i + 5]);
                println!("  Will change to: callvirt...0x2B(br.s)...ldstr");

                // 这里只是测试，不实际修改
                // data[i + 5] = 0x2B;
                patched += 1;
                break;
            }
        }
    }

    println!("\n✅ Would patch {} locations", patched);
    println!("\nTo apply: Run UniFree and click 'Patch Hub'");
}
