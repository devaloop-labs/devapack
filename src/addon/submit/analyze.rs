use crate::types::addon::{AddonInfo, AddonMetadata};

pub async fn analyze_addon(selected_addon: &AddonInfo) -> Result<AddonMetadata, String> {
    let addon_toml_file = match selected_addon.addon_type.as_str() {
        "bank" => "bank.toml",
        "plugin" => "plugin.toml",
        _ => {
            return Err("Unknown addon type".to_string());
        }
    };

    let addon_toml_file_path = format!("{}/{}", selected_addon.path, addon_toml_file);

    let toml_content = std::fs::read_to_string(&addon_toml_file_path)
        .map_err(|e| format!("Failed to read addon TOML file: {}", e))?;

    let parsed_toml: toml::Value = toml::from_str(&toml_content)
        .map_err(|e| format!("Failed to parse addon TOML file: {}", e))?;

    let name = parsed_toml
        .get(selected_addon.addon_type.as_str())
        .unwrap()
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();

    let version = parsed_toml
        .get(selected_addon.addon_type.as_str())
        .unwrap()
        .get("version")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();

    let access = parsed_toml
        .get(selected_addon.addon_type.as_str())
        .unwrap()
        .get("access")
        .and_then(|v| v.as_str())
        .unwrap_or("public")
        .to_string();

    let publisher = parsed_toml
        .get(selected_addon.addon_type.as_str())
        .unwrap()
        .get("publisher")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();

    Ok(AddonMetadata {
        name,
        version,
        access,
        publisher,
    })
}
