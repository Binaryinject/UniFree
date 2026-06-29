use std::fs;

fn main() {
    let dll_path = r"C:\Program Files\Unity Hub\UnityLicensingClient_V1\Unity.Licensing.EntitlementResolver.dll";

    println!("Reading DLL...");
    let mut data = fs::read(dll_path).expect("Failed to read DLL");

    // 根据之前 dnSpy MCP 的结果，我们知道：
    // Index 120 在 IL offset 349 (0x15D)
    // Index 138 在 IL offset 403 (0x193)
    // 但这些是相对于方法开始的偏移

    // 我们需要找到 ValidateSignature 方法的实际文件偏移
    // 方法 Token: 0x06000060 (100663392)

    // 从之前的分析，ValidateSignature 方法在 RVA 0x0004BA80
    // 我们需要将 RVA 转换为文件偏移

    // 简化方案：搜索所有 callvirt + brtrue.s 模式，然后手动检查
    println!("\nSearching all callvirt (0x6F) + brtrue.s (0x2D) patterns:");

    let mut candidates = Vec::new();
    for i in 0..data.len().saturating_sub(12) {
        if data[i] == 0x6F && data[i + 5] == 0x2D {
            // 检查后面几个字节看起来像不像我们要找的
            candidates.push(i);
        }
    }

    println!("Found {} potential candidates", candidates.len());

    // 查找与 CheckSignature 相关的 - 应该在后面跟着错误消息
    println!("\nLooking for candidates near 'invalid' strings:");

    let invalid_str = b"invalid";
    for &offset in &candidates {
        // 在后面 100 字节内查找 "invalid"
        let search_end = (offset + 150).min(data.len());
        for j in offset..search_end {
            if j + invalid_str.len() < data.len() {
                if &data[j..j+invalid_str.len()] == invalid_str {
                    println!("\n✓ Candidate at {:#x}:", offset);
                    println!("  Bytes: {:02x} {:02x} {:02x} {:02x} {:02x} | {:02x} {:02x} | {:02x}...",
                        data[offset], data[offset+1], data[offset+2], data[offset+3], data[offset+4],
                        data[offset+5], data[offset+6], data[offset+7]);
                    println!("  'invalid' found {} bytes later", j - offset);

                    // 检查实际内容
                    if offset + 5 < data.len() && data[offset + 5] == 0x2D {
                        println!("  -> Would change byte at {:#x} from 0x2D to 0x2B", offset + 5);
                    }
                    break;
                }
            }
        }
    }
}
