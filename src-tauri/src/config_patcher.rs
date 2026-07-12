use std::fs;
use serde_json;

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
