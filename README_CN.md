# UniFree 2.2.0

[English](README.md) | [中文](README_CN.md)

> Unity Hub & Editor 许可证补丁工具

## 功能特性

- **Unity Hub 补丁** - 通过 JavaScript 补丁绕过许可证验证（UniHacker 方法）
- **Unity Editor 补丁** - 替换 DLL 绕过签名验证
- **许可证生成** - 从硬件信息生成有效的许可证文件
- **自定义路径** - 支持自定义 Hub 和 Editor 目录
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

替换 `Unity.Licensing.EntitlementResolver.dll` 为预补丁版本，绕过 `ValidateSignature` 验证。

### 许可证生成

1. 收集硬件信息（Windows 产品 ID、磁盘序列号、BIOS 序列号、MAC 地址）
2. 生成带有真实机器绑定的 ALF（激活许可证文件）
3. 转换为带有空签名节点的 ULF
4. 写入 `C:\ProgramData\Unity\Unity_lic.ulf`

## 修改内容

| 组件 | 文件 | 操作 |
|------|------|------|
| Hub | `app.asar` | 提取到 `app/`，补丁 JS，重命名为 `.bak` |
| Hub | `hubConfig.json` | 更新登录和更新设置 |
| Editor | `Unity.Licensing.EntitlementResolver.dll` | 替换为预补丁版本 |
| License | `C:\ProgramData\Unity\Unity_lic.ulf` | 生成并写入许可证文件 |

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

**UniFree 2.2.0** - Unity 许可证自由工具
