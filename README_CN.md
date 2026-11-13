# UniFree 2.9.4

[English](README.md) | [中文](README_CN.md)

> Unity Hub & Editor 许可证补丁工具

## 功能特性

- **Unity Hub 补丁** - 通过 JavaScript 补丁绕过许可证验证（UniHacker 方法）
- **Unity Editor 补丁** - 版本感知补丁，绕过签名验证
  - Unity 6000.7+：**Native AOT 二进制补丁** — 1 个字节级锚点，直接短路整个 ValidateSignature 验证
  - Unity 6000.0-6000.6：**就地 IL 字节补丁** `Unity.Licensing.EntitlementResolver.dll`（任意 LocalIPC 线通用，并禁用 ReadyToRun）
  - Unity 2019-2022：替换 `System.Security.Cryptography.Xml.dll`
- **许可证生成** - 从硬件信息生成 RSA 签名的 Unity Pro 许可证文件
- **自定义路径** - 支持自定义 Hub 和 Editor 扫描目录
- **Editor 刷新** - 刷新按钮重新扫描已安装的编辑器
- **现代化界面** - 基于 Tauri 2.0 + React + Material-UI 构建
- **多语言支持** - 中文 & 英文
- **自动备份** - 所有修改均可还原
- **macOS（Apple Silicon）** - 原生 arm64 构建。Unity 6 编辑器分两条路补：托管 licensing client 走就地 IL 补丁，
  6000.3.28+ 的 Native AOT client 走原生锚点；同时绕开编辑器侧的 Licensing Client 代码签名校验，并对整个
  `Unity.app` 做 ad-hoc 重签。macOS 上的 Unity Hub 同样支持。

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

UniFree 就地重建 `app.asar`（保留 `app.asar.unpacked/` 里的 native 模块）并补丁 JavaScript 文件以绕过许可证验证；同时翻转 Hub exe 里的 Electron `EnableEmbeddedAsarIntegrityValidation` fuse，否则改动 asar 会触发启动崩溃：

| 文件 | 补丁 |
|------|------|
| `licenseService-*.js` | `isLicenseValid()` → 返回 `true` |
| `licenseQueryService-*.js` | `isLicenseValid()` → 返回 `true` |
| `licenseQueryService-*.js` | `getLicense()` → 返回伪造的 Unity Pro 数据 |
| `DefaultLocalConfig-*.js` | `DisableSignInRequired` → `true` |
| `DefaultLocalConfig-*.js` | `DisableAutoUpdate` → `true` |

### Editor 补丁

版本感知补丁，绕过 `ValidateSignature` 签名验证：

| Unity 版本 | 目标文件 | 方法 |
|------------|----------|------|
| **6000.7+** | `Unity.Licensing.Client.exe` | **字节级锚点补丁**（1 个补丁：ValidateSignature 包装函数 → 恒为有效） |
| 6000.0-6000.6 | `Unity.Licensing.EntitlementResolver.dll` | 就地 IL 字节补丁（任意 LocalIPC 线）· 未命中时回退按线选预补丁 DLL |
| 2019-2022 | `System.Security.Cryptography.Xml.dll` | 预补丁 DLL 替换 |

对于 6000.7+，二进制文件为 .NET 10 Native AOT 编译（无 IL 代码）。补丁使用模式匹配锚点定位并修改原生指令，在同发布线内具有跨小版本的兼容性。详见 `docs/editor-dll-patching.md`。

### 许可证生成

1. 收集硬件信息（Windows 产品 ID、磁盘序列号、BIOS 序列号、MAC 地址）
2. 生成带有真实机器绑定的 ALF（激活许可证文件）
3. 使用随机私钥进行 RSA-SHA1 签名
4. 写入 `C:\ProgramData\Unity\Unity_lic.ulf`

**Unity Pro Features:** 0, 2, 4, 9, 13, 20, 21, 22, 30, 39, 40, 60, 65

## 修改内容

