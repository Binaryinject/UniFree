#!/usr/bin/env python3
"""
分析 System.Security.Cryptography.Xml.dll 破解版与原版的差异
不需要额外依赖，直接分析二进制内容
"""

import os
import re
from collections import Counter

def extract_strings(file_path, min_length=4):
    """从二进制文件中提取可读字符串"""
    with open(file_path, 'rb') as f:
        data = f.read()

    # ASCII 字符串 (单字节)
    ascii_pattern = rb'[\x20-\x7E]{' + str(min_length).encode() + rb',}'
    ascii_strings = re.findall(ascii_pattern, data)

    # Unicode 字符串 (UTF-16LE, .NET常用)
    unicode_strings = []
    i = 0
    current_string = []
    while i < len(data) - 1:
        if 0x20 <= data[i] <= 0x7E and data[i+1] == 0:
            current_string.append(chr(data[i]))
            i += 2
        else:
            if len(current_string) >= min_length:
                unicode_strings.append(''.join(current_string))
            current_string = []
            i += 1

    return [s.decode('ascii', errors='ignore') for s in ascii_strings], unicode_strings

def analyze_dll(dll_path):
    """分析DLL内容"""
    print(f"\n{'='*80}")
    print(f"分析: {os.path.basename(dll_path)}")
    print(f"{'='*80}")

    size = os.path.getsize(dll_path)
    print(f"文件大小: {size:,} bytes ({size/1024:.1f} KB)")

    # 提取字符串
    ascii_strings, unicode_strings = extract_strings(dll_path)
    all_strings = set(ascii_strings + unicode_strings)

    print(f"提取到的字符串数量: {len(all_strings)}")

    # 查找关键字符串
    keywords = ['CheckSignature', 'Verify', 'Signature', 'SignedXml',
                'ValidateSignature', 'ComputeSignature', 'GetSignature']

    found_keywords = {}
    for keyword in keywords:
        matches = [s for s in all_strings if keyword.lower() in s.lower()]
        if matches:
            found_keywords[keyword] = matches

    if found_keywords:
        print(f"\n找到的签名验证相关字符串:")
        for keyword, matches in found_keywords.items():
            print(f"\n  [{keyword}] ({len(matches)} 个匹配):")
            for match in matches[:10]:  # 只显示前10个
                if len(match) < 100:  # 只显示不太长的字符串
                    print(f"    - {match}")

    # 查找类型和方法名
    print(f"\n类型和命名空间:")
    namespaces = [s for s in all_strings if '.' in s and
                  any(s.startswith(ns) for ns in ['System.', 'Microsoft.', 'Mono.'])]
    for ns in sorted(set(namespaces))[:20]:
        if len(ns) < 80:
            print(f"  - {ns}")

    return all_strings

def compare_strings(original_strings, cracked_strings):
    """对比两个DLL的字符串差异"""
    print(f"\n{'='*80}")
    print(f"字符串差异分析")
    print(f"{'='*80}")

    only_in_original = original_strings - cracked_strings
    only_in_cracked = cracked_strings - original_strings

    print(f"\n仅在原版中的字符串: {len(only_in_original)} 个")
    if only_in_original:
        relevant = [s for s in only_in_original if any(k in s.lower()
                    for k in ['sign', 'verify', 'check', 'valid', 'crypto'])]
        if relevant:
            print(f"  相关的字符串 (前30个):")
            for s in sorted(relevant)[:30]:
                if 4 < len(s) < 100:
                    print(f"    - {s}")

    print(f"\n仅在破解版中的字符串: {len(only_in_cracked)} 个")
    if only_in_cracked:
        relevant = [s for s in only_in_cracked if any(k in s.lower()
                    for k in ['sign', 'verify', 'check', 'valid', 'crypto'])]
        if relevant:
            print(f"  相关的字符串 (前30个):")
            for s in sorted(relevant)[:30]:
                if 4 < len(s) < 100:
                    print(f"    - {s}")

def main():
    original = r"C:\Program Files\Unity Hub\UnityLicensingClient_V1\System.Security.Cryptography.Xml.dll.bak"
    cracked = r"D:\GIT\UniFree\src-tauri\resources\cracked_CryptoXml.dll"

    if not os.path.exists(original):
        print(f"错误: 找不到原版DLL: {original}")
        return

    if not os.path.exists(cracked):
        print(f"错误: 找不到破解DLL: {cracked}")
        return

    # 文件大小对比
    original_size = os.path.getsize(original)
    cracked_size = os.path.getsize(cracked)

    print(f"\n{'='*80}")
    print(f"System.Security.Cryptography.Xml.dll 破解分析")
    print(f"{'='*80}")
    print(f"\n文件大小对比:")
    print(f"  原版: {original_size:,} bytes ({original_size/1024:.1f} KB)")
    print(f"  破解: {cracked_size:,} bytes ({cracked_size/1024:.1f} KB)")
    print(f"  差异: {original_size - cracked_size:,} bytes")
    print(f"  缩减比例: {100 - (cracked_size/original_size)*100:.1f}%")

    # 分析原版
    print(f"\n\n" + "="*80)
    print("【原版 DLL 分析】")
    original_strings = analyze_dll(original)

    # 分析破解版
    print(f"\n\n" + "="*80)
    print("【破解版 DLL 分析】")
    cracked_strings = analyze_dll(cracked)

    # 对比差异
    compare_strings(original_strings, cracked_strings)

    # 总结
    print(f"\n\n{'='*80}")
    print(f"总结")
    print(f"{'='*80}")
    print(f"""
破解版 DLL 比原版小了 {original_size - cracked_size:,} bytes ({100 - (cracked_size/original_size)*100:.1f}%)

这意味着破解版可能：
1. 移除了实际的签名验证逻辑代码
2. 简化了 CheckSignature 等方法，直接返回 true
3. 删除了不必要的加密算法实现
4. 移除了某些依赖或资源

关键修改推测：
- SignedXml.CheckSignature() -> 总是返回 true
- SignedXml.CheckSignatureReturningKey() -> 总是返回 true
- 其他验证方法 -> 跳过实际验证

这样一来，任何 XML 文件（包括 Unity_lic.ulf）都能通过签名验证！
    """)

if __name__ == "__main__":
    main()
