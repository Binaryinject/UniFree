use crate::commands::EditorInfo;
use crate::patcher;
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};

fn base_install_path() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        let pf = std::env::var("PROGRAMFILES").unwrap_or_else(|_| r"C:\Program Files".into());
        PathBuf::from(pf).join("Unity").join("Hub").join("Editor")
    }
    #[cfg(target_os = "macos")]
    { PathBuf::from("/Applications/Unity/Hub/Editor") }
    #[cfg(target_os = "linux")]
    {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/home".into());
        PathBuf::from(home).join("Unity").join("Hub").join("Editor")
    }
}

pub fn hub_app_data() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        let appdata = std::env::var("APPDATA").unwrap_or_else(|_| {
            let home = std::env::var("USERPROFILE").unwrap_or_else(|_| r"C:\Users\Public".into());
            PathBuf::from(home).join("AppData").join("Roaming").to_string_lossy().to_string()
        });
        PathBuf::from(appdata).join("UnityHub")
    }
    #[cfg(target_os = "macos")]
    {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/home".into());
        PathBuf::from(home).join("Library").join("Application Support").join("UnityHub")
    }
    #[cfg(target_os = "linux")]
    {
        let xdg = std::env::var("XDG_DATA_HOME").ok().map(PathBuf::from);
        let home = std::env::var("HOME").unwrap_or_else(|_| "/home".into());
        xdg.unwrap_or_else(|| PathBuf::from(home).join(".local").join("share")).join("unityhub")
    }
}

fn editor_exe_for_folder(folder: &PathBuf) -> PathBuf {
    #[cfg(target_os = "windows")]
    { folder.join("Editor").join("Unity.exe") }
    #[cfg(target_os = "macos")]
    { folder.join("Unity.app") }
    #[cfg(target_os = "linux")]
    { folder.join("Editor").join("Unity") }
}

/// Check if version uses Native AOT Unity.Licensing.Client.exe (>= 6000.7)
fn is_native_aot_editor(version: &str) -> bool {
    let normalized = version
        .split(|c: char| !c.is_ascii_digit() && c != '.')
        .find(|token| !token.is_empty() && token.chars().next().is_some_and(|c| c.is_ascii_digit()))
        .unwrap_or(version);
    let parts: Vec<&str> = normalized.split('.').collect();
    if parts.len() < 2 {
        return false;
    }
    let major: u32 = parts[0].parse().unwrap_or(0);
    let minor: u32 = parts[1].parse().unwrap_or(0);
    major == 6000 && minor >= 7
}

/// Get the target filename for patching based on Unity version
/// >= 6000.7: Unity.Licensing.Client.exe (Native AOT)
/// 6000.0-6000.6: Unity.Licensing.EntitlementResolver.dll
/// < 6000: System.Security.Cryptography.Xml.dll
fn target_file_name_for_version(version_folder: &str) -> &'static str {
    let normalized = version_folder
        .split(|c: char| !c.is_ascii_digit() && c != '.')
        .find(|token| !token.is_empty() && token.chars().next().is_some_and(|c| c.is_ascii_digit()))
        .unwrap_or(version_folder);
    if is_native_aot_editor(normalized) {
        #[cfg(target_os = "windows")]
        { "Unity.Licensing.Client.exe" }
        #[cfg(any(target_os = "macos", target_os = "linux"))]
        { "Unity.Licensing.Client" }
    } else if normalized.starts_with('6') {
        "Unity.Licensing.EntitlementResolver.dll"
    } else {
        "System.Security.Cryptography.Xml.dll"
    }
}

/// 编辑器自带 licensing client 所在目录（跨平台统一入口）
pub fn editor_licensing_dir(folder: &Path) -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        // Unity 6：Unity.app/Contents/Helpers/UnityLicensingClient.app/Contents/MacOS
        let helpers = folder
            .join("Unity.app")
            .join("Contents")
            .join("Helpers")
            .join("UnityLicensingClient.app")
            .join("Contents")
            .join("MacOS");
        if helpers.exists() {
            return helpers;
        }
        // 旧版（Unity 2021-2022）布局
        folder
            .join("Unity.app")
            .join("Contents")
            .join("Resources")
            .join("Licensing")
            .join("Client")
    }
    #[cfg(not(target_os = "macos"))]
    {
        folder
            .join("Editor")
            .join("Data")
            .join("Resources")
            .join("Licensing")
            .join("Client")
    }
}

