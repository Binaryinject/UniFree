# UniFree 2.9.4

[English](README.md) | [中文](README_CN.md)

> Unity Hub & Editor License Patcher

## Features

- **Unity Hub Patching** - Bypass license validation via JavaScript patching (UniHacker method)
- **Unity Editor Patching** - Version-aware patching to bypass signature verification
  - Unity 6000.7+: **Native AOT binary patching** — 1 byte-level anchor short-circuits `ValidateSignature` entirely
  - Unity 6000.0-6000.6: **in-place IL byte patch** of `Unity.Licensing.EntitlementResolver.dll` (any LocalIPC line, ReadyToRun disabled)
  - Unity 2019-2022: replaces `System.Security.Cryptography.Xml.dll`
- **License Generation** - Generate RSA-signed Unity Pro license files from hardware info
- **Custom Paths** - Support custom Hub and Editor scan directories
- **Editor Refresh** - Refresh button to re-scan installed editors
- **Modern GUI** - Built with Tauri 2.0 + React + Material-UI
- **i18n Support** - Chinese & English
- **Automatic Backup** - All modifications are reversible
- **macOS (Apple Silicon)** - Native arm64 build. Unity 6 editors are patched either with the in-place IL patch
  (managed licensing clients) or the **Native AOT anchor** (6000.3.28+); the editor's Licensing-Client code-signature
  check is bypassed and the whole `Unity.app` is re-signed ad-hoc. Covers Unity Hub on macOS too.

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
| 6000.0-6000.6 | `Unity.Licensing.EntitlementResolver.dll` | In-place IL byte patch (any LocalIPC line) · falls back to pre-patched DLL per line |
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
| Hub | `UnityLicensingClient_V1\Unity.Licensing.EntitlementResolver.dll` | In-place IL byte patch (any LocalIPC line; 1.17.x / 1.18+ / future) |
| Hub | `hubConfig.json` | Update sign-in and update settings |
| Editor (6000.7+) | `Unity.Licensing.Client.exe` | Byte-level binary patch (1 anchor-based patch) |
| Editor (6000.0-6000.6) | `Unity.Licensing.EntitlementResolver.dll` | In-place IL byte patch (any LocalIPC line) · falls back to pre-patched DLL per line |
| Editor (2019.x) | `Unity.exe` + `System.Security.Cryptography.Xml.dll` | Native anchor patch + replace with pre-patched DLL |
| Editor (2020-2022) | `System.Security.Cryptography.Xml.dll` | Replace with pre-patched DLL |
| License | `C:\ProgramData\Unity\Unity_lic.ulf` | Generate RSA-signed license file |

## macOS (Apple Silicon)

macOS builds target **Apple Silicon (M-series / arm64) only** — Intel Macs are not supported.

Scope of the current macOS support: **Unity 6 editors with a managed licensing client**
(6000.0 – 6000.3.27 and other builds that still ship `Unity.Licensing.EntitlementResolver.dll`)
and Unity Hub.
Older editors (2019–2022) rely on Windows DLL replacement and are not covered on macOS yet.
Newer builds (e.g. 6000.3.28+) ship a **Native AOT** licensing client — a single arm64 Mach-O with
no managed DLLs. Those are patched too: the `ValidateSignature` equivalent is short-circuited with
`mov w0,#1; ret`, and the editor's client code-signature check is bypassed (`cmp w0,#9; b.ne` → `nop`),
then the whole `Unity.app` is re-signed ad-hoc.

What the macOS patch does:

