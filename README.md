# UniFree 2.8.1

[English](README.md) | [中文](README_CN.md)

> Unity Hub & Editor License Patcher

## Features

- **Unity Hub Patching** - Bypass license validation via JavaScript patching (UniHacker method)
- **Unity Editor Patching** - Version-aware patching to bypass signature verification
  - Unity 6000.7+: **Native AOT binary patching** — 1 byte-level anchor short-circuits `ValidateSignature` entirely
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

UniFree rebuilds `app.asar` in-place (preserving `app.asar.unpacked/` native modules) and patches JavaScript files to bypass license validation. It also flips the `EnableEmbeddedAsarIntegrityValidation` fuse in the Hub executable, otherwise modifying the asar crashes the Hub on startup:

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
| **6000.7+** | `Unity.Licensing.Client.exe` | **Byte-level anchor-based patching** (1 patch: `ValidateSignature` wrapper → always valid) |
| 6000.0-6000.6 | `Unity.Licensing.EntitlementResolver.dll` | Pre-patched DLL replacement |
| 2019.x | `Unity.exe` + `System.Security.Cryptography.Xml.dll` | Native anchor patch (`ValidateServerProcess` → always valid) + pre-patched DLL replacement |
| 2020-2022 | `System.Security.Cryptography.Xml.dll` | Pre-patched DLL replacement |

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
| Hub | `app.asar` | Rebuild in-place (preserve unpacked markers), patch `getLicense`/`isLicenseValid` |
| Hub | `Unity Hub.exe` | Flip Electron `EnableEmbeddedAsarIntegrityValidation` fuse |
| Hub | `UnityLicensingClient_V1\Unity.Licensing.EntitlementResolver.dll` | Replace with pre-patched DLL (v2.8.1, 1.17.x line) |
| Hub | `hubConfig.json` | Update sign-in and update settings |
| Editor (6000.7+) | `Unity.Licensing.Client.exe` | Byte-level binary patch (1 anchor-based patch) |
| Editor (6000.0-6000.6) | `Unity.Licensing.EntitlementResolver.dll` | Replace with pre-patched DLL (per 1.17.x / 1.18+ line, v2.8.1) |
| Editor (2019.x) | `Unity.exe` + `System.Security.Cryptography.Xml.dll` | Native anchor patch + replace with pre-patched DLL |
| Editor (2020-2022) | `System.Security.Cryptography.Xml.dll` | Replace with pre-patched DLL |
| License | `C:\ProgramData\Unity\Unity_lic.ulf` | Generate RSA-signed license file |

## Build from Source

### Prerequisites

- Node.js 18+
- Rust 1.85+

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

**UniFree 2.8.1** - Unity License Freedom Tool

## Changelog

### v2.8.1
- **Fixed "No valid Unity Editor license found." after patching 6000.3.10f1 (LocalIPC 1.17.x)**:
  - Pre-patched resolvers are now selected per licensing-client release line: 1.17.x (e.g. 6000.3.10f1,
    original assembly version 1.17.4.0, v7 references) uses a patched DLL generated from the 1.17.4
    original with the assembly version preserved (`Unity.Licensing.EntitlementResolver.1.17.4.dll`);
    1.18+ (e.g. 6000.3.20f1/22f1, 0.0.0.0, v8 references) keeps the bundled one. The old 0.0.0.0
    patched DLL crashed the 1.17.4 client at startup (deps version mismatch:
    `Could not load file or assembly '…, Version=1.17.4.0'`).
  - **The Hub's licensing client resolver is patched as well** (`Unity.Licensing.EntitlementResolver.hub.1.17.4.dll`):
    editors launched from the Hub now pass the ULF signature check through the Hub's licensing client
    (global pipe `LicenseClient-wbn`) — **no standalone IPC process needed, no UniFree resident;
    patch once and it survives restarts**. 1.18+ editors' own licensing clients are unaffected
    (separate folder/process/versioned pipe; the Hub client still answers them with
    `505 Unsupported protocol version`).
  - Added `tools/patchresolver`: one-click generation of a same-line patched resolver from any
    1.17.x original.
  - See `docs/editor-dll-patching.md` for details.

### v2.8.0
- **Unity 2019.4 native patch**: added two anchor-based byte patches to `Unity.exe` for 2019.x editors:
  - `ValidateServerProcess` bypass — the licensing client's Authenticode signature check (whose code-signing cert expired 2024-07-19) no longer rejects the licensing client.
  - `LICENSE SYSTEM` error dispatcher bypass — the native `WinILicensingAdapter` no longer reports "Unity license information is invalid.".
