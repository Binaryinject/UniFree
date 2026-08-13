use std::fs;
use serde_json;

/// 检测 hubConfig.json 的补丁状态，与 asar/exe 补丁状态解耦
/// 返回 "patched" / "partial" / "original" / "not_found" / "unknown"
pub fn get_hub_config_status() -> String {
    let hub_config_path = crate::scanner::hub_app_data().join("hubConfig.json");
    if !hub_config_path.exists() {
        return "not_found".into();
    }

    let content = match fs::read_to_string(&hub_config_path) {
        Ok(c) => c,
        Err(_) => return "unknown".into(),
    };
    let config: serde_json::Value = match serde_json::from_str(&content) {
        Ok(c) => c,
        Err(_) => return "unknown".into(),
    };

    let signin = config.get("hubDisableSignInRequired").and_then(|v| v.as_bool()).unwrap_or(false);
    let update = config.get("hubDisableAutoUpdate").and_then(|v| v.as_bool()).unwrap_or(false);
    match (signin, update) {
        (true, true) => "patched".into(),
        (false, false) => "original".into(),
        _ => "partial".into(),
    }
}

/// 更新本地 hubConfig.json 文件
pub fn update_hub_config(disable_signin: bool, disable_update: bool) -> Result<String, String> {
    let hub_config_path = crate::scanner::hub_app_data().join("hubConfig.json");

    if !hub_config_path.exists() {
        return Err(format!("hubConfig.json not found at: {}", hub_config_path.display()));
    }

    let content = fs::read_to_string(&hub_config_path)
        .map_err(|e| format!("Failed to read hubConfig.json: {}", e))?;

    let mut config: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse hubConfig.json: {}", e))?;

    if disable_signin {
        config["hubDisableSignInRequired"] = serde_json::json!(true);
    }
    if disable_update {
        config["hubDisableAutoUpdate"] = serde_json::json!(true);
    }

    let updated_content = serde_json::to_string(&config)
        .map_err(|e| format!("Failed to serialize hubConfig.json: {}", e))?;

    fs::write(&hub_config_path, updated_content)
        .map_err(|e| format!("Failed to write hubConfig.json: {}", e))?;

    Ok(format!("Updated hubConfig.json: {}", hub_config_path.display()))
}
