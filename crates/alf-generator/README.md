# ALF Generator

Unity ALF (Activation License File) 生成器，与 Unity Licensing Client 完全兼容。

## 功能

- 生成与 Unity Hub 完全一致的 ALF 文件
- 使用与 Unity Licensing Client 相同的机器绑定算法
- 支持 Windows 平台的完整硬件绑定
- 可作为库或命令行工具使用

## 安装

### 作为库使用

在 `Cargo.toml` 中添加：

```toml
[dependencies]
alf-generator = { path = "../crates/alf-generator" }
```

### 作为命令行工具

```bash
cargo install --path crates/alf-generator
```

## 使用方法

### 库 API

```rust
use alf_generator::AlfGenerator;

// 使用默认 Unity 版本 (2017.2.0)
let generator = AlfGenerator::new();
let alf_content = generator.generate();
std::fs::write("Unity_lic.alf", alf_content).unwrap();

// 指定 Unity 版本
let generator = AlfGenerator::new().with_unity_version("2022.3.0f1");
let alf_content = generator.generate();
```

### 命令行工具

```bash
# 生成默认 ALF 文件
alf-gen

# 指定输出路径
alf-gen -o my_license.alf

# 指定 Unity 版本
alf-gen -v 2022.3.0f1

# 显示机器绑定信息
alf-gen -b

# 查看帮助
alf-gen -h
```

## 机器绑定说明

ALF 文件包含以下机器绑定信息：

| Key | 说明 | 来源 |
|-----|------|------|
| 1 | Windows Product ID | 注册表 `HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion\ProductId` |
| 2 | 启动磁盘序列号 | C: 盘物理磁盘序列号，经过 FlipAndCodeBytes 处理 |
| 4 | BIOS 序列号 | WMI `Win32_BIOS.SerialNumber`，Base64 编码 |
| 5 | MAC 地址 | 第一个网络适配器的 MAC 地址 |

MachineID 生成算法：
```
MachineID = Base64(SHA1(Binding1 + Binding2 + Binding4))
```

## 兼容性

- Windows 10/11
- 与 Unity Licensing Client 1.17.x 完全兼容
- 生成的 ALF 文件可直接用于 LicenseGenerate.exe

## 许可证

MIT