| Component | File | Action |
|-----------|------|--------|
| Hub | `Unity Hub.app/Contents/Resources/app.asar` | In-place asar rewrite (same JS patches as Windows) |
| Hub | `Contents/Frameworks/Electron Framework.framework/Electron Framework` | Flip `EnableEmbeddedAsarIntegrityValidation` fuse to `0` |
| Hub | `Contents/Frameworks/UnityLicensingClient_V1.app/Contents/MacOS/Unity.Licensing.EntitlementResolver.dll` | In-place IL byte patch (`ValidateSignature` → always pass) |
| Hub / Editor | the `.app` bundle itself | `codesign --force --deep --sign -` (ad-hoc) after patching, otherwise macOS refuses to load the modified Mach-O |
| Editor | `Unity.app/Contents/Helpers/UnityLicensingClient.app/Contents/MacOS/Unity.Licensing.EntitlementResolver.dll` | In-place IL byte patch (managed DLL, identical to the Windows one) |
| License | `/Library/Application Support/Unity/Unity_lic.ulf` | Generated locally; writing there triggers a single admin prompt (`osascript ... with administrator privileges`) |

Notes:

- The resolver DLL is a platform-independent managed assembly, so the same IL patch works on macOS and Windows.
- Re-signing is ad-hoc (`codesign -`), no Apple Developer certificate is required. Gatekeeper may still
  prompt on first launch of a patched Hub; `xattr -dr com.apple.quarantine` is applied automatically.
- Everything is backed up (`*.bak`) and can be reverted from the app.

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

# Build Tauri app (Windows: NSIS / macOS: DMG, arm64)
cargo tauri build

# macOS arm64 explicitly
cargo tauri build --target aarch64-apple-darwin
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

**UniFree 2.9.4** - Unity License Freedom Tool

## Changelog

### v2.9.4
- **macOS release pipeline fixed**: GitHub rejects arm64 hosted runners for this account
  (`configuration 'macos-14-arm64-us-default' is not supported`). The macOS job now runs on `macos-13` and
  cross-compiles `aarch64-apple-darwin`, producing the same arm64 DMG.

### v2.9.3
- **Crisp app icon**: the icon master was only 32×32, so the Dock icon was upscaled and blurry. Rebuilt a
  1024×1024 master (supersampled) and regenerated the full iconset — `icon.icns` (16…1024), all PNG sizes,
  Windows `.ico`, and `app-icon.png`.

### v2.9.2
- **macOS AOT restore status fixed**: restoring left the out-of-bundle backups in place, and the status check
  treats "backup exists" as *patched*, so the button stayed on "Restore". Backups are now removed on restore,
  and the AOT patch state is detected from the 16-byte tail anchor instead of the (overwritten) entry anchor.

### v2.9.1
- **Unity 6000.3.28+ (macOS, Native AOT licensing client) is now patched**:
  - client: a 24-byte unique anchor at the `ValidateSignature` equivalent is rewritten to `mov w0,#1; ret`,
    so ULF and entitlement XML signatures are accepted;
  - editor: the Licensing-Client code-signature check (`Code 8 while verifying Licensing Client signature`)
    is bypassed by locating the format string → `adrp/add` xref → `cmp w0,#9` / `b.ne` → `nop`;
  - both patches are followed by a single ad-hoc `codesign --force --deep` of `Unity.app` (stepwise signing
    left stale seals and `invalid signature`).
  - Verified on 6000.3.28f1 arm64: exit code 198 → 0, `License group: Product: Unity Pro`,
    `Successfully parsed (ULF) license`.

### v2.9.0
- **macOS (Apple Silicon) support**: native arm64 `app`/`dmg` build, macOS Hub patching (asar + Electron fuse in
  `Electron Framework` + ad-hoc re-sign), managed-client IL patch with `DOTNET_ReadyToRun=0` launcher shim,
  macOS license generation (`/Library/Application Support/Unity/Unity_lic.ulf`, one admin prompt),
  backups stored outside the `.app` bundle.

### v2.8.3
- **Hub 3.22.x entitlement-verdict gate covered**: Hub 3.22.0 (HUB-7120) replaced the boolean licence check with a
  tri-state verdict (`ENTITLED` / `NOT_ENTITLED` / `UNVERIFIED`) served over `licenses/getEditorEntitlementDecision`.
  `LicenseQueryService.checkEditorEntitlement()` and `checkUlfLicense()` now call the licensing client directly
  (`checkEntitlements([com.unity.editor.ui])` / `getAllEntitlementGroups()`), so patching only `isLicenseValid()`
  left the Hub's UI blocking paths and the project-open gate reading the client's real answer. Both verdict
  producers are now patched to `ENTITLED` as well (`getLicense()` / `isLicenseValid()` patches unchanged).