fn dll_path_for_folder(folder: &PathBuf) -> PathBuf {
    let version = folder.file_name().unwrap_or_default().to_string_lossy();
    let target_name = target_file_name_for_version(&version);
    let client = editor_licensing_dir(folder);
    let preferred = client.join(target_name);
    if preferred.exists() {
        preferred
    } else if client.join("Unity.Licensing.Client.exe").exists() {
        client.join("Unity.Licensing.Client.exe")
    } else if client.join("Unity.Licensing.Client").exists() {
        client.join("Unity.Licensing.Client")
    } else if client.join("Unity.Licensing.EntitlementResolver.dll").exists() {
        client.join("Unity.Licensing.EntitlementResolver.dll")
    } else {
        client.join("System.Security.Cryptography.Xml.dll")
    }
}

fn read_product_name(folder: &PathBuf) -> String {
    let build_info = folder.join("Editor").join("buildInfo");
    if let Ok(content) = fs::read_to_string(&build_info) {
        for line in content.lines() {
            if let Some(val) = line.strip_prefix("ProductName:") {
                return val.trim().to_string();
            }
        }
    }
    let metadata = folder.join("metadata.hub.json");
    if let Ok(content) = fs::read_to_string(&metadata) {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(name) = json.get("productName").and_then(|v| v.as_str()) {
                return name.to_string();
            }
        }
    }
    "Unity".to_string()
}

#[cfg(target_os = "windows")]
fn read_unity_exe_version(exe: &PathBuf) -> Option<String> {
    use std::os::windows::ffi::OsStrExt;
    use windows::core::PCWSTR;
    use windows::Win32::Storage::FileSystem::{
        GetFileVersionInfoSizeW, GetFileVersionInfoW, VerQueryValueW,
    };

    let wide: Vec<u16> = exe.as_os_str().encode_wide().chain(std::iter::once(0)).collect();

    unsafe {
        let size = GetFileVersionInfoSizeW(PCWSTR(wide.as_ptr()), None);
        if size == 0 {
            return None;
        }
        let mut buf = vec![0u8; size as usize];
        if GetFileVersionInfoW(PCWSTR(wide.as_ptr()), 0, size, buf.as_mut_ptr() as *mut _).is_err() {
            return None;
        }

        // 读 Translation 以定位 StringFileInfo 的语言/代码页
        let make_key = |s: &str| -> Vec<u16> { s.encode_utf16().chain(std::iter::once(0)).collect() };
        let trans_key = make_key("\\VarFileInfo\\Translation");
        let mut trans_ptr: *mut std::ffi::c_void = std::ptr::null_mut();
        let mut trans_len = 0u32;
        if !VerQueryValueW(buf.as_ptr() as *const _, PCWSTR(trans_key.as_ptr()), &mut trans_ptr, &mut trans_len).as_bool() {
            return None;
        }
        if trans_ptr.is_null() || trans_len < 4 {
            return None;
        }
        let lang = *(trans_ptr as *const u16);
        let codepage = *((trans_ptr as *const u16).add(1));
        let product_key = make_key(&format!("\\StringFileInfo\\{lang:04x}{codepage:04x}\\ProductVersion"));

        let mut val_ptr: *mut std::ffi::c_void = std::ptr::null_mut();
        let mut val_len = 0u32;
        if !VerQueryValueW(buf.as_ptr() as *const _, PCWSTR(product_key.as_ptr()), &mut val_ptr, &mut val_len).as_bool() {
            return None;
        }
        if val_ptr.is_null() || val_len == 0 {
            return None;
        }
        let chars = std::slice::from_raw_parts(val_ptr as *const u16, (val_len as usize).saturating_sub(1));
        let version = String::from_utf16_lossy(chars);
        (!version.is_empty()).then_some(version)
    }
}

