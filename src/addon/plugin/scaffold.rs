use std::path::Path;

use crate::addon::plugin::preset::{
    empty::create_plugin_src_empty, synth::create_plugin_src_synth,
};
use crate::utils::logger::{LogLevel, Logger};
use reqwest;
use serde_json::Value as JsonValue;

pub async fn scaffold_plugin(
    cwd: &str,
    name: String,
    publisher: String,
    description: String,
    access: String,
    preset_type: String,
) -> Result<(), String> {
    let plugins_root = Path::new(cwd).join("generated").join("plugins");

    let plugin_path = plugins_root.join(&publisher).join(&name);
    if plugin_path.exists() {
        Logger::new().log_message(LogLevel::Error, "Plugin already exists, aborting.");
        return Err("Plugin already exists".into());
    }

    if let Err(e) = std::fs::create_dir_all(&plugin_path) {
        Logger::new().log_message(
            LogLevel::Error,
            &format!("Error creating plugin directory: {}", e),
        );
        return Err(format!("Failed to create plugin directory: {}", e));
    }

    if let Err(e) = create_plugin_toml(&plugin_path, &name, &publisher, &description, &access).await
    {
        Logger::new().log_message(
            LogLevel::Error,
            &format!("Error creating plugin toml: {}", e),
        );
        return Err(format!("Failed to create plugin toml: {}", e));
    }

    if let Err(e) =
        create_plugin_cargo_toml(cwd, &plugin_path, &name, &publisher, &description).await
    {
        Logger::new().log_message(
            LogLevel::Error,
            &format!("Error creating Cargo.toml: {}", e),
        );
        return Err(format!("Failed to create Cargo.toml: {}", e));
    }

    if let Err(e) = create_plugin_src_dir(&plugin_path, &preset_type).await {
        Logger::new().log_message(
            LogLevel::Error,
            &format!("Error creating plugin src directory: {}", e),
        );
        return Err(format!("Failed to create plugin src directory: {}", e));
    }

    if let Err(e) = write_default_docs(&plugin_path, &publisher, &name, &description).await {
        Logger::new().log_message(
            LogLevel::Warning,
            &format!("Warning: failed to create default docs: {}", e),
        );
    }

    Ok(())
}

async fn write_default_docs(
    plugin_path: &Path,
    publisher: &str,
    name: &str,
    description: &str,
) -> Result<(), String> {
    // README.md
    let readme_path = plugin_path.join("README.md");
    if !readme_path.exists() {
        let readme = format!(
            "# {}.{} Plugin\n\n{}\n\nContents:\n- plugin.toml\n- src/lib.rs\n- LICENSE\n\nBuilt with devapack.\n",
            publisher, name, description
        );
        std::fs::write(&readme_path, readme)
            .map_err(|e| format!("Failed to write README.md: {}", e))?;
    }

    // LICENSE (MIT)
    let license_path = plugin_path.join("LICENSE");
    if !license_path.exists() {
        let license = format!(
            "MIT License\n\nCopyright (c) {}\n\nPermission is hereby granted, free of charge, to any person obtaining a copy\n of this software and associated documentation files (the \"Software\"), to deal\n in the Software without restriction, including without limitation the rights\n to use, copy, modify, merge, publish, distribute, sublicense, and/or sell\n copies of the Software, and to permit persons to whom the Software is\n furnished to do so, subject to the following conditions:\n\nThe above copyright notice and this permission notice shall be included in all\n copies or substantial portions of the Software.\n\nTHE SOFTWARE IS PROVIDED \"AS IS\", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR\n IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,\n FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE\n publisherS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER\n LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,\n OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE\n SOFTWARE.\n",
            publisher
        );
        std::fs::write(&license_path, license)
            .map_err(|e| format!("Failed to write LICENSE: {}", e))?;
    }

    Ok(())
}

pub async fn create_plugin_toml(
    plugin_path: &Path,
    name: &str,
    publisher: &str,
    description: &str,
    access: &str,
) -> Result<(), String> {
    let version = "0.0.1";
    let toml_content = format!(
        r#"[plugin]
name = "{name}"
publisher = "{publisher}"
description = "{description}"
version = "{version}"
access = "{access}"

[[exports]]
name = "process"
kind = "func"
"#,
        name = name,
        publisher = publisher,
        description = description,
        version = version,
        access = access
    );

    let toml_path = plugin_path.join("plugin.toml");
    if let Err(e) = std::fs::write(&toml_path, toml_content) {
        Logger::new().log_message(
            LogLevel::Error,
            &format!("Error creating plugin.toml: {}", e),
        );
        return Err(format!("Failed to create plugin.toml: {}", e));
    }

    Ok(())
}