| 组件 | 文件 | 操作 |
|------|------|------|
| Hub | `app.asar` | 就地重建（保留 unpacked 标记），补丁 `getLicense`/`isLicenseValid` |
| Hub | `Unity Hub.exe` | 翻转 Electron `EnableEmbeddedAsarIntegrityValidation` fuse |
| Hub | `UnityLicensingClient_V1\Unity.Licensing.EntitlementResolver.dll` | 就地 IL 字节补丁（任意 LocalIPC 线：1.17.x / 1.18+ / 未来） |
| Hub | `hubConfig.json` | 更新登录和更新设置 |
| Editor (6000.7+) | `Unity.Licensing.Client.exe` | 字节级二进制补丁（1 个锚点补丁） |
| Editor (6000.0-6000.6) | `Unity.Licensing.EntitlementResolver.dll` | 就地 IL 字节补丁（任意 LocalIPC 线）· 未命中时回退按线选预补丁 DLL |
| Editor (2019-2022) | `System.Security.Cryptography.Xml.dll` | 替换为预补丁 DLL |
| License | `C:\ProgramData\Unity\Unity_lic.ulf` | 生成 RSA 签名的许可证文件 |

## macOS（Apple Silicon）

macOS 版本**只支持 Apple Silicon（M 系列 / arm64）**，Intel Mac 不在支持范围内。

当前 macOS 覆盖范围：**使用托管 licensing client 的 Unity 6 编辑器**
（6000.0 – 6000.3.27，即目录里仍有 `Unity.Licensing.EntitlementResolver.dll` 的版本）与 Unity Hub。
2019–2022 依赖 Windows 侧的 DLL 替换，macOS 暂不覆盖。
更新的版本（如 6000.3.28+）改用 **Native AOT** client——单个 arm64 Mach-O、不再有托管 DLL：
这类版本同样支持，直接短路 `ValidateSignature` 等价函数（`mov w0,#1; ret`），并绕开编辑器侧的
client 代码签名校验（`cmp w0,#9; b.ne` → `nop`），最后对整个 `Unity.app` 做 ad-hoc 重签。

macOS 补丁做了什么：

| 组件 | 文件 | 操作 |
|------|------|------|
| Hub | `Unity Hub.app/Contents/Resources/app.asar` | 就地重写 asar（JS 补丁与 Windows 一致） |
| Hub | `Contents/Frameworks/Electron Framework.framework/Electron Framework` | 翻转 `EnableEmbeddedAsarIntegrityValidation` fuse 为 `0` |
| Hub | `.../UnityLicensingClient_V1.app/Contents/MacOS/Unity.Licensing.EntitlementResolver.dll` | 就地 IL 字节补丁（`ValidateSignature` 恒通过） |
| Hub / 编辑器 | `.app` 包本身 | 补丁后 `codesign --force --deep --sign -`（ad-hoc）重签，否则 macOS 拒绝加载被改动的 Mach-O |
| 编辑器 | `Unity.app/Contents/Helpers/UnityLicensingClient.app/Contents/MacOS/Unity.Licensing.EntitlementResolver.dll` | 就地 IL 字节补丁（托管 DLL，与 Windows 完全一致） |
| 许可证 | `/Library/Application Support/Unity/Unity_lic.ulf` | 本地生成；写入时弹一次管理员授权（`osascript ... with administrator privileges`） |

说明：

- resolver DLL 是与平台无关的托管程序集，同一套 IL 补丁在 macOS / Windows 通用。
- 重签是 ad-hoc（`codesign -`），不需要 Apple 开发者证书；补丁后会自动清除 quarantine 标记。
- 所有改动都有 `*.bak` 备份，可在应用内还原。

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

# 构建 Tauri 应用（Windows: NSIS / macOS: DMG，arm64）
cargo tauri build

# 明确构建 macOS arm64
cargo tauri build --target aarch64-apple-darwin
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

**UniFree 2.8.3** - Unity 许可证自由工具

## 更新日志

### v2.9.4
- **修复 macOS 发布流水线**：本账号不支持 arm64 托管 runner（报
  `configuration 'macos-14-arm64-us-default' is not supported`），macOS job 改在 `macos-13` 上交叉编译
  `aarch64-apple-darwin`，产物仍是 arm64 DMG。

### v2.9.3
- **图标不再模糊**：原图标母版只有 32×32，Dock 放大后发糊。重新生成 1024×1024 母版（超采样），并重出全套
  `icon.icns`（16…1024）、各尺寸 PNG、Windows `.ico` 与 `app-icon.png`。

### v2.9.2
- **修复 macOS AOT 还原后状态**：还原时没有删除包外备份，而状态判定把“存在备份”当成“已补丁”，导致按钮一直停在
  “还原”。现在还原会清理备份，AOT 补丁状态改用 16 字节尾部锚点判定（入口锚点已被补丁覆盖）。

