use crate::utils::semver;
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize, Default)]
struct PluginSection {
    name: Option<String>,
    publisher: Option<String>,
    description: Option<String>,
    version: Option<String>,
    access: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct PluginTomlDoc {
    plugin: Option<PluginSection>,
}

/// Lists all plugins in the `generated/plugins` directory.
pub fn list_plugins(cwd: &str) -> Result<(), String> {
    let root = Path::new(cwd).join("generated").join("plugins");
    if !root.exists() {
        crate::utils::logger::Logger::new().log_message(
            crate::utils::logger::LogLevel::Info,
            &format!("No plugins directory at {}", root.to_string_lossy()),
        );
        return Ok(());
    }
    let mut entries: Vec<PathBuf> = Vec::new();
    let rd = fs::read_dir(&root)
        .map_err(|e| format!("Failed to list {}: {}", root.to_string_lossy(), e))?;
    for pub_entry in rd.flatten() {
        let pub_path = pub_entry.path();
        if !pub_path.is_dir() {
            continue;
        }
        if let Ok(child_rd) = fs::read_dir(&pub_path) {
            for child in child_rd.flatten() {
                let p = child.path();
                if p.is_dir() && p.join("plugin.toml").exists() {
                    entries.push(p);
                }
            }
        }
    }
    if entries.is_empty() {
        crate::utils::logger::Logger::new().log_message(
            crate::utils::logger::LogLevel::Info,
            &format!("No plugins found in {}", root.to_string_lossy()),
        );
        return Ok(());
    }
    entries.sort();
    for p in entries {
        let id = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
        let fp = p.join("plugin.toml");
        let doc: PluginTomlDoc = fs::read_to_string(&fp)
            .ok()
            .and_then(|s| toml::from_str(&s).ok())
            .unwrap_or_default();
        let pl = doc.plugin.unwrap_or_default();
        let publisher = pl.publisher.unwrap_or_else(|| "?".into());
        let name = pl.name.unwrap_or_else(|| id.to_string());
        let version = pl.version.unwrap_or_else(|| "?".into());
        let access = pl.access.unwrap_or_else(|| "?".into());
        let description = pl.description.unwrap_or_default();
        crate::utils::logger::Logger::new().log_message(
            crate::utils::logger::LogLevel::Info,
            &format!(
                "- {}.{}  v{}  [{}]  {}",
                publisher, name, version, access, description
            ),
        );
    }
    Ok(())
}

/// Bumps the version of a plugin.
pub fn bump_version(cwd: &str, id: &str, bump: &str) -> Result<(), String> {
    // accept id in form <publisher>.<name>
    let plugin_dir = if id.contains('.') {
        let mut parts = id.splitn(2, '.');
        let publisher = parts.next().unwrap_or("");
        let name = parts.next().unwrap_or("");
        Path::new(cwd)
            .join("generated")
            .join("plugins")
            .join(publisher)
            .join(name)
    } else {
        Path::new(cwd).join("generated").join("plugins").join(id)
    };
    if !plugin_dir.is_dir() {
        return Err(format!(
            "Plugin '{}' not found under {}",
            id,
            plugin_dir
                .parent()
                .unwrap_or(Path::new(""))
                .to_string_lossy()
        ));
    }
    let path = plugin_dir.join("plugin.toml");
    if !path.exists() {
        return Err(format!(
            "plugin.toml not found in {}",
            plugin_dir.to_string_lossy()
        ));
    }

    let content = fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read {}: {}", path.to_string_lossy(), e))?;
    let current = parse_version_from_plugin_toml(&content).unwrap_or_else(|| "0.0.1".to_string());
    let new_version = semver::compute_bump(&current, bump)?;

    let updated = write_version_in_plugin_toml(&content, &new_version)?;
    fs::write(&path, updated)
        .map_err(|e| format!("Failed to write {}: {}", path.to_string_lossy(), e))?;
    crate::utils::logger::Logger::new().log_message(
        crate::utils::logger::LogLevel::Success,
        &format!("✅ {} -> {}", current, new_version),
    );
    Ok(())
}

fn parse_version_from_plugin_toml(toml_text: &str) -> Option<String> {
    if let Ok(doc) = toml::from_str::<PluginTomlDoc>(toml_text) {
        if let Some(p) = doc.plugin {
            return p.version;
        }
    }
    None
}

fn write_version_in_plugin_toml(original: &str, new_version: &str) -> Result<String, String> {
    let mut lines: Vec<String> = original.lines().map(|s| s.to_string()).collect();
    let mut in_plugin = false;
    let mut plugin_start = None::<usize>;
    let mut plugin_end = lines.len();
    for (i, l) in lines.iter().enumerate() {
        let t = l.trim();
        if t == "[plugin]" {
            in_plugin = true;
            plugin_start = Some(i);
            continue;
        }
        if in_plugin && t.starts_with('[') && t != "[plugin]" {
            plugin_end = i;
            break;
        }
    }
    if !in_plugin {
        return Err("[plugin] section not found".into());
    }
    let start = plugin_start.unwrap();
    let mut version_line_idx: Option<usize> = None;
    for (i, line) in lines.iter().enumerate().take(plugin_end).skip(start + 1) {
        let t = line.trim();
        if t.starts_with("version") && t.contains('=') {
            version_line_idx = Some(i);
            break;
        }
    }

    let version_line = format!("version = \"{}\"", new_version);
    match version_line_idx {
        Some(i) => {
            let indent = lines[i]
                .chars()
                .take_while(|c| c.is_whitespace())
                .collect::<String>();
            lines[i] = format!("{}{}", indent, version_line);
        }
        None => {
            let mut insert_at = plugin_end;
            for (i, line) in lines.iter().enumerate().take(plugin_end).skip(start + 1) {
                if line.trim().is_empty() {
                    insert_at = i;
                    break;
                }
            }
            if insert_at == plugin_end {
                insert_at = plugin_end;
            }
            lines.insert(insert_at, version_line);
        }
    }
    let mut out = lines.join("\n");
    if !out.ends_with('\n') {
        out.push('\n');
    }
    Ok(out)
}
