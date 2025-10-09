use crate::builder::{bank as bank_builder, plugin as plugin_builder};
use crate::{
    addon::{
        publish::request::post_publish_addon_to_forge_api,
        submit::{
            analyze::analyze_addon, discover::discover_addons, request::post_addon_to_forge_api,
        },
    },
    types::addon::AddonSubmissionData,
    utils::logger::{LogLevel, Logger},
    utils::spinner::with_spinner,
};

pub async fn prompt_submit_addon(cwd: &str) -> Result<(), String> {
    println!();
    println!("⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯");
    println!("Devalang Addon Submitter");
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
        match inquire::Select::new("Select an addon to submit:", addons_list).prompt() {
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
        "Submit addon '{}' with version '{}' and access '{}' ?",
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

    let submit_addon_spinner = with_spinner("Submitting addon...");

    let submission_data = AddonSubmissionData {
        id: None,
        name: addon_metadata.name.clone(),
        addon_type: selected_addon.addon_type.clone(),
        path: selected_addon.path.clone(),
        version: addon_metadata.version.clone(),
        access: addon_metadata.access.clone(),
        files: selected_addon.files.clone(),
        publisher: addon_metadata.publisher.clone(),
    };

    // Build the addon before submitting (produces .tar.gz in output/)
    {
        let build_spinner = with_spinner("Building addon before submit...");
        let build_result = match submission_data.addon_type.as_str() {
            "bank" => bank_builder::build_bank(&submission_data.path, cwd),
            "plugin" =>
            // Align with update flow: do not show summary during submit build
            {
                plugin_builder::build_plugin(&submission_data.path, &false, cwd, false, false)
            }
            _ => Err("Unknown addon type for build".to_string()),
        };
        build_spinner.finish_and_clear();
        if let Err(e) = build_result {
            return Err(format!("Failed to build addon before submit: {}", e));
        }
    }

    // Ensure keypair exists (create if missing)
    if let Err(e) = crate::utils::signing::ensure_keypair() {
        Logger::new().log_message(
            LogLevel::Warning,
            &format!("Failed to ensure signing keypair: {}", e),
        );
    }

    let (addon_id_opt, sig_opt, pub_opt, sha_opt) =
        match post_addon_to_forge_api(&submission_data).await {
            Ok(tuple) => tuple,
            Err(e) => {
                return Err(format!("Failed to submit addon: {}", e));
            }
        };

    let addon_id = match addon_id_opt {
        Some(id) => id,
        None => {
            return Err("Addon ID missing after submission".to_string());
        }
    };

    // If signature & pubkey were produced by the client and returned, call the sign endpoint to register them.
    if let (Some(sig_b64), Some(pub_b64), Some(sha_hex)) = (sig_opt, pub_opt, sha_opt) {
        match crate::addon::remote_sign::register_signature_with_server(
            &addon_id, &sig_b64, &pub_b64, &sha_hex,
        )
        .await
        {
            Ok(json) => {
                let payload = json.get("payload").unwrap_or(&json);
                if let Ok(key_path) = crate::utils::signing::key_path() {
                    crate::addon::summary::print_addon_summary(payload, &key_path);
                }
            }
            Err(e) => {
                return Err(e);
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
        let publish_addon_spinner = with_spinner("Publishing addon...");

        if let Err(e) = post_publish_addon_to_forge_api(&Some(addon_id.clone())).await {
            return Err(format!("Failed to publish addon: {}", e));
        }

        publish_addon_spinner.finish_and_clear();

        Logger::new().log_message(
            LogLevel::Success,
            &format!(
                "Addon '{}' version '{}' published successfully !",
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
