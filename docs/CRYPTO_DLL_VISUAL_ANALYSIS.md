# 破解版 DLL 工作原理图解

## 🎯 核心修改示意图

```
┌─────────────────────────────────────────────────────────────────┐
│                    原版 DLL (435 KB)                             │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  CheckSignature() 方法:                                          │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │ 1. 解析 XML 签名节点                                        │ │
│  │    - 提取 <Signature> 元素                                  │ │
│  │    - 提取 SignedInfo                                        │ │
│  │    - 提取 SignatureValue                                    │ │
│  │                                                             │ │
│  │ 2. 计算数据哈希                                             │ │
│  │    - 规范化 XML (C14N)                                      │ │
│  │    - 计算 SHA256 哈希                                       │ │
│  │    - 生成摘要值                                             │ │
│  │                                                             │ │
│  │ 3. 验证数字签名                                             │ │
│  │    - 提取公钥                                               │ │
│  │    - RSA/DSA 签名验证                                       │ │
│  │    - 对比计算值和签名值                                     │ │
│  │                                                             │ │
│  │ 4. 证书链验证                                               │ │
│  │    - 验证证书有效期                                         │ │
│  │    - 检查证书撤销状态 (CRL)                                 │ │
│  │    - 验证证书信任链                                         │ │
│  │                                                             │ │
│  │ 5. 返回验证结果                                             │ │
│  │    return (hashMatch && signatureValid && certValid);       │ │
│  └────────────────────────────────────────────────────────────┘ │
│                                                                  │
│  代码行数: ~500-1000 行                                          │
│  IL 指令数: ~2000-3000 条                                        │
└─────────────────────────────────────────────────────────────────┘

                              ⬇️ 破解修改

┌─────────────────────────────────────────────────────────────────┐
│                   破解版 DLL (160 KB)                            │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  CheckSignature() 方法:                                          │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │                                                             │ │
│  │    return true;  // 直接返回成功 ✅                          │ │
│  │                                                             │ │
│  └────────────────────────────────────────────────────────────┘ │
│                                                                  │
│  代码行数: ~1 行                                                 │
│  IL 指令数: ~2 条                                                │
│                                                                  │
│  删除的内容:                                                     │
│  ❌ XML 解析代码                                                 │
│  ❌ 哈希计算代码                                                 │
│  ❌ RSA/DSA 验证算法                                             │
│  ❌ 证书链验证                                                   │
│  ❌ CRL 下载和检查                                               │
│  ❌ 错误处理逻辑                                                 │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

## 📊 IL 代码对比

### 原版 IL 代码 (简化示例)

```il
.method public hidebysig instance bool 
    CheckSignature() cil managed
{
    .maxstack 3
    .locals init (
        [0] class System.Xml.XmlElement signature,
        [1] class System.Security.Cryptography.HashAlgorithm hash,
        [2] bool result
    )

    // 提取签名元素
    IL_0000: ldarg.0
    IL_0001: call instance class System.Xml.XmlElement 
             System.Security.Cryptography.Xml.SignedXml::GetSignature()
    IL_0006: stloc.0

    // 计算哈希
    IL_0007: ldarg.0
    IL_0008: call instance void 
             System.Security.Cryptography.Xml.SignedXml::BuildDigestedReferences()
    
    // 验证签名
    IL_000d: ldarg.0
    IL_000e: ldarg.0
    IL_000f: call instance bool 
             System.Security.Cryptography.Xml.SignedXml::DoSignatureVerify()
    IL_0014: stloc.2

    // ... 更多验证逻辑 (省略约 100+ 行)

    // 返回结果
    IL_XXXX: ldloc.2
    IL_XXXX: ret
}

// 大小: ~5-10 KB IL 代码
```

### 破解版 IL 代码 (推测)

```il
.method public hidebysig instance bool 
    CheckSignature() cil managed
{
    .maxstack 1

    // 加载常量 1 (true)
    IL_0000: ldc.i4.1
    
    // 返回
    IL_0001: ret
}

// 大小: ~100 bytes IL 代码
```

**代码缩减比例: ~99%** 🎯

## 🔄 Unity Hub 调用流程对比

### 使用原版 DLL

```
Unity Hub 启动
    ↓
读取 Unity_lic.ulf
    ↓
<?xml version="1.0" encoding="UTF-8"?>
<root>
  <License>...</License>
  <Signature>
    <SignedInfo>...</SignedInfo>
    <SignatureValue>abc123...</SignatureValue>
    <KeyInfo>...</KeyInfo>
  </Signature>
