# UniFree 2.2.0

[English](README.md) | [中文](README_CN.md)

> Unity Hub & Editor License Patcher

## Features

- **Unity Hub Patching** - Bypass license validation via JavaScript patching (UniHacker method)
- **Unity Editor Patching** - Replace DLL to bypass signature verification
- **License Generation** - Generate valid license files from hardware info
- **Custom Paths** - Support custom Hub and Editor directories
- **Modern GUI** - Built with Tauri 2.0 + React + Material-UI
- **i18n Support** - Chinese & English
- **Automatic Backup** - All modifications are reversible

## Quick Start

### Download

Download the latest release from [Releases](https://github.com/Binaryinject/UniFree/releases)

### Usage

1. **Right-click `unifree.exe` → Run as Administrator**
2. Go to **License** tab → Click **"Generate License"**
3. Go to **Hub** tab → Click **"Patch Hub"**
4. Unity Hub will launch automatically
5. To patch Editor: Go to **Editor** tab → Click **"Patch"** for each editor

## How It Works

### Hub Patching (JavaScript Method)

UniFree extracts `app.asar` and patches JavaScript files to bypass license validation:

| File | Patch |
|------|-------|
| `licenseService-*.js` | `isLicenseValid()` → return `true` |
| `licenseQueryService-*.js` | `isLicenseValid()` → return `true` |
| `licenseQueryService-*.js` | `getLicense()` → return fake Unity Pro data |
| `DefaultLocalConfig-*.js` | `DisableSignInRequired` → `true` |
| `DefaultLocalConfig-*.js` | `DisableAutoUpdate` → `true` |

### Editor Patching (DLL Replacement)

Replaces `Unity.Licensing.EntitlementResolver.dll` with a pre-patched version that bypasses `ValidateSignature`.

### License Generation

1. Collects hardware info (Windows Product ID, Disk Serial, BIOS Serial, MAC Address)
2. Generates ALF (Activation License File) with real machine bindings
3. Converts to ULF with dummy signature node
4. Writes to `C:\ProgramData\Unity\Unity_lic.ulf`

## What Gets Modified

| Component | File | Action |
|-----------|------|--------|
| Hub | `app.asar` | Extract to `app/`, patch JS, rename to `.bak` |
| Hub | `hubConfig.json` | Update sign-in and update settings |
| Editor | `Unity.Licensing.EntitlementResolver.dll` | Replace with pre-patched version |
| License | `C:\ProgramData\Unity\Unity_lic.ulf` | Generate and write license file |

## Build from Source

### Prerequisites

- Node.js 18+
- Rust 1.70+

### Build Steps

```bash
# Install dependencies
npm install

# Build frontend
npm run build

# Build Tauri app
cargo tauri build
```

## Disclaimer

**For educational purposes only.**

- This tool is for learning and understanding Unity's licensing mechanism
- Use at your own risk
- Consider using Unity Personal (free) or purchasing a legitimate license
- The author is not responsible for any misuse

## Credits

- [UniHacker](https://gitee.com/WitLau/UniHacker) - Original method inspiration
- [Tauri](https://tauri.app/) - Cross-platform desktop framework
- [React](https://react.dev/) - UI framework
- [Material-UI](https://mui.com/) - UI components

## License

MIT License - See [LICENSE](LICENSE) for details

---

**UniFree 2.2.0** - Unity License Freedom Tool
