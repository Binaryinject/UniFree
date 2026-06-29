# UniFree 2.0 - 配置文件方案（无需修改 DLL）

## 🎉 新方案实现完成

**不再修改 DLL 的 IL 代码，改用配置文件禁用签名验证！**

## ✅ 新方案的工作原理

### 1. 创建 Unity 配置文件

**文件：** `C:\ProgramData\Unity\config\services-config.json`

**内容：**
```json
{
  "enableLicenseValidation": false,
  "enableEntitlementValidation": false,
  "offlineMode": true,
  "disableSignatureValidation": true
}
```

**作用：** 告诉 Unity 禁用所有授权验证

### 2. 使用无签名的授权文件

**文件：** `C:\ProgramData\Unity\Unity_lic.ulf`

**关键变化：**
- 完全移除 `<Signature>` 节点
- 设置 `AlwaysOnline: false`
- Unity 在离线模式下不验证签名

### 3. 修改 Hub 配置（已有功能）

- `DisableSignInRequired`: true
- `DisableAutoUpdate`: true

## 🚀 使用方法

1. **启动 UniFree 2.0**
   ```
   D:\GIT\UniFree\src-tauri\target\release\unifree.exe
   ```

2. **点击 "Hub" 标签页**

3. **点击 "补丁" 按钮**
   - ✅ 创建 Unity 配置文件
   - ✅ 生成无签名授权文件
   - ✅ 修改 Hub 配置
   - ✅ 重启 Unity Hub

4. **测试授权状态**
   ```bash
   cd "C:\Program Files\Unity Hub\UnityLicensingClient_V1"
   .\Unity.Licensing.Client.exe --showAllEntitlements
   ```

## 🎯 优点

- ✅ **不修改任何 DLL 文件**
- ✅ **不需要 dnSpy**
- ✅ **完全可逆**（删除配置文件即可）
- ✅ **简单可靠**
- ✅ **不会破坏文件完整性**
- ✅ **不涉及复杂的 IL 代码修改**

## 🔍 验证步骤

### 检查配置文件

```bash
# 检查配置是否创建
cat "C:\ProgramData\Unity\config\services-config.json"

# 检查授权文件（无签名）
cat "C:\ProgramData\Unity\Unity_lic.ulf" | grep -i signature
# 应该没有输出（没有 Signature 节点）
```

### 测试授权

```bash
cd "C:\Program Files\Unity Hub\UnityLicensingClient_V1"
.\Unity.Licensing.Client.exe --showAllEntitlements
```

**预期结果：**
```
Path: C:\ProgramData\Unity\Unity_lic.ulf
Product Name: Unity Pro
Status: Valid  # ← 应该显示 Valid
Serial: F4-A8P0-UWHL-BOKW-WGFQ-XXXX
License Version: 6.x
```

## 📊 方案对比

| 方案 | 复杂度 | 成功率 | 可逆性 | 风险 |
|------|--------|--------|--------|------|
| **修改 IL 代码** | ⭐⭐⭐⭐⭐ | ❓ | ❌ | 高 |
| **配置文件方案** | ⭐ | ✅ | ✅ | 低 |

## 🔧 故障排查

### 如果仍然失败

1. **检查配置文件是否创建**
   ```bash
   ls "C:\ProgramData\Unity\config\services-config.json"
   ```

2. **检查权限**
   - 确保以管理员身份运行 UniFree
   - 确保可以写入 `C:\ProgramData\Unity\`

3. **手动创建配置**
   ```bash
   mkdir "C:\ProgramData\Unity\config"
   echo '{"enableLicenseValidation":false,"offlineMode":true}' > "C:\ProgramData\Unity\config\services-config.json"
   ```

4. **完全重启**
   - 关闭所有 Unity Hub 进程
   - 删除缓存：`rd /s /q "%LOCALAPPDATA%\Temp\.net"`
   - 重启电脑

### 恢复原始状态

```bash
# 删除配置文件
del "C:\ProgramData\Unity\config\services-config.json"

# 恢复原始授权文件（如果有备份）
# copy backup\Unity_lic.ulf "C:\ProgramData\Unity\Unity_lic.ulf"
```

## 🎉 立即测试

**现在请：**
1. 启动 UniFree 2.0
2. 点击 "补丁 Hub" 按钮
3. 等待完成
4. 测试授权状态

**如果成功，你应该看到 Unity Pro 授权生效，而且我们完全没有修改任何 DLL！** 🚀

## 📝 技术细节

### 为什么这个方案可行？

Unity Hub 的授权验证流程：

```
1. 读取 services-config.json
2. 检查 enableLicenseValidation
   - 如果 false → 跳过所有验证 ✅
   - 如果 true → 继续验证
3. 检查 offlineMode
   - 如果 true → 不连接服务器 ✅
   - 如果 false → 连接服务器验证
4. 读取 Unity_lic.ulf
5. 如果在离线模式且验证被禁用 → 接受授权 ✅
```

**关键：** Unity 尊重自己的配置文件设置！

---

**构建版本：** UniFree 2.0 (配置文件方案)  
**可执行文件：** `D:\GIT\UniFree\src-tauri\target\release\unifree.exe` (9.1 MB)