/// macOS：`Unity.app` 的版本号取安装目录名（如 `6000.3.21f1`），
/// 读不到时回退 `Info.plist` 的 `CFBundleVersion`。
#[cfg(target_os = "macos")]
fn read_unity_exe_version(app: &PathBuf) -> Option<String> {
    if let Some(name) = app.parent().and_then(|p| p.file_name()) {
        let version = name.to_string_lossy().to_string();
        if version.chars().next().is_some_and(|c| c.is_ascii_digit()) {
            return Some(version);
        }
    }
    let plist = app.join("Contents").join("Info.plist");
    let out = std::process::Command::new("/usr/libexec/PlistBuddy")
        .args(["-c", "Print :CFBundleVersion"])
        .arg(&plist)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let version = String::from_utf8_lossy(&out.stdout).trim().to_string();
    (!version.is_empty()).then_some(version)
}

#[cfg(target_os = "linux")]
fn read_unity_exe_version(_exe: &PathBuf) -> Option<String> { None }

/// 读取安装版本号：Windows 取 PE 的 `ProductVersion` 资源，
/// macOS 取 `.app` 包的 `CFBundleShortVersionString`。
/// 用于嗅探 Hub 版本（patcher 判定陈旧 `.bak` 备份）。
pub fn read_file_product_version(path: &std::path::Path) -> Option<String> {
    #[cfg(target_os = "windows")]
    {
        read_unity_exe_version(&path.to_path_buf())
    }
    #[cfg(target_os = "macos")]
    {
        // 传入的可能是 app 包内的可执行文件（如 Electron Framework），向上找 .app
        let mut current = Some(path);
        while let Some(p) = current {
            if p.extension().map(|e| e == "app").unwrap_or(false) {
                let out = std::process::Command::new("/usr/libexec/PlistBuddy")
                    .args(["-c", "Print :CFBundleShortVersionString"])
                    .arg(p.join("Contents").join("Info.plist"))
                    .output()
                    .ok()?;
                if !out.status.success() {
                    return None;
                }
                let version = String::from_utf8_lossy(&out.stdout).trim().to_string();
                return (!version.is_empty()).then_some(version);
            }
            current = p.parent();
        }
        None
    }
    #[cfg(target_os = "linux")]
    {
        read_unity_exe_version(&path.to_path_buf())
    }
}

fn read_architecture(folder: &PathBuf) -> String {
    let metadata = folder.join("metadata.hub.json");
    if let Ok(content) = fs::read_to_string(&metadata) {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(arch) = json.get("architecture").and_then(|v| v.as_str()) {
                return arch.to_string();
            }
        }
    }
    "x86_64".to_string()
}

fn scan_folder(base: &PathBuf) -> Vec<EditorInfo> {
    let mut editors = Vec::new();
    if !base.exists() {
        return editors;
    }
    let entries = match fs::read_dir(base) {
        Ok(e) => e,
        Err(_) => return editors,
    };
    for entry in entries.flatten() {
        let folder = entry.path();
        let exe = editor_exe_for_folder(&folder);
        if !exe.exists() {
            continue;
        }
        let version = match read_unity_exe_version(&exe) {
            Some(version) => version,
            None => continue,
        };
        let dll = dll_path_for_folder(&folder);
        let dll_status = if dll.exists() {
            patcher::get_editor_dll_status(dll.to_string_lossy().as_ref())
        } else {
            "not_found".into()
        };
        editors.push(EditorInfo {
            version,
            path: exe.to_string_lossy().to_string(),
            dll_path: dll.to_string_lossy().to_string(),
            dll_status,
            product_name: read_product_name(&folder),
            architecture: read_architecture(&folder),
        });
    }
    editors
}

#[derive(Deserialize)]
struct LocatedEditorData {
    data: Option<Vec<LocatedEditor>>,
}

#[derive(Deserialize)]
struct LocatedEditor {
    version: Option<String>,
    location: Option<serde_json::Value>,
    architecture: Option<String>,
}

