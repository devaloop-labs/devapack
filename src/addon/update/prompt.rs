use crate::builder::{bank as bank_builder, plugin as plugin_builder};
use crate::utils::api::get_forge_api_base_url;
use crate::utils::fs::get_user_home;
use crate::{
    addon::{
        publish::request::post_publish_addon_to_forge_api,
        submit::{analyze::analyze_addon, discover::discover_addons},
        update::request::post_update_addon_to_forge_api,
    },
    types::addon::AddonSubmissionData,
    utils::logger::{LogLevel, Logger},
    utils::spinner::with_spinner,
};
use ed25519_dalek::SecretKey;
use getrandom::getrandom;
use std::io::Write;

pub async fn prompt_update_addon(cwd: &str) -> Result<(), String> {
    println!();
    println!("⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯");
    println!("Devalang Addon Updater");
    println!("⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯");
    println!();

    let fetch_addons_spinner = with_spinner("Fetching available addons...");

    let discovered_addons = match discover_addons().await {
        Ok(addons) => addons,
        Err(e) => {
            return Err(format!("Failed to discover addons: {}", e));
        }
    };

    fetch_addons_spinner.finish_and_clear();

    let addons_list = discovered_addons
        .iter()
        .map(|addon| format!("{} ({})", addon.name.clone(), addon.addon_type.clone()))
        .collect::<Vec<_>>();

    let selected_addon_string =
        match inquire::Select::new("Select an addon to update:", addons_list).prompt() {
            Ok(addon) => addon,
            Err(e) => {
                return Err(format!("Failed to prompt for addon type: {}", e));
            }
        };

    let selected_addon_analyze_spinner = with_spinner("Analyzing selected addon...");

    let selected_addon_name = selected_addon_string
        .split(' ')
        .next()
        .unwrap_or("")
        .to_string();

    let selected_addon = match discovered_addons
        .iter()
        .find(|a| a.name.clone() == selected_addon_name)
    {
        Some(addon) => addon,
        None => return Err("Selected addon not found in discovered addons".to_string()),
    };

    let addon_metadata = match analyze_addon(selected_addon).await {
        Ok(meta) => meta,
        Err(e) => {
            return Err(format!("Failed to analyze addon: {}", e));
        }
    };

    selected_addon_analyze_spinner.finish_and_clear();

    let _confirm_prompt = match inquire::Confirm::new(&format!(
        "Update addon '{}' with version '{}' and access '{}' ?",
        selected_addon.name, addon_metadata.version, addon_metadata.access
    ))
    .with_default(true)
    .prompt()
    {
        Ok(c) => c,
        Err(e) => {
            return Err(format!("Failed to prompt for confirmation: {}", e));
        }
    };

    let submit_addon_spinner = with_spinner("Submitting addon update...");

    let addon_id = fetch_addon_id(&addon_metadata.publisher, &addon_metadata.name).await?;

    let submission_data = AddonSubmissionData {
        id: Some(addon_id),
        name: addon_metadata.name.clone(),
        addon_type: selected_addon.addon_type.clone(),
        path: selected_addon.path.clone(),
        version: addon_metadata.version.clone(),
        access: addon_metadata.access.clone(),
        files: selected_addon.files.clone(),
        publisher: addon_metadata.publisher.clone(),
    };

    // Build the addon before updating (produces .devabank or .devaplugin in output/)
    {
        let build_spinner = with_spinner("Building addon before update...");
        let build_result = match submission_data.addon_type.as_str() {
            "bank" => bank_builder::build_bank(&submission_data.path, cwd),
            "plugin" => {
                plugin_builder::build_plugin(&submission_data.path, &false, cwd, false, false)
            }
            _ => Err("Unknown addon type for build".to_string()),
        };
        build_spinner.finish_and_clear();
        if let Err(e) = build_result {
            return Err(format!("Failed to build addon before update: {}", e));
        }
    }

    // Ensure keypair exists (create if missing) for update flow as well
    if let Ok(home) = get_user_home() {
        let keys_dir = home.join(".devalang").join("keys");
        let key_file = keys_dir.join("ed25519.key");
        if !key_file.exists() {
            if let Err(e) = std::fs::create_dir_all(&keys_dir) {
                eprintln!(
                    "Failed to create keys directory {}: {}",
                    keys_dir.display(),
                    e
                );
            }
            // generate 32 bytes seed
            let mut seed = [0u8; 32];
            if getrandom(&mut seed).is_ok() {
                if let Ok(sk) = SecretKey::from_bytes(&seed) {
                    let public = ed25519_dalek::PublicKey::from(&sk);
                    let kp = ed25519_dalek::Keypair { secret: sk, public };
                    match std::fs::File::create(&key_file) {
                        Ok(mut f) => match f.write_all(&kp.to_bytes()) {
                            Ok(_) => Logger::new().log_message(
                                LogLevel::Success,
                                &format!("Created ed25519 keypair at {}", key_file.display()),
                            ),
                            Err(e) => {
                                Logger::new().log_message(
                                    LogLevel::Error,
                                    &format!(
                                        "Failed to write key file {}: {}",
                                        key_file.display(),
                                        e
                                    ),
                                );
                            }
                        },
                        Err(e) => {
                            Logger::new().log_message(
                                LogLevel::Error,
                                &format!("Failed to create key file {}: {}", key_file.display(), e),
                            );
                        }
                    }
                } else {
                    Logger::new().log_message(
                        LogLevel::Error,
                        "Failed to derive secret key from random seed",
                    );
                }
            } else {
                Logger::new().log_message(
                    LogLevel::Error,
                    "Failed to gather randomness to create ed25519 key",
                );
            }
        }
    }

    let (addon_id_opt, sig_opt, pub_opt, sha_opt) =
        match post_update_addon_to_forge_api(&submission_data).await {
            Ok(tuple) => tuple,
            Err(e) => {
                return Err(format!("Failed to update addon: {}", e));
            }
        };

    let addon_id = match addon_id_opt {
        Some(id) => id,
        None => {
            return Err("Addon ID missing after update".to_string());
        }
    };

    // If signature & pubkey were produced by the client and returned, call the sign endpoint to register them.
    if let (Some(sig_b64), Some(pub_b64), Some(sha_hex)) = (sig_opt, pub_opt, sha_opt) {
        let sign_url = format!("{}/v1/addon/sign/{}", get_forge_api_base_url(), addon_id);
        let client = reqwest::Client::new();
        let payload = serde_json::json!({
            "public_key": pub_b64,
            "signature": sig_b64,
            "archive_sha256": sha_hex
        });

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

        let res = client
            .post(&sign_url)
            .headers({
                let mut headers = reqwest::header::HeaderMap::new();
                headers.insert(
                    "Authorization",
                    format!("Bearer {}", user_session_token).parse().unwrap(),
                );
                headers
            })
            .json(&payload)
            .send()
            .await;
        match res {
            Ok(r) => {
                if !r.status().is_success() {
                    return Err(format!(
                        "Failed to register signature: HTTP {}, Body: {:?}",
                        r.status(),
                        r.text().await
                    ));
                }
                // parse body and print summary
                let body = r.text().await.unwrap_or_default();
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&body) {
                    let payload = json.get("payload").unwrap_or(&json);
                    let key_path = get_user_home()
                        .unwrap()
                        .join(".devalang")
                        .join("keys")
                        .join("ed25519.key");
                    crate::addon::summary::print_addon_summary(payload, &key_path);
                }
            }
            Err(e) => {
                return Err(format!("Failed to call sign endpoint: {}", e));
            }
        }
    }

    submit_addon_spinner.finish_and_clear();

    let publish_confirmation = match inquire::Confirm::new("Do you want to publish now ?")
        .with_default(true)
        .prompt()
    {
        Ok(c) => c,
        Err(e) => {
            return Err(format!("Failed to prompt for confirmation: {}", e));
        }
    };

    if publish_confirmation {
        let publish_addon_spinner = with_spinner("Publishing addon update...");

        if let Err(e) = post_publish_addon_to_forge_api(&submission_data.id).await {
            return Err(format!("Failed to publish addon: {}", e));
        }

        publish_addon_spinner.finish_and_clear();

        Logger::new().log_message(
            LogLevel::Success,
            &format!(
                "Addon '{}' version '{}' updated successfully !",
                submission_data.name, submission_data.version
            ),
        );
    } else {
        Logger::new().log_message(
            LogLevel::Info,
            "You can publish your addon later using the appropriate command.",
        );
    }

    Ok(())
}

async fn fetch_addon_id(addon_publisher: &String, addon_name: &String) -> Result<String, String> {
    let forge_api_url = format!(
        "{}/v1/addon/get/{}/{}",
        get_forge_api_base_url(),
        addon_publisher,
        addon_name
    );

    let response = reqwest::get(&forge_api_url)
        .await
        .map_err(|e| format!("Failed to send request to Forge API: {}", e))?;

    if !response.status().is_success() {
        return Err(format!(
            "Failed to fetch addon metadata: HTTP {}",
            response.status()
        ));
    }

    let metadata: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse response JSON: {}", e))?;

    // The API may return the addon id under different keys (id, addon_id, addonId).
    // Accept string or numeric values and return a normalized string.
    let payload = metadata.get("payload").unwrap_or(&metadata);

    let id_val = payload
        .get("id")
        .or_else(|| payload.get("addon_id"))
        .or_else(|| payload.get("addonId"));

    if let Some(idv) = id_val {
        // If it's a string, return it raw; if number, use its to_string(); otherwise fallback to JSON string.
        let id_str = if let Some(s) = idv.as_str() {
            s.to_string()
        } else {
            idv.to_string()
        };
        return Ok(id_str);
    }

    Err("Addon ID not found in response payload".to_string())
}
