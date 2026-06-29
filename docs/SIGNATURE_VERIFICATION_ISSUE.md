# Unity Hub 签名验证问题分析

## 🔍 问题诊断

### 当前状态
- ✅ Hub 配置已正确补丁（DisableSignInRequired, DisableAutoUpdate）
- ✅ 授权文件已复制（Unity_lic.ulf）
- ❌ **签名验证失败：** `The digital signature in the license is invalid`

### 错误原因
```
Path: C:\ProgramData\Unity\Unity_lic.ulf
Product Name: Unknown
Status: LicenseParsingSignatureError
    validation error: The digital signature in the license is invalid
EntitlementGroupId: InvalidEntitlementGroup-779237F6
```

## ⚠️ 之前的错误方法

### 尝试 1：替换 DLL 中的 PEM 证书（失败）
**问题：**
- 原始证书块：1,797 字节
- 新证书块：~3,000+ 字节
- 文件大小变化：514,472 → 516,818 字节（+2,346 字节）
- **结果：** .NET 程序集加载失败
  ```
  System.IO.FileNotFoundException: Could not load file or assembly 
  'Unity.Licensing.EntitlementResolver, Version=1.17.4.0'
  ```

**为什么失败：**
- .NET 程序集有严格的元数据和哈希校验
- 改变文件大小会破坏程序集的结构
- CLR 无法加载被修改的程序集

## ✅ 正确的解决方案

### 方案 1：修改 IL 代码跳过签名验证（推荐）

使用 dnSpy 或 dnlib 修改 `Unity.Licensing.EntitlementResolver.dll`：

1. **找到签名验证方法：**
   - 可能的类名：`SignatureValidator`, `LicenseValidator`, `CertificateValidator`
   - 搜索字符串：`"digital signature"`, `"signature"`, `"invalid"`

2. **修改验证逻辑：**
   ```csharp
   // 原始代码：
   public bool ValidateSignature(XmlDocument license) {
       // 复杂的签名验证逻辑
       return VerifyXmlSignature(license);
   }
   
   // 修改后：
   public bool ValidateSignature(XmlDocument license) {
       return true;  // 直接返回 true，跳过验证
   }
   ```

3. **保存修改后的 DLL**（文件大小可能略有变化，但程序集结构保持完整）

### 方案 2：使用 Harmony 运行时 Hook

创建 BepInEx 插件，在运行时 Hook 签名验证方法：

```csharp
[HarmonyPatch(typeof(SignatureValidator), nameof(SignatureValidator.Validate))]
class SignatureValidatorPatch {
    static bool Prefix(ref bool __result) {
        __result = true;  // 强制返回验证通过
        return false;     // 跳过原方法
    }
}
```

### 方案 3：生成自签名证书并安装到系统（复杂）

1. 生成自签名 CA 证书
2. 用该 CA 签名授权文件
3. 将 CA 证书安装到 Windows 受信任的根证书颁发机构
4. 替换 DLL 中的证书指纹验证

**缺点：** 需要修改系统证书存储，风险较高

## 🛠️ 推荐实现步骤

### 使用 dnSpy 修改 DLL

1. **在 dnSpy 中打开 DLL：**
   ```
   C:\Program Files\Unity Hub\UnityLicensingClient_V1\Unity.Licensing.EntitlementResolver.dll
   ```

2. **搜索签名验证相关代码：**
   - 搜索字符串：`"signature"`, `"invalid"`, `"validation"`
   - 查找方法：`Validate`, `Verify`, `Check`

3. **修改 IL 代码：**
   - 右键方法 → Edit IL Instructions
   - 将方法体改为直接返回 `true`：
     ```il
     ldc.i4.1    // 加载整数 1 (true)
     ret         // 返回
     ```

4. **保存修改后的 DLL：**
   - File → Save Module
   - 文件大小应该与原始版本接近（差异 < 100 字节）

### 集成到 UniFree

修改 `patcher.rs` 中的 `patch_hub()` 函数：

```rust
pub fn patch_hub(_resources_path: &str, disable_signin: bool, disable_update: bool) -> Result<String, String> {
    // ... 现有代码 ...
    
    // 不再修补 DLL 证书，而是：
    // 1. 提示用户使用 dnSpy 手动修改 DLL
    // 2. 或者集成 dnlib 库自动修改 IL 代码
    // 3. 或者提供预修改的 DLL 文件
    
    // 只修改配置和复制授权文件
    patch_asar_config()?;
    copy_license()?;
    
    Ok("Patched: Hub config (DLL needs manual patching)".into())
}
```

## 📋 手动修补步骤（临时方案）

1. **备份原始 DLL：**
   ```bash
   copy "C:\Program Files\Unity Hub\UnityLicensingClient_V1\Unity.Licensing.EntitlementResolver.dll" Unity.Licensing.EntitlementResolver.dll.bak
   ```

2. **用 dnSpy 打开 DLL**

3. **找到签名验证方法并修改为返回 true**

4. **保存并测试**

5. **运行 UniFree 完成其他补丁（配置+授权文件）**

## 🔍 需要分析的内容

使用 dnSpy MCP 查找：

1. **签名验证入口点：**
   ```csharp
   // 搜索这些命名空间/类
   Unity.Licensing.*
   *.SignatureValidator
   *.LicenseValidator
   ```

2. **关键方法：**
   - 包含 "Signature" 的方法
   - 包含 "Validate" 的方法
   - 返回类型为 bool 的验证方法

3. **错误消息定位：**
   搜索字符串：`"digital signature in the license is invalid"`
   这会直接定位到验证失败的代码位置

## 🚧 当前状态

- [x] Hub 配置补丁工作正常
- [x] 授权文件复制工作正常
- [ ] **DLL 签名验证绕过 - 需要实现**
- [ ] 集成自动化的 IL 修改工具

## 📝 下一步

1. 在 dnSpy 中加载 `Unity.Licensing.EntitlementResolver.dll`
2. 使用 dnSpy MCP 分析签名验证逻辑
3. 找到验证方法并记录其签名
4. 决定使用哪种方案（IL 修改 vs Harmony Hook）
5. 实现自动化补丁