### v2.9.1
- **支持 Unity 6000.3.28+（macOS，Native AOT licensing client）**：
  - 客户端：24 字节唯一锚点定位 `ValidateSignature` 等价函数，入口改写为 `mov w0,#1; ret`，
    ULF 与 entitlement XML 的签名校验同时通过；
  - 编辑器：定位 `Code {0} while verifying Licensing Client signature` 格式串 → `adrp/add` 引用 →
    `cmp w0,#9` 后的 `b.ne` 改 `nop`，绕开 client 代码签名校验；
  - 两处改完统一 `codesign --force --deep` 重签 `Unity.app`（分步重签会留下不一致封印）。
  - 已在 6000.3.28f1 arm64 实测：退出码 198 → 0，`License group: Product: Unity Pro`，
    `Successfully parsed (ULF) license`。

### v2.9.0
- **新增 macOS（Apple Silicon）支持**：原生 arm64 `app`/`dmg`；macOS Hub 补丁（asar + `Electron Framework`
  里的 fuse + ad-hoc 重签）；托管 client 就地 IL 补丁 + `DOTNET_ReadyToRun=0` 启动器；
  macOS 许可证生成（`/Library/Application Support/Unity/Unity_lic.ulf`，一次管理员授权）；
  备份改存 `.app` 包外。

### v2.8.3
- **补齐 Hub 3.22.x 的三态授权门禁**：Hub 3.22.0（HUB-7120）把布尔式许可校验换成了三态 verdict
  （`ENTITLED` / `NOT_ENTITLED` / `UNVERIFIED`），并通过 `licenses/getEditorEntitlementDecision` 暴露给渲染进程。
  `LicenseQueryService.checkEditorEntitlement()` 与 `checkUlfLicense()` 会直连 licensing client
  （`checkEntitlements([com.unity.editor.ui])` / `getAllEntitlementGroups()`），所以只补 `isLicenseValid()`
  已不够：Hub 的 UI 阻塞点与项目打开门禁仍按客户端真实答复走。现在这两个 verdict 生产点也一并改成 `ENTITLED`
  （`getLicense()` / `isLicenseValid()` 的补丁保持原样）。
- **补丁状态改为按内容判定，不再看 `.bak` 残留**：安装器/卸载器都不会删除
  `app.asar.bak` / `*.dll.bak` / `Unity Hub.exe.bak`，所以 Hub 重装或升级后本工具仍显示"已补丁"，
  而安装器其实已经把 `app.asar` 还原成原版——这正是"重启后 Hub 显示没有许可证、但项目还能打开"的成因。
  现在按三个独立补丁点判定（asar 内 JS 标记 / resolver IL 补丁状态 / asar 完整性 fuse 字节），
  分别报 `patched` / `partial` / `original`。
- **陈旧备份保护**：从 `app.asar.bak` 恢复前，先比对备份内 `package.json` 的版本与当前安装 Hub 的
  `ProductVersion`；属于旧版本的备份直接丢弃，避免把上一版本的 JS 与文件表灌进新版本 Hub。
- **跳过登录的补丁扩大到 3.22 的第二份默认值副本**：只改 `DefaultLocalConfig-*.js` 会让「从 Hub 安装编辑器」
  的容器窗口（`installerContainerSetup-*.js` 里另有一份 `DefaultLocalConfig`）重新要求登录。现在按内容匹配
  任意 JS 里的默认值行（`DisableSignInRequired` / `DisableAutoUpdate`），`tools/patch-hub-js.mjs` 同步支持，
  并按 `config_patcher` 的语义一并写 `hubConfig.json` 的 `hubDisableSignInRequired` / `hubDisableAutoUpdate`。
- `partial` 状态下也允许"恢复原版"。

### v2.8.2
- **就地 IL 字节补丁取代预补丁 resolver DLL（6000.0-6000.6 与 Hub）**：不再按发行线替换重编译 DLL，
  而是直接改写原文件里 `ValidateSignature` 的 IL——`brtrue.s`+`ldstr`+`newobj`+`throw` →
  `pop`+12×`nop`（13 字节等长、栈语义等价）——保留程序集身份，**任意 LocalIPC 发行线直接可用**
  （1.17.x、1.18+、未来新线），无需再维护按线预编译产物；预补丁 DLL 保留为自动回退。