fn read_located_editors() -> Vec<EditorInfo> {
    let mut editors = Vec::new();
    let app_data = hub_app_data();

    // Try editors-v2.json first, fallback to editors.json
    for key in &["editors-v2.json", "editors.json"] {
        let path = app_data.join(key);
        if !path.exists() { continue; }
        let content = match fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        if *key == "editors-v2.json" {
            if let Ok(parsed) = serde_json::from_str::<LocatedEditorData>(&content) {
                if let Some(list) = parsed.data {
                    for e in list {
                        if let Some(info) = parse_located_editor(&e) {
                            if !editors.iter().any(|existing: &EditorInfo| existing.dll_path == info.dll_path) {
                                editors.push(info);
                            }
                        }
                    }
                }
            }
        } else {
            // Legacy format: { "version": { version, location, ... }, ... }
            if let Ok(map) = serde_json::from_str::<serde_json::Value>(&content) {
                if let Some(obj) = map.as_object() {
                    for (_, val) in obj {
                        let version = val.get("version").and_then(|v| v.as_str()).unwrap_or("").to_string();
                        let location = val.get("location").cloned();
                        let arch = val.get("architecture").and_then(|v| v.as_str()).unwrap_or("x86_64").to_string();
                        if let Some(info) = build_editor_info(&version, location, &arch) {
                            if !editors.iter().any(|e: &EditorInfo| e.dll_path == info.dll_path) {
                                editors.push(info);
                            }
                        }
                    }
                }
            }
        }
        if !editors.is_empty() { break; }
    }
    editors
}

fn parse_located_editor(editor: &LocatedEditor) -> Option<EditorInfo> {
    let version = editor.version.as_deref()?;
    let location = editor.location.as_ref()?;
    let arch = editor.architecture.as_deref().unwrap_or("x86_64");
    build_editor_info(version, Some(location.clone()), arch)
}

fn build_editor_info(version: &str, location: Option<serde_json::Value>, arch: &str) -> Option<EditorInfo> {
    if version.is_empty() { return None; }

    // location can be a string or array of strings
    let exe_path = match location {
        Some(serde_json::Value::String(s)) => PathBuf::from(&s),
        Some(serde_json::Value::Array(arr)) => {
            if let Some(s) = arr.first().and_then(|v| v.as_str()) {
                PathBuf::from(s)
            } else {
                return None;
            }
        }
        _ => return None,
    };

    if !exe_path.exists() { return None; }

    // Derive DLL path from exe path
    let dll = derive_dll_from_exe(&exe_path);
    let dll_status = if dll.exists() {
        patcher::get_editor_dll_status(dll.to_string_lossy().as_ref())
    } else {
        "not_found".into()
    };

    let folder = exe_path.parent().map(|p| p.to_path_buf()).unwrap_or_default();
    Some(EditorInfo {
        version: version.to_string(),
        path: exe_path.to_string_lossy().to_string(),
        dll_path: dll.to_string_lossy().to_string(),
        dll_status,
        product_name: read_product_name(&folder),
        architecture: arch.to_string(),
    })
}

fn derive_dll_from_exe(exe: &PathBuf) -> PathBuf {
    // Windows/Linux: .../<version>/Editor/Unity(.exe)  → <version>
    // macOS:         .../<version>/Unity.app            → <version>
    let Some(parent) = exe.parent() else {
        return PathBuf::new();
    };
    #[cfg(target_os = "macos")]
    let folder = parent.to_path_buf();
    #[cfg(not(target_os = "macos"))]
    let folder = if parent.file_name().map(|n| n == "Editor").unwrap_or(false) {
        parent.parent().map(|p| p.to_path_buf()).unwrap_or_else(|| parent.to_path_buf())
    } else {
        parent.to_path_buf()
    };
    if !folder.exists() {
        return PathBuf::new();
    }
    dll_path_for_folder(&folder)
}

fn read_secondary_install_path() -> Option<PathBuf> {
    let app_data = hub_app_data();
    let path = app_data.join("secondaryInstallPath.json");
    if let Ok(content) = fs::read_to_string(&path) {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(p) = json.as_str().map(PathBuf::from) {
                if p.exists() { return Some(p); }
            }
        }
    }
    None
}