pub async fn create_plugin_cargo_toml(
    cwd: &str,
    plugin_path: &Path,
    name: &str,
    publisher: &str,
    description: &str,
) -> Result<(), String> {
    // Helper: attempt to fetch latest version of a crate from crates.io
    async fn latest_crate_version(crate_name: &str) -> Result<Option<String>, String> {
        let url = format!("https://crates.io/api/v1/crates/{}", crate_name);
        let resp = reqwest::get(&url)
            .await
            .map_err(|e| format!("Failed to query crates.io: {}", e))?;
        if !resp.status().is_success() {
            return Ok(None);
        }
        let json: JsonValue = resp
            .json()
            .await
            .map_err(|e| format!("Failed to parse crates.io response: {}", e))?;
        if let Some(v) = json
            .get("crate")
            .and_then(|c| c.get("max_version"))
            .and_then(|m| m.as_str())
        {
            Ok(Some(v.to_string()))
        } else {
            Ok(None)
        }
    }

    // Try to get the latest published version of `devalang` from crates.io.
    // If we can fetch it, generate the plugin Cargo.toml to depend on that version.
    // Otherwise, fall back to using the local relative path to `devalang`.
    let registry_version = match latest_crate_version("devalang").await {
        Ok(Some(v)) => {
            Logger::new().log_message(
                LogLevel::Info,
                &format!(
                    "Using devalang crate version {} from crates.io for plugin Cargo.toml",
                    v
                ),
            );
            Some(v)
        }
        Ok(None) => {
            Logger::new().log_message(
                LogLevel::Warning,
                "Could not find devalang on crates.io, falling back to local path dependency.",
            );
            None
        }
        Err(e) => {
            Logger::new().log_message(
                LogLevel::Warning,
                &format!(
                    "Failed to query crates.io for devalang: {}. Using local path.",
                    e
                ),
            );
            None
        }
    };

    let cargo_toml_content = if let Some(ver) = registry_version {
        format!(
            r#"[package]
name = "{name}"
description = "{description}"
version = "0.0.1"
authors = ['{publisher}']
edition = "2024"

[workspace]
members = ["."]

[lib]
name = "{name}"
path = "src/lib.rs"
crate-type = ["cdylib"]

[dependencies]
devalang = {{ version = "{ver}", default-features = false, features = ["plugin"] }}
"#,
            name = name,
            description = description,
            publisher = publisher,
            ver = ver
        )
    } else {
        Logger::new().log_message(
            LogLevel::Error,
            &format!("Unable to determine Devalang version from crates.io"),
        );
        return Err("Unable to determine Devalang version from crates.io".into());
    };

    // Write plugin Cargo.toml
    let cargo_toml_path = plugin_path.join("Cargo.toml");
    if let Err(e) = std::fs::write(&cargo_toml_path, cargo_toml_content) {
        Logger::new().log_message(
            LogLevel::Error,
            &format!("Error creating Cargo.toml: {}", e),
        );
        return Err(format!("Failed to create Cargo.toml: {}", e));
    }

    if let Err(e) = add_plugin_to_root_cargo(cwd).await {
        Logger::new().log_message(
            LogLevel::Error,
            &format!("Error adding plugin to workspace: {}", e),
        );
        return Err(format!("Failed to add plugin to workspace: {}", e));
    }

    Ok(())
}

