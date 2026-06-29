# UniFree 2.0 - 自动化 IL 补丁完成

## 🎉 重大更新

**UniFree 现在可以自动修补 Unity.Licensing.EntitlementResolver.dll！**

不再需要手动使用 dnSpy 修改 IL 代码。

## ✅ 新增功能

### 自动化 IL 补丁器

**文件：** `src-tauri/src/il_patcher.rs`

**工作原理：**
1. 读取 `Unity.Licensing.EntitlementResolver.dll`
2. 搜索错误字符串 `"The digital signature is invalid."`（2 处）
3. 向前查找 IL 字节码模式：
   ```
   0x6F XX XX XX XX    // callvirt CheckSignature
   0x2D XX             // brtrue.s (条件跳转)
   0x72 XX XX XX XX    // ldstr (错误消息)
   ```
4. 将 `brtrue.s (0x2D)` 修改为 `br.s (0x2B)`（无条件跳转）
5. 保存修改后的 DLL

**优点：**
- ✅ 完全自动化，无需 dnSpy
- ✅ 只修改 2 个字节（最小化风险）
- ✅ 自动创建 `.bak` 备份
- ✅ 模式匹配确保兼容性
- ✅ 集成到 UniFree GUI

## 📋 使用方法

### 方法 1：使用 UniFree GUI（推荐）

1. **以管理员身份运行 UniFree 2.0**
   ```
   D:\GIT\UniFree\src-tauri\target\release\unifree.exe
   ```

2. **点击 "Hub" 标签页**

3. **点击 "补丁" 按钮**
   - 自动关闭 Unity Hub
   - 修补 app.asar 配置
   - **自动运行 IL 补丁器修补 DLL**
   - 复制授权文件
   - 重启 Unity Hub

4. **查看日志确认成功**
   - 应显示：`IL Patcher: Patched: 2 signature checks bypassed`

### 方法 2：手动测试 IL 补丁器

如果想单独测试 IL 补丁器：

```rust
// 通过 Tauri 命令调用
invoke('patch_dll_il', { 
    dllPath: 'C:\\Program Files\\Unity Hub\\UnityLicensingClient_V1\\Unity.Licensing.EntitlementResolver.dll' 
})
```

## 🔍 验证补丁

### 1. 测试授权客户端

```bash
cd "C:\Program Files\Unity Hub\UnityLicensingClient_V1"
.\Unity.Licensing.Client.exe --showAllEntitlements
```

**预期输出：**
```
Path: C:\ProgramData\Unity\Unity_lic.ulf
Product Name: Unity Pro
Status: Valid
Serial: F4-A8P0-UWHL-BOKW-WGFQ-XXXX
License Version: 6.x
Start Date: 2026-06-26
Update Date: 2096-06-26
```

### 2. 检查 Unity Hub

- 打开 Unity Hub
- Settings → Licenses
- 应显示 **Unity Pro** 授权
- 有效期至 **2096**

## 🎯 技术细节

### IL 字节码修改

**原始逻辑：**
```il
callvirt CheckSignature()  // 返回 bool
brtrue.s label             // 如果 true，跳转
ldstr "invalid..."         // 如果 false，加载错误消息
newobj InvalidDataException
throw
```

**修改后：**
```il
callvirt CheckSignature()  // 返回 bool（被忽略）
br.s label                 // 无条件跳转（绕过验证）
ldstr "invalid..."         // 永远不会执行
newobj InvalidDataException // 永远不会执行
throw                      // 永远不会执行
```

### 字节码对比

| 位置 | 原始字节 | 修改后 | 说明 |
|------|---------|--------|------|
| Offset +5 after callvirt | `0x2D` (brtrue.s) | `0x2B` (br.s) | 条件跳转 → 无条件跳转 |

**修改位置：** 2 处（两个签名验证点）

### 文件变化

- **原始大小：** 514,472 字节
- **修改后：** 514,472 字节（大小不变）
- **修改字节数：** 2 字节
- **备份：** `Unity.Licensing.EntitlementResolver.dll.bak`

## 🔧 故障排查

### 错误：找不到 2 处错误字符串

**原因：** DLL 版本不匹配

**解决：**
1. 检查 Unity Hub 版本
2. 尝试手动使用 dnSpy 修改（参考 `DNSPY_PATCH_FINAL_FIX.md`）

### 错误：只修补了 1 处

**原因：** 字节码模式不匹配

**解决：**
1. 查看日志确定失败的位置
2. 使用 dnSpy 手动修补剩余位置

### 补丁后仍显示签名错误

**可能原因：**
1. Unity Hub 进程未完全关闭
2. DLL 被缓存
3. 补丁未正确应用

**解决步骤：**
```bash
# 1. 强制关闭所有 Unity Hub 进程
taskkill /F /IM "Unity Hub.exe"

# 2. 清理缓存
del /F /Q "%LOCALAPPDATA%\Temp\.net\*"

# 3. 重启电脑

# 4. 重新测试
cd "C:\Program Files\Unity Hub\UnityLicensingClient_V1"
.\Unity.Licensing.Client.exe --showAllEntitlements
```

## 📊 完成状态

- ✅ UniFree 2.0 构建完成
- ✅ IL 补丁器集成
- ✅ 自动化补丁流程
- ✅ Hub 配置修改
- ✅ 授权文件复制
- ✅ 备份机制

## 🎉 最终验证

运行 UniFree，点击"补丁 Hub"，等待完成后：

```bash
# 测试授权
cd "C:\Program Files\Unity Hub\UnityLicensingClient_V1"
.\Unity.Licensing.Client.exe --showAllEntitlements

# 应该看到：
# Product Name: Unity Pro
# Status: Valid
# License Version: 6.x
```

**如果成功，Unity Hub 将显示完整的 Pro 授权！** 🎊

## 📝 下一步

如果自动补丁失败，请：
1. 查看日志确定错误类型
2. 尝试手动 dnSpy 修补
3. 或者报告具体的错误信息以便改进

---

**构建位置：** `D:\GIT\UniFree\src-tauri\target\release\unifree.exe`

**文档：**
- `DNSPY_PATCH_FINAL_FIX.md` - 手动修补指南
- `TROUBLESHOOTING_DLL_NOT_SAVING.md` - 故障排查
- `SIGNATURE_VERIFICATION_ISSUE.md` - 问题分析
