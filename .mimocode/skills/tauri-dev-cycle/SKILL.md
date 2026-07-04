---
name: tauri-dev-cycle
description: >
  Tauri + Rust development compile-check-fix cycle for the UniFree project.
  Use when editing Rust source files (src-tauri/src/*.rs), adding Cargo
  dependencies, or fixing compilation errors/warnings in the Tauri backend.
---

# Tauri Development Cycle (UniFree)

## Project structure

```
C:\GIT\UniFree\
├── src-tauri/
│   ├── Cargo.toml          # Rust dependencies
│   ├── tauri.conf.json     # Tauri config
│   ├── src/
│   │   ├── main.rs         # Entry point
│   │   ├── lib.rs          # Module declarations + Tauri builder
│   │   ├── commands.rs     # Tauri command handlers
│   │   ├── patcher.rs      # ASAR patching logic
│   │   ├── config_patcher.rs # Hub config modifications
│   │   ├── license.rs      # ULF copy/status
│   │   ├── license_automator.rs # LicenseGenerate.exe GUI automation
│   │   ├── alf_generator.rs # ALF file generation
│   │   └── il_patcher.rs   # .NET IL patching
│   └── resources/           # Bundled binaries (LicenseGenerate.exe, etc.)
├── src/                     # React/TypeScript frontend
├── package.json
└── vite.config.ts
```

## Compile-check cycle

```bash
# Quick check (preferred for iterative development)
cd C:/GIT/UniFree/src-tauri && cargo check 2>&1 | grep -E "error:|Finished"

# Check with warnings
cd C:/GIT/UniFree/src-tauri && cargo check 2>&1 | grep -E "error:|warning: unused|Finished"

# Full build
cd C:/GIT/UniFree/src-tauri && cargo build 2>&1 | tail -20

# Release build
cd C:/GIT/UniFree/src-tauri && cargo build --release 2>&1 | tail -30
```

## Adding new modules

1. Create the `.rs` file in `src-tauri/src/`
2. Add `mod module_name;` to `lib.rs`
3. If it exposes Tauri commands, add `invoke_handler` entry in `lib.rs`
4. Add `use crate::module_name;` to `commands.rs` if needed
5. Run `cargo check` to verify

## Adding Cargo dependencies

Edit `src-tauri/Cargo.toml`:

```toml
# Windows-only dependency
[target.'cfg(windows)'.dependencies]
windows = { version = "0.58", features = ["Win32_UI_WindowsAndMessaging", "Win32_Foundation"] }

# Cross-platform
[dependencies]
sha1 = "0.10"
```

## Common fixes

### Dead code warnings
```bash
# Add #[allow(dead_code)] to unused constants/functions
# Or remove unused code entirely
```

### Module not found
```bash
# Check lib.rs has the mod declaration
grep -n "^mod " src-tauri/src/lib.rs
```

### Windows API feature flags
```toml
# Common Windows features needed:
# Win32_UI_WindowsAndMessaging - window manipulation
# Win32_Foundation - HWND, BOOL types
# Win32_UI_Input_KeyboardAndMouse - SendInput
# Win32_System_LibraryLoader - LoadLibrary
```

## Tauri dev server

```bash
# Start dev server (background)
npm run tauri dev

# Check if running
Get-NetTCPConnection -LocalPort 1420,1421 -ErrorAction SilentlyContinue

# Kill dev server
Get-Process | Where-Object { $_.ProcessName -like "*tauri*" } | Stop-Process
```
