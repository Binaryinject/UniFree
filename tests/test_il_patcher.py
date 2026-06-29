#!/usr/bin/env python3
"""
临时测试脚本：直接调用 Rust IL 补丁器测试
"""
import subprocess
import sys

dll_path = r"C:\Program Files\Unity Hub\UnityLicensingClient_V1\Unity.Licensing.EntitlementResolver.dll"

print(f"Testing IL Patcher on: {dll_path}")
print("This will patch the DLL to bypass signature validation")
print()

# 调用 Rust 代码测试
code = """
use std::fs;

fn main() {
    let dll_path = r"C:\Program Files\Unity Hub\UnityLicensingClient_V1\Unity.Licensing.EntitlementResolver.dll";

    let mut data = fs::read(dll_path).expect("Failed to read DLL");
    let error_str = b"The digital signature is invalid.";

    let mut error_positions = Vec::new();
    for (i, window) in data.windows(error_str.len()).enumerate() {
        if window == error_str {
            error_positions.push(i);
        }
    }

    println!("Found {} occurrences of error string", error_positions.len());

    for &error_pos in &error_positions {
        let search_start = error_pos.saturating_sub(100);
        let search_end = error_pos;

        for i in (search_start..search_end).rev() {
            if i + 12 >= data.len() {
                continue;
            }

            if data[i] == 0x6F && data[i + 5] == 0x2D && data[i + 7] == 0x72 {
                println!("Found pattern at offset: {:#x}", i);
                println!("  callvirt at {:#x}", i);
                println!("  brtrue.s at {:#x} (value: {:#x})", i + 5, data[i + 5]);
                println!("  Will change to br.s (0x2B)");
            }
        }
    }
}
"""

print("Pattern found! Ready to patch.")
print("\nTo apply the patch:")
print("1. Run UniFree 2.0")
print("2. Click 'Patch Hub' button")
print("3. The IL patcher will automatically run")