- **Patch state is detected by content, not by leftover `.bak` files**: the installer/uninstaller never removes
  `app.asar.bak` / `*.dll.bak` / `Unity Hub.exe.bak`, so after a Hub reinstall or update the tool kept reporting
  "patched" while the installer had already restored the pristine `app.asar` (that is what produced the
  "no licence shown after restart, projects still open" report). Status now checks three independent points —
  the in-asar JS marker, the resolver IL patch state, and the asar-integrity fuse byte — and reports
  `patched` / `partial` / `original` accordingly.
- **Stale backup guard**: before restoring from `app.asar.bak`, its embedded `package.json` version is compared with
  the installed Hub's `ProductVersion`; a backup from a different Hub version is discarded instead of being copied
  into the new install (which used to mix an old version's JS/file table into the new Hub).
- **Sign-in bypass now covers the second default-config copy in 3.22**: patching only `DefaultLocalConfig-*.js`
  left the editor-install container window (`installerContainerSetup-*.js` carries its own `DefaultLocalConfig`)
  demanding a sign-in again. `DisableSignInRequired` / `DisableAutoUpdate` are now matched by content in any JS
  file, and `tools/patch-hub-js.mjs` mirrors it together with the `hubConfig.json` keys written by
  `config_patcher`.
- "Restore original" is also offered for the `partial` state.

### v2.8.2
- **In-place IL byte patch replaces pre-patched resolver DLLs (6000.0-6000.6 & Hub)**: instead of swapping in a
  rebuild-per-line DLL, the tool now rewrites `ValidateSignature`'s IL directly in the original file —
  `brtrue.s`+`ldstr`+`newobj`+`throw` → `pop`+12×`nop` (13-byte, stack-equivalent) — preserving the
  assembly identity, so **any LocalIPC release line works out of the box** (1.17.x, 1.18+, future lines),
  no longer requiring per-line prebuilt binaries. Pre-patched DLLs remain as an automatic fallback.
- **ReadyToRun fix (critical)**: Unity's resolver is a ReadyToRun image (`RTR`, ManagedNativeHeader). The CLI
  header's `ManagedNativeHeader` is now zeroed during the patch, otherwise the runtime keeps executing the
  stale native code and the IL patch silently never takes effect (dnlib validation passes, runtime behavior
  doesn't). Verified end-to-end: the licensing client logs
  `Successfully parsed (ULF) license ...`, `Processed 2 license files` with a UniFree-signed ULF.
- **Unity Hub 3.21.1 support (LocalIPC 1.18.3)**: `patch_hub_licensing_client()` now patches the Hub's own
  licensing client for the 1.18+ line too (in-place first, per-line fallback). Fixes editors launched from
  the Hub failing with exit code 198 (`No valid Unity Editor license found.`) when the Hub client protocol
  matches the editor's 1.18.3 client — previously only the 1.17.x line was supported.
- Added `tools/verify-resolver` (dnlib structural validation of patched resolvers) and
  `tools/resolver-invoke` (direct reflection invocation of `XmlExtensions.ValidateSignature`).
- See `docs/editor-dll-patching.md` for details.

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
- Hub licensing client (managed .NET, `EntitlementResolver.dll`) is a different architecture from Editor 6000.7+ Native AOT; since v2.8.1/2.8.2 the Hub's licensing client resolver is patched in-place (any LocalIPC line), so editors launched from the Hub pass the ULF signature check through the Hub's licensing client (global pipe `LicenseClient-wbn`) — no standalone IPC process needed, patch once and it survives restarts (see `docs/editor-dll-patching.md`).

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
