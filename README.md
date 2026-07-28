# UniFree 2.5.0

[English](README.md) | [中文](README_CN.md)

> Unity Hub & Editor License Patcher

## Features

- **Unity Hub Patching** - Bypass license validation via JavaScript patching (UniHacker method)
- **Unity Editor Patching** - Version-aware patching to bypass signature verification
  - Unity 6000.7+: **Native AOT binary patching** — byte-level anchors bypass certificate chain, signature gate, and crypto verification
  - Unity 6000.0-6000.6: replaces `Unity.Licensing.EntitlementResolver.dll`
  - Unity 2019-2022: replaces `System.Security.Cryptography.Xml.dll`
- **License Generation** - Generate RSA-signed Unity Pro license files from hardware info
- **Custom Paths** - Support custom Hub and Editor scan directories
- **Editor Refresh** - Refresh button to re-scan installed editors
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

### Editor Patching

Version-aware patching that bypasses `ValidateSignature`:

| Unity Version | Target File | Method |
|---------------|------------|--------|
| **6000.7+** | `Unity.Licensing.Client.exe` | **Byte-level anchor-based patching** (4 patches: cert chain, signature gate, LABEL_14 check, BCrypt/NCrypt) |
| 6000.0-6000.6 | `Unity.Licensing.EntitlementResolver.dll` | Pre-patched DLL replacement |
| 2019-2022 | `System.Security.Cryptography.Xml.dll` | Pre-patched DLL replacement |

For 6000.7+, the binary is .NET 10 Native AOT compiled (no IL). Patches use pattern-matching anchors to locate and modify specific native instructions, providing cross-minor-version compatibility. See `docs/editor-dll-patching.md` for technical details.

### License Generation

1. Collects hardware info (Windows Product ID, Disk Serial, BIOS Serial, MAC Address)
2. Generates ALF (Activation License File) with real machine bindings and Pro features
3. Signs with RSA-SHA1 using a random private key
4. Writes to `C:\ProgramData\Unity\Unity_lic.ulf`

**Unity Pro Features:** 0, 2, 4, 9, 13, 20, 21, 22, 30, 39, 40, 60, 65

## What Gets Modified

| Component | File | Action |
|-----------|------|--------|
| Hub | `app.asar` | Extract to `app/`, patch JS, rename to `.bak` |
| Hub | `hubConfig.json` | Update sign-in and update settings |
| Editor (6000.7+) | `Unity.Licensing.Client.exe` | Byte-level binary patch (4 anchor-based patches) |
| Editor (6000.0-6000.6) | `Unity.Licensing.EntitlementResolver.dll` | Replace with pre-patched DLL |
| Editor (2019-2022) | `System.Security.Cryptography.Xml.dll` | Replace with pre-patched DLL |
| License | `C:\ProgramData\Unity\Unity_lic.ulf` | Generate RSA-signed license file |

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

**UniFree 2.5.0** - Unity License Freedom Tool

## Changelog

### v2.5.0
- **Native AOT binary patching for Unity 6000.7+** — byte-level anchor-based patches to bypass license signature verification in `Unity.Licensing.Client.exe`
- 4 precision patches: cert chain bypass, ValidateSignature gate skip, LABEL_14 cert check bypass, BCrypt/NCrypt NTSTATUS bypass
- Anchor-based pattern matching for cross-minor-version compatibility within the same Unity release line
- Automatic restore from backup before each patch, ensuring clean state

### v2.4.0
- Added auto-update: checks GitHub releases for new versions on startup
- Download progress bar with cancel support
- Silent NSIS installation after download

### v2.3.1
- Removed license type selection (Enterprise/Industrial/Plus) from UI
- Simplified to Unity Pro only - Unity Hub's Licensing Client does not support Enterprise/Industry types
- Removed custom RSA PEM key input (auto-generates random key)
- Replaced Hub's `System.Security.Cryptography.Xml.dll` for signature bypass
- Restored Hub's Licensing Client for proper license detection
