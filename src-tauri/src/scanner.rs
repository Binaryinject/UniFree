use crate::commands::EditorInfo;
use crate::patcher;
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

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

fn hub_app_data() -> PathBuf {
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

fn is_version_folder(name: &str) -> bool {
    let bytes = name.as_bytes();
    if bytes.is_empty() || !bytes[0].is_ascii_digit() {
        return false;
    }
    let mut dots = 0;
    let mut digits_before_dot = 0;
    let mut has_channel = false;
    for &b in bytes {
        if b == b'.' {
            if digits_before_dot == 0 { return false; }
            dots += 1;
            digits_before_dot = 0;
        } else if b.is_ascii_digit() {
            digits_before_dot += 1;
        } else if matches!(b, b'a' | b'b' | b'f' | b'p' | b'c') {
            has_channel = true;
            break;
        } else if b == b'-' {
            break;
        } else {
            return false;
        }
    }
    dots >= 2 && has_channel
}

fn editor_exe_for_folder(folder: &PathBuf) -> PathBuf {
    #[cfg(target_os = "windows")]
    { folder.join("Editor").join("Unity.exe") }
    #[cfg(target_os = "macos")]
    { folder.join("Unity.app") }
    #[cfg(target_os = "linux")]
    { folder.join("Editor").join("Unity") }
}

fn dll_path_for_folder(folder: &PathBuf) -> PathBuf {
    #[cfg(target_os = "windows")]
    { folder.join("Editor").join("Data").join("Resources").join("Licensing").join("Client").join("Unity.Licensing.EntitlementResolver.dll") }
    #[cfg(target_os = "macos")]
    { folder.join("Contents").join("Resources").join("Licensing").join("Client").join("Unity.Licensing.EntitlementResolver.dll") }
    #[cfg(target_os = "linux")]
    { folder.join("Editor").join("Data").join("Resources").join("Licensing").join("Client").join("Unity.Licensing.EntitlementResolver.dll") }
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
        let name = entry.file_name().to_string_lossy().to_string();
        if !is_version_folder(&name) {
            continue;
        }
        let folder = entry.path();
        let exe = editor_exe_for_folder(&folder);
        if !exe.exists() {
            continue;
        }
        let dll = dll_path_for_folder(&folder);
        let dll_status = if dll.exists() {
            patcher::get_editor_dll_status(dll.to_string_lossy().as_ref())
        } else {
            "not_found".into()
        };
        editors.push(EditorInfo {
            version: name,
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
                            if !editors.iter().any(|existing: &EditorInfo| existing.version == info.version && existing.architecture == info.architecture) {
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
                            if !editors.iter().any(|e: &EditorInfo| e.version == info.version && e.architecture == info.architecture) {
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
    // exe is like: ...\2022.3.1f1\Editor\Unity.exe
    // dll is:      ...\2022.3.1f1\Editor\Data\Resources\Licensing\Client\Unity.Licensing.EntitlementResolver.dll
    if let Some(editor_dir) = exe.parent() {
        #[cfg(target_os = "windows")]
        { editor_dir.join("Data").join("Resources").join("Licensing").join("Client").join("Unity.Licensing.EntitlementResolver.dll") }
        #[cfg(target_os = "macos")]
        {
            // For macOS: exe is inside Unity.app/Contents/MacOS/Unity
            // dll is at Unity.app/Contents/Resources/Licensing/Client/...
            if let Some(app_dir) = editor_dir.parent().and_then(|p| p.parent()) {
                app_dir.join("Contents").join("Resources").join("Licensing").join("Client").join("Unity.Licensing.EntitlementResolver.dll")
            } else {
                PathBuf::new()
            }
        }
        #[cfg(target_os = "linux")]
        { editor_dir.join("Data").join("Resources").join("Licensing").join("Client").join("Unity.Licensing.EntitlementResolver.dll") }
    } else {
        PathBuf::new()
    }
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

fn hub_licensing_dir() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        let pf = std::env::var("PROGRAMFILES").unwrap_or_else(|_| r"C:\Program Files".into());
        PathBuf::from(pf).join("Unity Hub").join("UnityLicensingClient_V1")
    }
    #[cfg(target_os = "macos")]
    {
        PathBuf::from("/Applications/Unity Hub.app/Contents/Resources").join("UnityLicensingClient_V1")
    }
    #[cfg(target_os = "linux")]
    {
        PathBuf::from("/usr/share/unityhub").join("UnityLicensingClient_V1")
    }
}

pub fn get_hub_dll_path() -> String {
    hub_licensing_dir()
        .join("Unity.Licensing.EntitlementResolver.dll")
        .to_string_lossy()
        .to_string()
}

pub fn hub_resources_path() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        let pf = std::env::var("PROGRAMFILES").unwrap_or_else(|_| r"C:\Program Files".into());
        PathBuf::from(pf).join("Unity Hub").join("resources")
    }
    #[cfg(target_os = "macos")]
    {
        PathBuf::from("/Applications/Unity Hub.app/Contents/Resources")
    }
    #[cfg(target_os = "linux")]
    {
        PathBuf::from("/usr/share/unityhub/resources")
    }
}

pub fn scan_installed_editors() -> Vec<EditorInfo> {
    let mut all = Vec::new();
    let mut seen = std::collections::HashSet::new();

    // 1. Base install path
    for e in scan_folder(&base_install_path()) {
        let key = format!("{}-{}", e.version, e.architecture);
        if seen.insert(key) { all.push(e); }
    }

    // 2. Secondary install path
    if let Some(secondary) = read_secondary_install_path() {
        for e in scan_folder(&secondary) {
            let key = format!("{}-{}", e.version, e.architecture);
            if seen.insert(key) { all.push(e); }
        }
    }

    // 3. Manually located editors from JSON storage
    for e in read_located_editors() {
        let key = format!("{}-{}", e.version, e.architecture);
        if seen.insert(key) { all.push(e); }
    }

    all
}
