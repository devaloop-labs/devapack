use serde_json::Value;

pub fn load_session_token() -> Result<String, String> {
    let home = crate::utils::fs::get_user_home()?;
    let config_path = home.join(".devalang").join("config.json");
    if !config_path.exists() {
        return Err("Configuration file not found. Please log in first.".to_string());
    }
    let cfg_text = std::fs::read_to_string(&config_path)
        .map_err(|e| format!("Failed to read config file: {}", e))?;
    let cfg_json: Value = cfg_text
        .parse()
        .map_err(|e| format!("Failed to parse config file: {}", e))?;
    let token = cfg_json
        .get("session")
        .and_then(|v| v.as_str())
        .ok_or("Session token not found in config file".to_string())?;
    Ok(token.to_string())
}