/// Unity Hub 安装位置：macOS 为 `.app` 包目录，Windows 为含 `Unity Hub.exe` 的目录。
pub fn hub_install_dir() -> PathBuf {
    if let Some(custom_path) = crate::app_config::get_hub_install_path() {
        return normalize_hub_install_dir(custom_path);
    }
    #[cfg(target_os = "windows")]
    {
        let pf = std::env::var("PROGRAMFILES").unwrap_or_else(|_| r"C:\Program Files".into());
        PathBuf::from(pf).join("Unity Hub")
    }
    #[cfg(target_os = "macos")]
    {
        PathBuf::from("/Applications/Unity Hub.app")
    }
    #[cfg(target_os = "linux")]
    {
        PathBuf::from("/usr/share/unityhub")
    }
}

/// 用户自定义 Hub 路径归一化：macOS 允许选中 `Unity Hub.app` 或其父目录。
fn normalize_hub_install_dir(path: PathBuf) -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        if path.extension().map(|e| e == "app").unwrap_or(false) {
            return path;
        }
        let app = path.join("Unity Hub.app");
        if app.exists() {
            return app;
        }
    }
    path
}

pub fn hub_resources_path() -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        hub_install_dir().join("Contents").join("Resources")
    }
    #[cfg(not(target_os = "macos"))]
    {
        hub_install_dir().join("resources")
    }
}

/// Hub 自带 licensing client 的目录（含 `Unity.Licensing.EntitlementResolver.dll`）。
pub fn hub_licensing_dir() -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        hub_install_dir()
            .join("Contents")
            .join("Frameworks")
            .join("UnityLicensingClient_V1.app")
            .join("Contents")
            .join("MacOS")
    }
    #[cfg(not(target_os = "macos"))]
    {
        hub_install_dir().join("UnityLicensingClient_V1")
    }
}

/// 将 editors 按版本+架构去重后追加到 all
fn dedup_extend(
    editors: Vec<EditorInfo>,
    all: &mut Vec<EditorInfo>,
    seen: &mut std::collections::HashSet<String>,
) {
    for e in editors {
        // 以 dll 绝对路径作为唯一键：同版本不同安装位置的编辑器应被分别保留
        let key = e.dll_path.clone();
        if seen.insert(key) {
            all.push(e);
        }
    }
}

pub fn scan_installed_editors() -> Vec<EditorInfo> {
    let mut all: Vec<EditorInfo> = Vec::new();
    let mut seen = std::collections::HashSet::new();

    // 1. Base install path
    dedup_extend(scan_folder(&base_install_path()), &mut all, &mut seen);

    // 2. Secondary install path
    if let Some(secondary) = read_secondary_install_path() {
        dedup_extend(scan_folder(&secondary), &mut all, &mut seen);
    }

    // 3. Manually located editors from JSON storage
    dedup_extend(read_located_editors(), &mut all, &mut seen);

    // 4. Custom user-added scan paths
    for custom_path in crate::app_config::get_editor_scan_paths() {
        let path = PathBuf::from(&custom_path);
        if !path.exists() {
            continue;
        }
        // Try as a direct version folder (e.g., D:\Unity\2022.3.1f1)
        let exe = editor_exe_for_folder(&path);
        if exe.exists() {
            if let Some(version) = read_unity_exe_version(&exe) {
                let dll = dll_path_for_folder(&path);
                let dll_status = if dll.exists() {
                    patcher::get_editor_dll_status(dll.to_string_lossy().as_ref())
                } else {
                    "not_found".into()
                };
                let info = EditorInfo {
                    version,
                    path: exe.to_string_lossy().to_string(),
                    dll_path: dll.to_string_lossy().to_string(),
                    dll_status,
                    product_name: read_product_name(&path),
                    architecture: read_architecture(&path),
                };
                dedup_extend(vec![info], &mut all, &mut seen);
            }
        }
        // Also scan as a parent directory containing version folders
        dedup_extend(scan_folder(&path), &mut all, &mut seen);
    }

    all
}
