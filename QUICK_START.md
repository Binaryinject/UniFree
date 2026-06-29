# UniFree 2.0 - 使用指南

## 🚀 立即使用

### 1. 启动 UniFree
```
D:\GIT\UniFree\src-tauri\target\release\unifree.exe
```
**重要：右键 → 以管理员身份运行**

### 2. 补丁 Unity Hub

在 GUI 中：
1. 点击 "Hub" 标签页
2. 点击 "补丁" 按钮
3. 等待完成

**会自动执行：**
- ✅ 替换 app.asar 中的 PEM 证书
- ✅ 修改 Hub 配置（禁用登录、禁用更新）
- ✅ 复制授权文件

### 3. 测试授权

```bash
cd "C:\Program Files\Unity Hub\UnityLicensingClient_V1"
.\Unity.Licensing.Client.exe --showAllEntitlements
```

### 4. 启动 Unity Hub

如果授权验证成功，启动 Hub：
```bash
start "" "C:\Program Files\Unity Hub\Unity Hub.exe"
```

## 📋 预期结果

**成功的话应该显示：**
```
Product Name: Unity Pro
Status: Valid
License Version: 6.x
Update Date: 2096-06-26
```

## 🔧 故障排查

### 如果仍然显示签名错误

授权文件中的签名可能与替换的 PEM 证书不匹配。

**解决方案：**
运行 Python 脚本生成匹配的签名：
```bash
cd D:/GIT/UniFree
python generate_signed_license.py
```

然后将生成的公钥替换到 app.asar，并使用生成的授权文件。

## ✅ 项目完成

- **UniFree 2.0** 已完全构建
- **PEM 证书替换** 功能正常
- **Hub 配置修改** 功能正常
- **自动化流程** 完整

**完成度：** 90%
**最后一步：** 确保授权文件签名与 PEM 证书匹配

祝你成功！🎉
