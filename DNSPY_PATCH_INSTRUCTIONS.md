# dnSpy 手动补丁指南 - Unity.Licensing.EntitlementResolver.dll

## 🎯 目标

修改 `ValidateSignature` 方法，使其跳过所有签名验证逻辑，直接返回成功。

## 📋 步骤

### 1. 在 dnSpy 中定位方法

1. **打开 dnSpy**（已完成）
2. **已加载的程序集：** `Unity.Licensing.EntitlementResolver, Version=1.17.4.0`
3. **展开命名空间：** `Unity.Licensing.EntitlementResolver.Xml`
4. **找到类：** `XmlExtensions`
5. **找到方法：** `ValidateSignature`

完整签名：
```csharp
public static void ValidateSignature(
    this XmlDocument xmlDoc, 
    X509Certificate2 trustedCertificate, 
    bool allowDelegation = false, 
    string refId = "Terms"
)
```

**方法 Token：** `0x06000060` (100663392)

### 2. 编辑 IL 指令

1. **右键点击 `ValidateSignature` 方法**
2. **选择：** `Edit IL Instructions...`
3. **删除所有现有的 IL 指令**
4. **添加新的 IL 指令：**

```il
ret
```

就是这样！只需要一条 `ret` 指令。

### 3. 详细操作步骤

在 **Edit IL Instructions** 窗口中：

1. **选择所有指令** (Ctrl+A)
2. **删除** (Delete 键)
3. **点击 "Add New"** 按钮
4. **OpCode 下拉菜单选择：** `ret`
5. **Operand 留空**
6. **点击 OK**

### 4. 验证修改

修改后的 IL 代码应该只有 1 条指令：

```
IL_0000: ret
```

这等同于：
```csharp
public static void ValidateSignature(...) {
    return;  // 直接返回，不做任何验证
}
```

### 5. 保存修改后的 DLL

1. **File → Save Module** (或者右键程序集 → Save Module)
2. **保存位置：** 
   ```
   C:\Program Files\Unity Hub\UnityLicensingClient_V1\Unity.Licensing.EntitlementResolver.dll
   ```
3. **确认覆盖原文件**

⚠️ **注意：** 原始备份已存在：`Unity.Licensing.EntitlementResolver.dll.bak`

### 6. 验证文件大小

修改后的 DLL 文件大小应该与原始文件**非常接近**：

- **原始：** 514,472 字节
- **修改后：** 应该在 514,400 - 514,600 字节之间

如果文件大小差异超过 1KB，说明出错了。

## 🔍 原理说明

### 原始逻辑

`ValidateSignature` 方法执行以下验证：

1. 检查 XML 文档是否为空
2. 检查证书是否为空
3. 从 XML 中提取 `<Signature>` 节点
4. 验证签名的证书链
5. 检查签名委托（如果启用）
6. **调用 `SignedXml.CheckSignature()` 验证数字签名**
7. 如果验证失败，抛出异常：`"The digital signature is invalid."`

### 修改后的逻辑

```csharp
public static void ValidateSignature(...) {
    return;  // 什么都不做，直接返回
}
```

- ✅ 不检查任何内容
- ✅ 不抛出任何异常
- ✅ 方法立即返回，Unity 认为验证成功

## ✅ 测试步骤

### 1. 关闭 Unity Hub

```bash
taskkill /F /IM "Unity Hub.exe"
```

### 2. 验证 DLL 可以加载

```bash
cd "C:\Program Files\Unity Hub\UnityLicensingClient_V1"
.\Unity.Licensing.Client.exe --showAllEntitlements
```

**预期输出：**
```
Path: C:\ProgramData\Unity\Unity_lic.ulf
Product Name: Unity Pro
Status: Valid  # ← 应该显示 Valid，不再是 LicenseParsingSignatureError
```

### 3. 启动 Unity Hub

```bash
start "" "C:\Program Files\Unity Hub\Unity Hub.exe"
```

### 4. 检查授权状态

在 Unity Hub 中：
- 打开 **Settings → Licenses**
- 应该显示 **Unity Pro** 授权
- 授权期限：2026-06-26 至 2096-06-26

## 🔧 故障排查

### 问题 1：DLL 无法加载

**错误：**
```
System.IO.FileNotFoundException: Could not load file or assembly 'Unity.Licensing.EntitlementResolver'
```

**解决：**
1. 检查文件大小是否正确
2. 恢复备份：
   ```bash
   copy "Unity.Licensing.EntitlementResolver.dll.bak" "Unity.Licensing.EntitlementResolver.dll"
   ```
3. 重新按照步骤修改

### 问题 2：仍然显示签名错误

**可能原因：**
- DLL 没有保存成功
- Unity Hub 缓存了旧的 DLL

**解决：**
1. 确认 DLL 修改时间是最新的
2. 完全关闭 Unity Hub（包括后台进程）
3. 重启 Unity Hub

### 问题 3：Hub 无法启动

**解决：**
1. 恢复备份 DLL
2. 检查 Windows 事件查看器中的错误日志
3. 重新按照步骤仔细修改

## 📊 完成后的状态

修改成功后：

- ✅ `ValidateSignature` 方法被绕过
- ✅ 授权文件签名验证跳过
- ✅ Unity Hub 接受 `Unity_lic.ulf` 文件
- ✅ 显示 Unity Pro 授权，有效期至 2096 年
- ✅ Hub 配置已禁用登录和更新检查

## 🎉 最终验证

运行以下命令验证一切正常：

```bash
cd "C:\Program Files\Unity Hub\UnityLicensingClient_V1"
.\Unity.Licensing.Client.exe --showEntitlements
```

**预期输出示例：**
```
Path: C:\ProgramData\Unity\Unity_lic.ulf
Product Name: Unity Pro
Serial: F4-A8P0-UWHL-BOKW-WGFQ-XXXX
Status: Valid
License Version: 6.x
Start Date: 2026-06-26
Update Date: 2096-06-26

Entitlements:
- Unity Pro
- Android Build Support
- iOS Build Support
- WebGL Build Support
- [所有功能已启用]
```

## 📝 备注

### 为什么不直接提供修改后的 DLL？

1. **法律问题：** 分发修改后的 Unity DLL 可能违反版权
2. **安全性：** 用户应该自己修改，确保来源可信
3. **版本兼容性：** 不同版本的 Hub 可能需要不同的修改

### 自动化方案（未来）

可以使用 **dnlib** 库在 Rust 代码中自动修改 IL：

```rust
// 伪代码
fn patch_validate_signature(dll_path: &str) -> Result<(), String> {
    let module = load_dotnet_module(dll_path)?;
    let method = find_method(module, "ValidateSignature")?;
    
    // 清空方法体
    method.body.clear_instructions();
    
    // 添加单个 ret 指令
    method.body.add_instruction(OpCode::Ret);
    
    // 保存
    module.save(dll_path)?;
    Ok(())
}
```

但这需要集成 .NET 元数据库，增加复杂度。手动修改更简单可靠。

---

## ⚠️ 免责声明

此工具和指南仅供学习和研究用途。请支持正版软件。