pub async fn add_plugin_to_root_cargo(cwd: &str) -> Result<(), String> {
    let cargo_toml_root_path = Path::new(cwd).join("Cargo.toml");

    // find most recently modified plugin directory under generated/plugins/<publisher>/<name>
    let generated_dir = Path::new(cwd).join("generated").join("plugins");
    let mut newest: Option<(std::time::SystemTime, String)> = None;
    if let Ok(rd) = std::fs::read_dir(&generated_dir) {
        for pub_entry in rd.flatten() {
            if let Ok(pub_ft) = pub_entry.file_type() {
                if pub_ft.is_dir() {
                    let pub_path = pub_entry.path();
                    if let Ok(child_rd) = std::fs::read_dir(&pub_path) {
                        for child in child_rd.flatten() {
                            if let Ok(child_ft) = child.file_type() {
                                if child_ft.is_dir() {
                                    if let Ok(m) = child.metadata().and_then(|m| m.modified()) {
                                        if let (Some(pub_name), Some(child_name)) = (
                                            pub_entry.file_name().to_str(),
                                            child.file_name().to_str(),
                                        ) {
                                            let rel = format!(
                                                "generated/plugins/{}/{}",
                                                pub_name, child_name
                                            );
                                            if newest.as_ref().map(|(t, _)| m > *t).unwrap_or(true)
                                            {
                                                newest = Some((m, rel));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    let plugin = match newest.map(|(_, p)| p) {
        Some(p) => p,
        None => {
            return Ok(());
        }
    };

    let orig = std::fs::read_to_string(&cargo_toml_root_path)
        .map_err(|e| format!("Failed to read root Cargo.toml: {}", e))?;

    // ensure [workspace] exists with members = ["."]
    let mut out = orig.clone();
    if !orig.contains("[workspace]") {
        out.push_str("\n[workspace]\nmembers = [\".\"]\nexclude = [\"");
        out.push_str(&plugin);
        out.push_str("\"]\n");
        std::fs::write(&cargo_toml_root_path, out)
            .map_err(|e| format!("Failed to write root Cargo.toml: {}", e))?;
        return Ok(());
    }

    // operate inside workspace section
    let lines: Vec<&str> = orig.lines().collect();
    let mut start = None;
    for (i, l) in lines.iter().enumerate() {
        if l.trim() == "[workspace]" {
            start = Some(i);
            break;
        }
    }
    let s = match start {
        Some(s) => s,
        None => {
            return Ok(());
        }
    };
    let mut end = lines.len();
    for (i, _) in lines.iter().enumerate().skip(s + 1) {
        if lines[i].trim_start().starts_with('[') {
            end = i;
            break;
        }
    }
    let section = lines[s..end].join("\n");

    if section.contains("exclude") {
        // find first '[' and ']' after exclude
        if let Some(p) = section.find("exclude") {
            if let Some(o) = section[p..].find('[') {
                let open = p + o;
                if let Some(c) = section[open..].find(']') {
                    let close = open + c;
                    let inside = &section[open + 1..close];
                    let mut items: Vec<String> = inside
                        .split(',')
                        .map(|s| s.trim().trim_matches('"').to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                    if items.iter().any(|it| it == &plugin) {
                        return Ok(());
                    }
                    items.push(plugin.clone());
                    let new_inside = items
                        .into_iter()
                        .map(|it| format!("\"{}\"", it))
                        .collect::<Vec<_>>()
                        .join(", ");
                    let old_fragment = &section[open..=close];
                    let new_fragment = format!("[{}]", new_inside);
                    let new_section = section.replacen(old_fragment, &new_fragment, 1);
                    out = orig.replacen(&section, &new_section, 1);
                    std::fs::write(&cargo_toml_root_path, out)
                        .map_err(|e| format!("Failed to write root Cargo.toml: {}", e))?;
                    return Ok(());
                }
            }
        }
    } else {
        // insert exclude = ["plugin"] after members line if present, else after header
        let mut new_lines: Vec<String> = lines.iter().map(|s| s.to_string()).collect();
        let mut inserted = false;
        for i in s + 1..end {
            if new_lines[i].contains("members") {
                new_lines.insert(i + 1, format!("exclude = [\"{}\"]", plugin));
                inserted = true;
                break;
            }
        }
        if !inserted {
            new_lines.insert(s + 1, format!("exclude = [\"{}\"]", plugin));
        }
        out = new_lines.join("\n");
        std::fs::write(&cargo_toml_root_path, out)
            .map_err(|e| format!("Failed to write root Cargo.toml: {}", e))?;
        return Ok(());
    }

    Ok(())
}

pub async fn create_plugin_src_dir(plugin_path: &Path, preset_type: &str) -> Result<(), String> {
    let src_path = plugin_path.join("src");

    match preset_type {
        "empty" => {
            if let Err(e) = create_plugin_src_empty(&src_path).await {
                Logger::new().log_message(
                    LogLevel::Error,
                    &format!("Error creating empty plugin src: {}", e),
                );
                return Err(format!("Failed to create empty plugin src: {}", e));
            }
        }

        "synth" => {
            if let Err(e) = create_plugin_src_synth(&src_path).await {
                Logger::new().log_message(
                    LogLevel::Error,
                    &format!("Error creating synth plugin src: {}", e),
                );
                return Err(format!("Failed to create synth plugin src: {}", e));
            }
        }

        // "fx" => {
        //     // Create an effects plugin structure
        // }

        // "sequencer" => {
        //     // Create a sequencer plugin structure
        // }

        // "midi" => {
        //     // Create a MIDI plugin structure
        // }

        // "utility" => {
        //     // Create a utility plugin structure
        // }
        _ => {
            Logger::new().log_message(
                LogLevel::Error,
                &format!("Unknown preset type: {}", preset_type),
            );
            return Err(format!("Unknown preset type: {}", preset_type));
        }
    }

    Ok(())
}
