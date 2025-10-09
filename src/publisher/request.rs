use crate::{
    types::publisher::{PublisherInfo, PublisherInfoUpdate},
    utils::{api::get_forge_api_base_url, fs::get_user_home},
};

pub async fn get_user_publishers() -> Result<Vec<PublisherInfo>, String> {
    let client = reqwest::Client::new();
    let api_url = format!("{}/v1/publisher/list", get_forge_api_base_url());

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

    let response = match client
        .get(api_url)
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
    {
        Ok(resp) => resp,
        Err(e) => {
            return Err(format!("Failed to send request to Forge API: {}", e));
        }
    };

    if response.status().is_success() {
        let body = response
            .text()
            .await
            .map_err(|e| format!("Failed to read response body: {}", e))?;

        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&body) {
            if let Some(payload) = json.get("payload") {
                if let Some(publishers) = payload.get("publishers") {
                    if let Some(publisher_array) = publishers.as_array() {
                        let publishers_data: Vec<PublisherInfo> = publisher_array
                            .iter()
                            .filter_map(|p| {
                                if let Some(name) = p.get("identifier") {
                                    name.as_str().map(|s| PublisherInfo {
                                        identifier: s.to_string(),
                                        display_name: p
                                            .get("display_name")
                                            .and_then(|dn| dn.as_str().map(|s| s.to_string()))
                                            .unwrap_or("".to_string()),
                                        description: p
                                            .get("description")
                                            .and_then(|desc| desc.as_str().map(|s| s.to_string()))
                                            .unwrap_or("".to_string()),
                                        logo_url: Some(
                                            p.get("logo_url")
                                                .and_then(|url| url.as_str().map(|s| s.to_string()))
                                                .unwrap_or("".to_string()),
                                        ),
                                        banner_url: Some(
                                            p.get("banner_url")
                                                .and_then(|url| url.as_str().map(|s| s.to_string()))
                                                .unwrap_or("".to_string()),
                                        ),
                                        country_code: Some(
                                            p.get("country_code")
                                                .and_then(|cc| cc.as_str().map(|s| s.to_string()))
                                                .unwrap_or("".to_string()),
                                        ),
                                        tags: (|| -> Option<Vec<String>> {
                                            if let Some(tags_val) = p.get("tags") {
                                                // Case 1: tags is already a JSON array
                                                if let Some(arr) = tags_val.as_array() {
                                                    return Some(
                                                        arr.iter()
                                                            .filter_map(|t| {
                                                                t.as_str().map(|s| s.to_string())
                                                            })
                                                            .collect(),
                                                    );
                                                }

                                                // Case 2: tags is a JSON string containing a JSON array like "[\"a\",\"b\"]"
                                                if let Some(s) = tags_val.as_str() {
                                                    if let Ok(parsed) =
                                                        serde_json::from_str::<serde_json::Value>(s)
                                                    {
                                                        if let Some(arr2) = parsed.as_array() {
                                                            return Some(
                                                                arr2.iter()
                                                                    .filter_map(|t| {
                                                                        t.as_str()
                                                                            .map(|s| s.to_string())
                                                                    })
                                                                    .collect(),
                                                            );
                                                        }
                                                    }
                                                }
                                            }
                                            None
                                        })()
                                        .unwrap_or_else(Vec::new),
                                    })
                                } else {
                                    None
                                }
                            })
                            .collect();

                        Ok(publishers_data)
                    } else {
                        Err("Publishers field is not an array".to_string())
                    }
                } else {
                    Err("Publishers field not found in response".to_string())
                }
            } else {
                Err("Payload field not found in response".to_string())
            }
        } else {
            Err("Failed to parse response JSON".to_string())
        }
    } else {
        let status = response.status();
        let error_text = match response.text().await {
            Ok(text) => text,
            Err(_) => "No additional error information".to_string(),
        };
        Err(format!(
            "Failed to fetch publishers. Status: {}, Error: {}",
            status, error_text
        ))
    }
}

pub async fn post_create_publisher_to_forge_api(
    publisher_payload: &PublisherInfo,
) -> Result<(), String> {
    let client = reqwest::Client::new();
    let api_url = format!("{}/v1/publisher/create", get_forge_api_base_url());

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

    let response = match client
        .post(api_url)
        .headers({
            let mut headers = reqwest::header::HeaderMap::new();
            headers.insert(
                "Authorization",
                format!("Bearer {}", user_session_token).parse().unwrap(),
            );
            headers
        })
        .json(&publisher_payload)
        .send()
        .await
    {
        Ok(resp) => resp,
        Err(e) => {
            return Err(format!("Failed to send request to Forge API: {}", e));
        }
    };

    if response.status().is_success() {
        Ok(())
    } else {
        let status = response.status();
        let error_text = match response.text().await {
            Ok(text) => text,
            Err(_) => "No additional error information".to_string(),
        };
        Err(format!(
            "Failed to create publisher. Status: {}, Error: {}",
            status, error_text
        ))
    }
}

pub async fn post_update_publisher_to_forge_api(
    publisher_id: &str,
    publisher_payload: &PublisherInfoUpdate,
) -> Result<(), String> {
    let client = reqwest::Client::new();
    let api_url = format!(
        "{}/v1/publisher/update/{}",
        get_forge_api_base_url(),
        publisher_id
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

    let response = match client
        .post(&api_url)
        .headers({
            let mut headers = reqwest::header::HeaderMap::new();
            headers.insert(
                "Authorization",
                format!("Bearer {}", user_session_token).parse().unwrap(),
            );
            headers
        })
        .json(&publisher_payload)
        .send()
        .await
    {
        Ok(resp) => resp,
        Err(e) => {
            return Err(format!("Failed to send request to Forge API: {}", e));
        }
    };

    if response.status().is_success() {
        Ok(())
    } else {
        let status = response.status();
        let error_text = match response.text().await {
            Ok(text) => text,
            Err(_) => "No additional error information".to_string(),
        };
        Err(format!(
            "Failed to update publisher. Status: {}, Error: {}",
            status, error_text
        ))
    }
}
