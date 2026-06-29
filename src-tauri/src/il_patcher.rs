use std::fs;
use std::path::Path;

/// 自动修补 Unity.Licensing.EntitlementResolver.dll
///
/// 修改 ValidateSignature 方法的 IL 代码，绕过签名验证
pub fn patch_signature_validation(dll_path: &str) -> Result<String, String> {
    let path = Path::new(dll_path);
    if !path.exists() {
        return Err("DLL not found".into());
    }

    // 读取 DLL
    let mut data = fs::read(path).map_err(|e| e.to_string())?;

    // 创建备份
    let bak_path = format!("{}.bak", dll_path);
    if !Path::new(&bak_path).exists() {
        fs::copy(path, &bak_path).map_err(|e| e.to_string())?;
    }

    // 应用二进制补丁
    let patches_applied = apply_binary_patches(&mut data)?;

    // 保存修改后的 DLL
    fs::write(path, &data).map_err(|e| e.to_string())?;

    Ok(format!("Patched: {} signature checks bypassed", patches_applied))
}

/// 查找签名验证的字节码模式
fn find_signature_check_pattern(data: &[u8], occurrence: usize) -> Result<Option<usize>, String> {
    // "The digital signature is invalid." 的 UTF-16 字符串
    let signature_error = b"The digital signature is invalid.";

    let mut found = 0;
    for (i, window) in data.windows(signature_error.len()).enumerate() {
        if window == signature_error {
            found += 1;
            if found == occurrence {
                // 向前搜索 callvirt 指令 (0x6F)
                for j in (i.saturating_sub(100)..i).rev() {
                    if data[j] == 0x6F {
                        return Ok(Some(j));
                    }
                }
            }
        }
    }
    Ok(None)
}

/// 应用二进制补丁（直接替换字节序列）
fn apply_binary_patches(data: &mut Vec<u8>) -> Result<usize, String> {
    let mut count = 0;

    // 错误字符串的 UTF-16 LE 编码
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

    if error_positions.len() != 2 {
        return Err(format!(
            "Expected 2 occurrences of error string, found {}. DLL version might not be supported.",
            error_positions.len()
        ));
    }

    // 对每个错误字符串位置，向前搜索并修补
    for &error_pos in &error_positions {
        // 在错误字符串前面应该有这个模式：
        // callvirt CheckSignature (0x6F XX XX XX XX, 5 bytes)
        // brtrue.s (0x2D XX, 2 bytes)
        // ldstr (0x72 XX XX XX XX, 5 bytes)
        // 我们需要将 brtrue.s 改为 br.s (0x2B)

        let search_start = error_pos.saturating_sub(100);
        let search_end = error_pos;

        // 查找 callvirt (0x6F) 指令
        for i in (search_start..search_end).rev() {
            if i + 12 >= data.len() {
                continue;
            }

            // 检查模式: callvirt (5 bytes) + brtrue.s (2 bytes) + ldstr (5 bytes)
            if data[i] == 0x6F  // callvirt
                && data[i + 5] == 0x2D  // brtrue.s
                && data[i + 7] == 0x72  // ldstr
            {
                // 找到了！将 brtrue.s 改为 br.s
                data[i + 5] = 0x2B;
                count += 1;
                break;
            }
        }
    }

    if count != 2 {
        return Err(format!(
            "Expected to patch 2 locations, but only patched {}. Manual patching required.",
            count
        ));
    }

    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_patch_signature() {
        // 这里需要实际的测试 DLL
        // let result = patch_signature_validation("test.dll");
        // assert!(result.is_ok());
    }
}
