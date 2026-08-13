use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::RwLock;

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct AppConfig {
    pub hub_install_path: Option<String>,
    #[serde(default)]
    pub editor_scan_paths: Vec<String>,
}

/// 进程内配置缓存，避免每次命令都重新读盘+解析 JSON
static CONFIG_CACHE: RwLock<Option<AppConfig>> = RwLock::new(None);

fn config_path() -> PathBuf {
    let data_dir = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from(r"C:\Users\Public\AppData\Roaming"));
    data_dir.join("UniFree").join("config.json")
}

fn load_from_disk() -> AppConfig {
    let path = config_path();
    if path.exists() {
        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(config) = serde_json::from_str(&content) {
                return config;
            }
        }
    }
    AppConfig::default()
}

pub fn load() -> AppConfig {
    if let Ok(guard) = CONFIG_CACHE.read() {
        if let Some(cfg) = &*guard {
            return cfg.clone();
        }
    }
    let cfg = load_from_disk();
    if let Ok(mut guard) = CONFIG_CACHE.write() {
        *guard = Some(cfg.clone());
    }
    cfg
}

pub fn save(config: &AppConfig) -> Result<(), String> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Failed to create config dir: {}", e))?;
    }
    let json = serde_json::to_string_pretty(config).map_err(|e| format!("Failed to serialize config: {}", e))?;
    fs::write(&path, json).map_err(|e| format!("Failed to write config: {}", e))?;
    // 同步更新缓存
    if let Ok(mut guard) = CONFIG_CACHE.write() {
        *guard = Some(config.clone());
    }
    Ok(())
}

pub fn get_hub_install_path() -> Option<PathBuf> {
    let config = load();
    config.hub_install_path.map(PathBuf::from).filter(|p| p.exists())
}

pub fn set_hub_install_path(path: &str) -> Result<(), String> {
    let mut config = load();
    config.hub_install_path = Some(path.to_string());
    save(&config)
}

pub fn reset_hub_install_path() -> Result<(), String> {
    let mut config = load();
    config.hub_install_path = None;
    save(&config)
}

pub fn get_editor_scan_paths() -> Vec<String> {
    load().editor_scan_paths
}

pub fn add_editor_scan_path(path: &str) -> Result<(), String> {
    let mut config = load();
    if !config.editor_scan_paths.iter().any(|p| p.eq_ignore_ascii_case(path)) {
        config.editor_scan_paths.push(path.to_string());
        save(&config)?;
    }
    Ok(())
}

pub fn remove_editor_scan_path(path: &str) -> Result<(), String> {
    let mut config = load();
    config.editor_scan_paths.retain(|p| !p.eq_ignore_ascii_case(path));
    save(&config)
}