- **ReadyToRun 修复（关键）**：Unity 的 resolver 是 ReadyToRun 镜像（`RTR`，ManagedNativeHeader）。
  补丁时同步清零 CLI 头的 `ManagedNativeHeader`，否则运行时仍执行旧的原生码，IL 补丁静默失效
  （dnlib 验证"通过"、运行期行为不变）。已端到端实测：licensing client 对 UniFree 签名的 ULF
  输出 `Successfully parsed (ULF) license ...`、`Processed 2 license files`。
- **Unity Hub 3.21.1 支持（LocalIPC 1.18.3）**：`patch_hub_licensing_client()` 现在也支持 1.18+ 线
  （就地补丁优先，按线回退）。修复了 Hub client 协议与编辑器 1.18.3 一致时，从 Hub 启动的编辑器
  报退出码 198（`No valid Unity Editor license found.`）的问题——此前只适配 1.17.x 线。
- 新增 `tools/verify-resolver`（dnlib 结构校验补丁产物）与 `tools/resolver-invoke`
  （反射直接调用 `XmlExtensions.ValidateSignature` 验证语义）。
- 详见 `docs/editor-dll-patching.md`。

### v2.8.1
- **修复 6000.3.10f1（LocalIPC 1.17.x）补丁后仍报 "No valid Unity Editor license found."**：
  - 按 licensing client 发行线选择预补丁解析器：1.17.x（如 6000.3.10f1，原版程序集版本 1.17.4.0、v7 引用）
    使用基于 1.17.4 原版生成、保留程序集版本的补丁 DLL（`Unity.Licensing.EntitlementResolver.1.17.4.dll`），
    1.18+（如 6000.3.20f1/22f1，0.0.0.0、v8 引用）沿用原捆绑版本；
    旧版 0.0.0.0 补丁 DLL 装入 1.17.4 client 会因 deps 版本不匹配启动即崩
    （`Could not load file or assembly '…, Version=1.17.4.0'`）。
  - **Hub licensing client 的 resolver 一并打补丁**（`Unity.Licensing.EntitlementResolver.hub.1.17.4.dll`）：
    从 Hub 启动的编辑器经由 Hub 的 licensing client（全局管道 `LicenseClient-wbn`）也能通过
    ULF 签名校验，**无需启动独立 IPC 进程、无需 UniFree 常驻，补丁一次永久生效（含重启）**；
    不影响 1.18+ 编辑器自己的 licensing client（独立目录/进程/版本化管道，Hub client 仍对其回
    `505 Unsupported protocol version`）。
  - 新增 `tools/patchresolver`：基于任意 1.17.x 原版一键生成同线预补丁解析器。
  - 详见 `docs/editor-dll-patching.md`。

### v2.8.0
- **Unity 2019.4 原生补丁**：为 2019.x 编辑器新增两个基于锚点的 `Unity.exe` 字节级补丁：
  - 绕过 `ValidateServerProcess` —— Licensing Client 的 Authenticode 签名校验（代码签名证书已于 2024-07-19 过期）不再拒绝 Licensing Client。
  - 绕过 `LICENSE SYSTEM` 错误分发器 —— 原生 `WinILicensingAdapter` 不再报 "Unity license information is invalid."。
- 编辑器补丁流程现在先应用这两个原生补丁，再替换 `System.Security.Cryptography.Xml.dll`；恢复时一并还原 `Unity.exe`。
- 已知问题（2019.4）：通过校验关卡后，编辑器仍报 `License is not active (com.unity.editor.ui)`，因为生成的 ULF 是旧版 `<Features>` 格式，而 Licensing Client 按新版 `<EntitlementGroups>` 格式解析。详见 `docs/editor-dll-patching.md`。

### v2.7.0
- 新增中英文语言切换，并持久化用户选择（重启后记住）。
- 日志面板新消息自动滚动（仅在停留在底部时）。
- Tab 懒加载 + vendor 分包，减小打包体积。
- 用 Win32 API 替换 PowerShell/wmic/tasklist，扫描更快、兼容性更好（修复 Win11 24H2+ 移除 wmic 的问题）。
- 编辑器 DLL 状态改为内容比对，不再依赖文件大小启发式。
- Hub 配置状态现在反映真实的 hubConfig.json 状态。
- 新增 Content Security Policy。
- 仓库清理：从 git 历史中移除构建产物。

