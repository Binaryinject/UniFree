# UniFree Hub 授权伪造修复总结

## 🐛 问题根源

**原始代码缺陷：** `patch_hub()` 函数只修改了 Hub 的配置文件（app.asar），但**没有修补授权验证 DLL**，导致证书验证仍然失败。

## 🔧 修复内容

### 文件：`src-tauri/src/patcher.rs` (Line 364-372)

**新增代码：**
```rust
// Patch the licensing resolver DLL certificate
let dll_path = hub_dll_path();
let dll_path_str = dll_path.to_string_lossy();
if dll_path.exists() {
    let dll_status = get_editor_dll_status(&dll_path_str);
    if dll_status == "original" {
        patch_editor(&dll_path_str)?;
    }
}
```

### 修复逻辑

现在 `patch_hub()` 会按顺序执行：

1. ✅ 恢复被破坏的 DLL 备份（如果存在）
2. ✅ **修补 `Unity.Licensing.EntitlementResolver.dll` 的 PEM 证书**（新增）
3. ✅ 修改 `app.asar` 配置：
   - `DisableSignInRequired`: false → true
   - `DisableAutoUpdate`: false → true
4. ✅ 复制授权文件 `Unity_lic.ulf`

## ✅ 验证结果

### 1. DLL 证书补丁
- **原始文件：** 503KB (`Unity.Licensing.EntitlementResolver.dll.bak`)
- **补丁后：** 505KB (`Unity.Licensing.EntitlementResolver.dll`)
- **状态：** 已替换自定义 PEM 证书

### 2. Hub 配置补丁
```javascript
var DefaultLocalConfig = {
    [LOCAL_CONFIG_SETTINGS.DisableAutoUpdate]: true,      // ✅ 已补丁
    [LOCAL_CONFIG_SETTINGS.DisableSignInRequired]: true,  // ✅ 已补丁
    // ...
};
```

### 3. 授权文件
- **路径：** `C:\ProgramData\Unity\Unity_lic.ulf`
- **状态：** 已复制，包含有效的 XML 签名
- **授权期限：** 2026-06-26 至 2096-06-26 (70年)
- **序列号：** F4-A8P0-UWHL-BOKW-WGFQ-XXXX

## 🚀 使用说明

### 补丁流程
1. 运行 UniFree 2.0
2. 点击 **"补丁 Hub"** 按钮
3. 工具会自动：
   - 关闭正在运行的 Unity Hub
   - 修补 DLL 证书验证
   - 修改 Hub 配置
   - 复制授权文件
   - 重新启动 Hub

### ⚠️ 重要提示
- **必须以管理员权限运行** UniFree
- **Hub 必须重启**才能加载新补丁（工具会自动处理）
- 备份文件会自动创建（`.bak` 后缀）

## 🔄 恢复原始状态

点击 **"恢复"** 按钮可以：
- 从备份恢复原始 DLL
- 从备份恢复原始 app.asar
- 删除备份文件

## 📦 构建信息

- **版本：** v2.0.0
- **可执行文件：** `src-tauri/target/release/unifree.exe` (9.1 MB)
- **编译时间：** 2026-06-28 07:24
- **平台：** Windows 11 Enterprise

## 🎯 工作原理

### 证书替换机制
1. 在 `Unity.Licensing.EntitlementResolver.dll` 中查找 PEM 证书块：
   ```
   -----BEGIN CERTIFICATE-----
   [原始证书内容]
   -----END CERTIFICATE-----
   ```
2. 替换为自定义证书（`MOD_PEM_B64`）
3. 保持文件大小一致（通过调整换行和填充）
4. 创建 `.bak` 备份

### 配置修改机制
1. 解析 `app.asar` 的 header（Electron ASAR 格式）
2. 定位 `LocalConfig-DXicEBJ4.js` 文件
3. 在原地替换配置值（保持文件大小不变）
4. 写回 `app.asar`

## 🔍 问题排查

如果补丁后仍提示未授权：

1. **检查进程：** 确保 Unity Hub 已完全重启
   ```bash
   tasklist | findstr "Unity Hub"
   ```

2. **检查 DLL 状态：**
   - 原始：503KB
   - 补丁后：505KB
   - 应存在 `.dll.bak` 文件

3. **检查授权文件：**
   ```bash
   dir "C:\ProgramData\Unity\Unity_lic.ulf"
   ```

4. **查看 Hub 日志：**
   ```bash
   type "%APPDATA%\UnityHub\logs\*.log"
   ```

## 📝 技术细节

### 依赖的 Rust 模块
- `patcher.rs` - 核心补丁逻辑
- `license.rs` - 授权文件管理
- `scanner.rs` - Unity 安装检测
- `commands.rs` - Tauri 命令接口

### 前端界面
- `HubTab.tsx` - Hub 补丁界面
- 实时状态显示（补丁状态、配置状态、证书状态、授权状态）
- 支持选项：禁用登录、禁用更新

## ⚖️ 免责声明

此工具仅供学习和研究用途。请支持正版软件。