- The editor patch flow now applies these native patches before replacing `System.Security.Cryptography.Xml.dll`; restore reverts `Unity.exe` from its backup.
- Known issue (2019.4): after the validity gates pass, the editor still reports `License is not active (com.unity.editor.ui)` because the generated ULF uses the legacy `<Features>` format while the licensing client resolves the newer `<EntitlementGroups>` format. See `docs/editor-dll-patching.md`.

### v2.7.0
- Added language toggle (Chinese/English) with persistence across restarts.
- Auto-scroll log panel to the latest entry (only when already at the bottom).
- Lazy-load tabs and split vendor chunks to reduce bundle size.
- Replaced PowerShell/wmic/tasklist with Win32 APIs for faster scanning and better compatibility (fixes `wmic` removal in Windows 11 24H2+).
- Editor DLL status now detected by content comparison instead of a file-size heuristic.
- Hub config status now reflects the actual `hubConfig.json` state.
- Added a Content Security Policy.
- Repository cleanup: removed build artifacts from git history.

### v2.6.1
- Detect Unity Editor versions from `Unity.exe` file metadata instead of installation folder names.
- Support renamed and non-Hub Unity installations, including Unity 6000.3.x layouts.
- Select licensing targets by their actual filename while preserving the exact detected version in the UI.

### v2.6.0
- **Unity Hub 3.20.0+ support**: Hub now ships Electron with `OnlyLoadAppFromAsar` + `EnableEmbeddedAsarIntegrityValidation` fuses, breaking both the old "extract to `app/`" and any asar modification (integrity FATAL).
  - `flip_hub_exe_fuses()` disables the asar-integrity fuse in `Unity Hub.exe` (Electron fuse wire: `[magic][ver=01][len=09][ASCII bits]`, fuse 4).
  - `rewrite_hub_asar()` rebuilds `app.asar` in-place while preserving the 10 `unpacked` native-module markers (AsarWriter would drop them → `Cannot find native binding`).
  - Patches `licenseQueryService.getLicense()` to return a fake Unity Pro ULF so the Hub **displays** the license, plus `isLicenseValid()` → `true`.
- Hub licensing client (managed .NET, `EntitlementResolver.dll` 1.17.4, same version line as 6000.3.10f1-class editors) is a different architecture from Editor 6000.7+ Native AOT; since v2.8.1 the Hub's licensing client resolver is also replaced with a pre-patched version, so editors launched from the Hub pass the ULF signature check through the Hub's licensing client (global pipe `LicenseClient-wbn`) — no standalone IPC process needed, patch once and it survives restarts (see `docs/editor-dll-patching.md`).

### v2.5.7
- **6000.7+ patch simplified to a single byte-level patch**: short-circuits the `ValidateSignature` wrapper (`sub_1404F1C10`) with `mov eax,1; ret`, replacing the previous 2-patch (signature gate + LABEL_14 trust check) and the original 4-patch approach
- IDA-verified: the wrapper's return value is ignored by the parser (failure throws), so one function-head patch bypasses the entire validation chain while keeping the RSA-signed ULF unchanged
- Anchor now targets the config-field reads + dual type checks; cross-minor-version compatible within the same release line
- Regression test applies the patch to a real binary copy and asserts exactly one write site

### v2.5.6
- Fix updater: batch file polls PID before install, then restarts

### v2.5.5
- Fix updater: raw process restart after install

### v2.5.4
- Fix updater: run NSIS installer in-process then restart

### v2.5.3
- Fix updater: use PowerShell script for reliable silent install + restart (handles paths with spaces correctly)

### v2.5.2
- Fix updater: use `ping` delay + `start /wait` for reliable silent install and auto-restart
- Prevents race condition where NSIS installer returns before installation completes

### v2.5.0
- **Native AOT binary patching for Unity 6000.7+** — byte-level anchor-based patches to bypass license signature verification in `Unity.Licensing.Client.exe`
- 4 precision patches: cert chain bypass, ValidateSignature gate skip, LABEL_14 cert check bypass, BCrypt/NCrypt NTSTATUS bypass
- Anchor-based pattern matching for cross-minor-version compatibility within the same Unity release line

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