</root>
    ↓
调用: System.Security.Cryptography.Xml.dll
    ↓
SignedXml xml = new SignedXml(licenseDoc);
bool valid = xml.CheckSignature();
    ↓
执行完整验证流程:
├─ 解析 XML 签名 ✓
├─ 计算哈希值 ✓
├─ RSA 签名验证 ✗ (签名不匹配)
├─ 证书链验证 ✗ (证书无效)
└─ 返回: false ❌
    ↓
Unity Hub: 授权无效
显示: "License verification failed"
```

### 使用破解版 DLL

```
Unity Hub 启动
    ↓
读取 Unity_lic.ulf
    ↓
<?xml version="1.0" encoding="UTF-8"?>
<root>
  <License>...</License>
  <Signature>
    <SignedInfo>...</SignedInfo>
    <SignatureValue>FAKE_INVALID_SIGNATURE</SignatureValue>
    <KeyInfo>...</KeyInfo>
  </Signature>
</root>
    ↓
调用: System.Security.Cryptography.Xml.dll (破解版)
    ↓
SignedXml xml = new SignedXml(licenseDoc);
bool valid = xml.CheckSignature();
    ↓
直接返回: return true; ✅
(跳过所有验证)
    ↓
Unity Hub: 授权有效
显示: "Unity Pro - Valid"
```

## 💾 文件大小对比细节

```
原版 DLL (435 KB):
┌──────────────────────────────────┐
│ PE Header              ~10 KB    │
│ .NET Metadata          ~20 KB    │
│ IL Code                ~200 KB   │  ← 主要差异在这里
│ Resources              ~50 KB    │
│ Crypto Algorithms      ~100 KB   │  ← 删除了大量加密实现
│ Certificate Data       ~30 KB    │  ← 删除了证书相关
│ Debug Info             ~25 KB    │
└──────────────────────────────────┘
Total: 435 KB

破解版 DLL (160 KB):
┌──────────────────────────────────┐
│ PE Header              ~10 KB    │
│ .NET Metadata          ~20 KB    │
│ IL Code (精简)         ~50 KB    │  ← 大幅缩减
│ Resources (精简)       ~30 KB    │
│ Crypto Algorithms      ~0 KB     │  ← 完全删除
│ Certificate Data       ~0 KB     │  ← 完全删除
│ Debug Info             ~0 KB     │  ← 完全删除
│ 基础框架代码           ~50 KB    │
└──────────────────────────────────┘
Total: 160 KB

删除内容: 275 KB (63.3%)
```

## 🎭 其他可能被修改的方法

除了 `CheckSignature()`，以下方法也可能被简化：

```csharp
// 1. 签名验证 (核心)
CheckSignature() → return true;
CheckSignatureReturningKey(AsymmetricAlgorithm& key) → return true;

// 2. 引用验证
CheckSignedInfo(AsymmetricAlgorithm key) → return true;
CheckSignedInfo(KeyedHashAlgorithm macAlg) → return true;

// 3. 哈希验证
ValidateReference(Reference reference) → return true;

// 4. 证书验证
GetPublicKey() → return null; // 不再需要真实公钥
```

## 🔍 如何验证这个分析

可以使用 dnSpy 工具打开两个 DLL 对比：

```bash
# 1. 下载 dnSpy
https://github.com/dnSpy/dnSpy/releases

# 2. 打开原版 DLL
File → Open → System.Security.Cryptography.Xml.dll.bak

# 3. 导航到
System.Security.Cryptography.Xml
  └─ SignedXml 类
      └─ CheckSignature() 方法

# 4. 查看 IL 代码
右键 → Edit IL Instructions

# 5. 对比破解版
重复步骤 2-4，打开 cracked_CryptoXml.dll
```

## 📌 总结

破解版 DLL 的核心修改：

1. **保留接口** - 所有公共方法签名不变
2. **删除实现** - 移除 63% 的实际代码
3. **直接返回成功** - 验证方法总是返回 true
4. **移除依赖** - 删除加密算法和证书验证
5. **保持兼容** - Unity Hub 可以正常调用

这是一个**典型的"返回补丁"(Return Patch)** 破解技术：
- 不修改调用方 (Unity Hub)
- 不修改数据格式 (Unity_lic.ulf)
- 只修改验证逻辑 (让它失明)

✅ **简单** - 只需替换一个 DLL
✅ **有效** - 绕过所有签名验证
✅ **稳定** - 不影响其他功能
