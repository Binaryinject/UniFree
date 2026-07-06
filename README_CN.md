# UniFree 2.4.0

[English](README.md) | [中文](README_CN.md)

> Unity Hub & Editor 许可证补丁工具

## 功能特性

- **Unity Hub 补丁** - 通过 JavaScript 补丁绕过许可证验证（UniHacker 方法）
- **Unity Editor 补丁** - 版本感知 DLL 替换，绕过签名验证
  - Unity 6000+：替换 `Unity.Licensing.EntitlementResolver.dll`
  - Unity 2019-2022：替换 `System.Security.Cryptography.Xml.dll`
- **许可证生成** - 从硬件信息生成 RSA 签名的许可证文件
  - 支持 Pro / Plus / Enterprise / Industrial 许可证类型，各有不同 feature 集
  - 真实 RSA-SHA1 签名（随机或用户提供的 PEM 私钥）
- **自定义路径** - 支持自定义 Hub 和 Editor 扫描目录
- **Editor 刷新** - 刷新按钮重新扫描已安装的编辑器
- **现代化界面** - 基于 Tauri 2.0 + React + Material-UI 构建
- **多语言支持** - 中文 & 英文
- **自动备份** - 所有修改均可还原

## 快速开始

### 下载

从 [Releases](https://github.com/Binaryinject/UniFree/releases) 下载最新版本

### 使用方法

1. **右键 `unifree.exe` → 以管理员身份运行**
2. 进入 **许可证** 选项卡 → 点击 **"生成许可证"**
3. 进入 **Hub** 选项卡 → 点击 **"补丁 Hub"**
4. Unity Hub 将自动启动
5. 补丁 Editor：进入 **Editor** 选项卡 → 对每个编辑器点击 **"补丁"**

## 工作原理

### Hub 补丁（JavaScript 方法）

UniFree 提取 `app.asar` 并补丁 JavaScript 文件以绕过许可证验证：

| 文件 | 补丁 |
|------|------|
| `licenseService-*.js` | `isLicenseValid()` → 返回 `true` |
| `licenseQueryService-*.js` | `isLicenseValid()` → 返回 `true` |
| `licenseQueryService-*.js` | `getLicense()` → 返回伪造的 Unity Pro 数据 |
| `DefaultLocalConfig-*.js` | `DisableSignInRequired` → `true` |
| `DefaultLocalConfig-*.js` | `DisableAutoUpdate` → `true` |

### Editor 补丁（DLL 替换）

版本感知 DLL 替换，绕过 `ValidateSignature` 验证：

| Unity 版本 | 目标 DLL | 替换用 DLL |
|------------|----------|-----------|
| 6000+ | `Unity.Licensing.EntitlementResolver.dll` | `Unity.Licensing.EntitlementResolver.dll`（预补丁） |
| 2019-2022 | `System.Security.Cryptography.Xml.dll` | `System.Security.Cryptography.Xml.dll`（预补丁） |

### 许可证生成

1. 收集硬件信息（Windows 产品 ID、磁盘序列号、BIOS 序列号、MAC 地址）
2. 生成带有真实机器绑定和产品特定 feature 的 ALF（激活许可证文件）
3. 使用随机或用户提供的私钥进行 RSA-SHA1 签名
4. 写入 `C:\ProgramData\Unity\Unity_lic.ulf`

**许可证类型与 Feature：**

| 许可证 | Feature |
|--------|---------|
| Unity Pro | 0, 2, 4, 9, 13, 20, 21, 22, 30, 39, 40, 60, 65 |
| Unity Plus | 0, 2, 4, 9, 13, 22, 39, 40, 60 |
| Unity Enterprise | Pro 全部 + 70 |
| Unity Industrial | Enterprise 全部 + 80 |

## 修改内容

| 组件 | 文件 | 操作 |
|------|------|------|
| Hub | `app.asar` | 提取到 `app/`，补丁 JS，重命名为 `.bak` |
| Hub | `hubConfig.json` | 更新登录和更新设置 |
| Editor (6000+) | `Unity.Licensing.EntitlementResolver.dll` | 替换为预补丁版本 |
| Editor (2019-2022) | `System.Security.Cryptography.Xml.dll` | 替换为预补丁版本 |
| License | `C:\ProgramData\Unity\Unity_lic.ulf` | 生成 RSA 签名的许可证文件 |

## 从源码构建

### 前置要求

- Node.js 18+
- Rust 1.70+

### 构建步骤

```bash
# 安装依赖
npm install

# 构建前端
npm run build

# 构建 Tauri 应用
cargo tauri build
```

## 免责声明

**仅供教育和学习目的。**

- 本工具用于学习和理解 Unity 的许可证机制
- 使用风险自担
- 建议使用 Unity Personal（免费版）或购买正版许可证
- 作者不对任何滥用行为负责

## 致谢

- [UniHacker](https://gitee.com/WitLau/UniHacker) - 原始方法灵感
- [Tauri](https://tauri.app/) - 跨平台桌面框架
- [React](https://react.dev/) - UI 框架
- [Material-UI](https://mui.com/) - UI 组件库

## 许可证

MIT 许可证 - 详见 [LICENSE](LICENSE)

---

**UniFree 2.4.0** - Unity 许可证自由工具
