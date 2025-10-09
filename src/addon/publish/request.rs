use crate::utils::{api::get_forge_api_base_url, fs::get_user_home};

pub async fn post_publish_addon_to_forge_api(addon_id: &Option<String>) -> Result<(), String> {
    let client = reqwest::Client::new();

    // let forge_api_url = format!("https://forge.devalang.com/v1/addon/publish/{}", addon_id.as_ref().unwrap());
    let forge_api_url = format!(
        "{}/v1/addon/publish/{}",
        get_forge_api_base_url(),
        addon_id.as_ref().unwrap()
    );

    let home_dir =
        get_user_home().map_err(|e| format!("Failed to get user home directory: {}", e))?;
    let config_path = home_dir.join(".devalang").join("config.json");

    if !config_path.exists() {
        return Err("Configuration file not found. Please log in first.".to_string());
    }

    let config_text_content = std::fs::read_to_string(&config_path)
        .map_err(|e| format!("Failed to read config file: {}", e))?;

    let config_json_content = config_text_content
        .parse::<serde_json::Value>()
        .map_err(|e| format!("Failed to parse config file: {}", e))?;

    let user_session_token = match config_json_content.get("session") {
        Some(token) => token
            .as_str()
            .ok_or("Invalid session token in config file".to_string())?,
        None => {
            return Err("Session token not found in config file".to_string());
        }
    };

    let response = client
        .post(&forge_api_url)
        .headers({
            let mut headers = reqwest::header::HeaderMap::new();
            headers.insert(
                "Authorization",
                format!("Bearer {}", user_session_token).parse().unwrap(),
            );
            headers
        })
        .send()
        .await
        .map_err(|e| format!("Failed to send request to Forge API: {}", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let error_message = response
            .json()
            .await
            .map(|json: serde_json::Value| {
                json.get("message")
                    .and_then(|e| e.as_str())
                    .unwrap_or("Unknown error")
                    .to_string()
            })
            .unwrap_or("Failed to parse error message".to_string());

        return Err(format!(
            "Failed to publish addon: HTTP {} - {}",
            status, error_message
        ));
    }

    Ok(())
}
