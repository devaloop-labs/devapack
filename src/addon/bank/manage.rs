use crate::utils::semver;
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize, Default)]
struct BankSection {
    name: Option<String>,
    publisher: Option<String>,
    description: Option<String>,
    version: Option<String>,
    access: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct BankTomlDoc {
    bank: Option<BankSection>,
}

/// Lists all banks in the `generated/banks` directory.
///
/// ### Parameters
/// - `cwd`: The current working directory.
///
pub fn list_banks(cwd: &str) -> Result<(), String> {
    let root = Path::new(cwd).join("generated").join("banks");
    if !root.exists() {
        crate::utils::logger::Logger::new().log_message(
            crate::utils::logger::LogLevel::Info,
            &format!("No banks directory at {}", root.to_string_lossy()),
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
                if p.is_dir() && p.join("bank.toml").exists() {
                    entries.push(p);
                }
            }
        }
    }
    if entries.is_empty() {
        crate::utils::logger::Logger::new().log_message(
            crate::utils::logger::LogLevel::Info,
            &format!("No banks found in {}", root.to_string_lossy()),
        );
        return Ok(());
    }
    entries.sort();
    for p in entries {
        let id = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
        let fp = p.join("bank.toml");
        let doc: BankTomlDoc = fs::read_to_string(&fp)
            .ok()
            .and_then(|s| toml::from_str(&s).ok())
            .unwrap_or_default();
        let b = doc.bank.unwrap_or_default();
        let publisher = b.publisher.unwrap_or_else(|| "?".into());
        let name = b.name.unwrap_or_else(|| id.to_string());
        let version = b.version.unwrap_or_else(|| "?".into());
        let access = b.access.unwrap_or_else(|| "?".into());
        let description = b.description.unwrap_or_default();
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

/// Bumps the version of a bank.
///
/// ### Parameters
/// - `cwd`: The current working directory.
/// - `id`: The ID of the bank (format: <publisher>.<name>).
/// - `bump`: The version bump to apply (e.g. "patch", "minor", "major").
///
pub fn bump_version(cwd: &str, id: &str, bump: &str) -> Result<(), String> {
    // accept id in form <publisher>.<name>
    let bank_dir = if id.contains('.') {
        let mut parts = id.splitn(2, '.');
        let publisher = parts.next().unwrap_or("");
        let name = parts.next().unwrap_or("");
        Path::new(cwd)
            .join("generated")
            .join("banks")
            .join(publisher)
            .join(name)
    } else {
        Path::new(cwd).join("generated").join("banks").join(id)
    };
    if !bank_dir.is_dir() {
        return Err(format!(
            "Bank '{}' not found under {}",
            id,
            bank_dir.parent().unwrap_or(Path::new("")).to_string_lossy()
        ));
    }
    let path = bank_dir.join("bank.toml");
    if !path.exists() {
        return Err(format!(
            "bank.toml not found in {}",
            bank_dir.to_string_lossy()
        ));
    }

    // Read current version from TOML, but update by editing the text to preserve formatting
    let content = fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read {}: {}", path.to_string_lossy(), e))?;
    let current = parse_version_from_bank_toml(&content).unwrap_or_else(|| "0.0.1".to_string());
    let new_version = semver::compute_bump(&current, bump)?;

    let updated = write_version_in_bank_toml(&content, &new_version)?;
    fs::write(&path, updated)
        .map_err(|e| format!("Failed to write {}: {}", path.to_string_lossy(), e))?;
    crate::utils::logger::Logger::new().log_message(
        crate::utils::logger::LogLevel::Success,
        &format!("✅ {} -> {}", current, new_version),
    );
    Ok(())
}

/// Deletes a generated bank directory under `generated/banks/<id>`.
///
/// ### Parameters
/// - `cwd`: current working directory.
/// - `id`: bank identifier `<publisher>.<name>`.
///
pub fn delete_bank(cwd: &str, id: &str) -> Result<(), String> {
    let bank_dir = if id.contains('.') {
        let mut parts = id.splitn(2, '.');
        let publisher = parts.next().unwrap_or("");
        let name = parts.next().unwrap_or("");
        Path::new(cwd)
            .join("generated")
            .join("banks")
            .join(publisher)
            .join(name)
    } else {
        Path::new(cwd).join("generated").join("banks").join(id)
    };
    if !bank_dir.exists() {
        return Err(format!(
            "Bank '{}' not found under {}",
            id,
            bank_dir.parent().unwrap_or(Path::new("")).to_string_lossy()
        ));
    }
    std::fs::remove_dir_all(&bank_dir)
        .map_err(|e| format!("Failed to remove {}: {}", bank_dir.to_string_lossy(), e))?;
    crate::utils::logger::Logger::new().log_message(
        crate::utils::logger::LogLevel::Success,
        &format!("✅ Deleted bank: {}", bank_dir.to_string_lossy()),
    );
    Ok(())
}

/// Parses the version from the bank.toml content.
///
/// ### Parameters
/// - `toml_text`: The TOML content to parse.
///
fn parse_version_from_bank_toml(toml_text: &str) -> Option<String> {
    if let Ok(doc) = toml::from_str::<BankTomlDoc>(toml_text) {
        if let Some(b) = doc.bank {
            return b.version;
        }
    }
    None
}

/// Writes the version to the bank.toml content.
///
/// ### Parameters
/// - `original`: The original bank version.
/// - `new_version`: The new version to write.
///
fn write_version_in_bank_toml(original: &str, new_version: &str) -> Result<String, String> {
    let mut lines: Vec<String> = original.lines().map(|s| s.to_string()).collect();
    let mut in_bank = false;
    let mut bank_start = None::<usize>;
    let mut bank_end = lines.len();
    for (i, l) in lines.iter().enumerate() {
        let t = l.trim();
        if t == "[bank]" {
            in_bank = true;
            bank_start = Some(i);
            continue;
        }
        if in_bank && t.starts_with('[') && t != "[bank]" {
            bank_end = i;
            break;
        }
    }
    if !in_bank {
        return Err("[bank] section not found".into());
    }
    let start = bank_start.unwrap();
    // Search for version line inside (start, bank_end)
    let mut version_line_idx: Option<usize> = None;
    for (i, line) in lines.iter().enumerate().take(bank_end).skip(start + 1) {
        let t = line.trim();
        if t.starts_with("version") && t.contains('=') {
            version_line_idx = Some(i);
            break;
        }
    }

    let version_line = format!("version = \"{}\"", new_version);
    match version_line_idx {
        Some(i) => {
            // Replace in place, keep indentation
            let indent = lines[i]
                .chars()
                .take_while(|c| c.is_whitespace())
                .collect::<String>();
            lines[i] = format!("{}{}", indent, version_line);
        }
        None => {
            // Insert before the blank line that separates bank and next section (if any)
            // Find last non-empty line inside bank block
            let mut insert_at = bank_end;
            for (i, line) in lines.iter().enumerate().take(bank_end).skip(start + 1) {
                if line.trim().is_empty() {
                    insert_at = i;
                    break;
                }
            }
            if insert_at == bank_end {
                insert_at = bank_end;
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