### v2.6.1
- 从 `Unity.exe` 文件信息读取编辑器版本，不再依赖安装目录名称。
- 支持重命名目录和非 Unity Hub 安装的编辑器，包括 Unity 6000.3.x 目录结构。
- 根据实际目标文件名选择补丁，同时在界面中保留完整版本号。

### v2.6.0
- **支持 Unity Hub 3.20.0+**：Hub 的 Electron 带了 `OnlyLoadAppFromAsar` + 
  `EnableEmbeddedAsarIntegrityValidation` 双 fuse，旧"解包到 app/"和任何 asar 改动都会失败
  （启动即退出 / Integrity FATAL）。
  - `flip_hub_exe_fuses()` 翻转 `Unity Hub.exe` 里的 asar 完整性 fuse（Electron fuse wire：
    `[magic][ver=01][len=09][ASCII '0'/'1']`，fuse 4）
  - `rewrite_hub_asar()` 就地重建 `app.asar` 并**保留 10 个 unpacked native 模块标记**
    （AsarWriter 会丢掉 → 报 `Cannot find native binding`）
  - patch `licenseQueryService.getLicense()` 返回假的 Unity Pro ULF，让 Hub **显示**许可证；
    `isLicenseValid()` → true
- Hub 的 licensing client（managed .NET，`Unity.Licensing.EntitlementResolver.dll`）与
  Editor 6000.7+ Native AOT 是两套架构；
  **就地 IL 补丁 Hub licensing client 的 resolver**（v2.8.2 起，任意 LocalIPC 线：
  1.17.x / 1.18+ / 未来）：从 Hub 启动的
  编辑器经由 Hub 的 licensing client（全局管道 `LicenseClient-wbn`）也能通过 ULF 签名校验，
  无需启动独立 IPC 进程，补丁一次永久生效（含重启后，见 `docs/editor-dll-patching.md`）。

### v2.5.7
- **6000.7+ 补丁精简为单个字节级补丁**：将 `ValidateSignature` 包装函数（sub_1404F1C10）
  头部改为 `mov eax,1; ret`，取代此前的 2 补丁（签名门控 + LABEL_14 信任检查）和最初的
  4 补丁方案
- IDA 实证：Parse 调用该包装函数但忽略返回值（失败靠异常中断），一个函数头补丁即可短路
  整个验证链路，RSA 签名 ULF 保持不变
- 锚点针对配置字段读取 + 双重类型检查，同发布线内跨小版本兼容
- 回归测试在真实二进制副本上应用补丁，断言恰好 1 个写入点

### v2.5.6
- 修复自动更新：批处理文件在安装前轮询 PID，安装后重启

### v2.5.5
- 修复自动更新：安装后原始进程重启

### v2.5.4
- 修复自动更新：进程内运行 NSIS 安装器后重启

### v2.5.3
- 修复自动更新：使用 PowerShell 脚本可靠静默安装 + 重启（正确处理含空格路径）

### v2.5.2
- 修复自动更新：使用 `ping` 延时 + `start /wait` 确保静默安装完成后自动重启
- 防止 NSIS 安装器子进程提前返回导致的竞态问题

### v2.5.0
- **Native AOT 二进制补丁支持 Unity 6000.7+** — 字节级锚点补丁绕过 `Unity.Licensing.Client.exe` 的许可证签名验证
- 4 个精准补丁：证书链绕过、ValidateSignature 门控跳过、LABEL_14 证书检查绕过、BCrypt/NCrypt NTSTATUS 绕过
- 基于锚点的模式匹配，同 Unity 发布线内跨小版本兼容

### v2.4.0
- 新增自动更新：启动时检查 GitHub releases 是否有新版本
- 下载进度条支持取消操作
- 下载完成后静默 NSIS 安装

### v2.3.1
- 移除许可证类型选择（Enterprise/Industrial/Plus）
- 简化为 Unity Pro — Unity Hub 的 Licensing Client 不支持 Enterprise/Industry 类型
- 移除自定义 RSA PEM 密钥输入（自动生成随机密钥）
- 替换 Hub 的 `System.Security.Cryptography.Xml.dll` 以绕过签名验证
- 恢复 Hub 的 Licensing Client 以正确检测许可证
