use crate::{
    types::addon::AddonInfo,
    utils::fs::{get_cwd, is_ignored_component, path_relative_to, to_unix_string, walk_files},
};
use std::path::PathBuf;

pub async fn discover_addons() -> Result<Vec<AddonInfo>, String> {
    let cwd = get_cwd()?;
    let mut generated_path = PathBuf::from(&cwd);
    generated_path.push("generated");

    if !generated_path.exists() {
        return Err(format!(
            "Addons directory does not exist: {}",
            generated_path.display()
        ));
    }

    let mut addons: Vec<AddonInfo> = Vec::new();

    for cat_entry in std::fs::read_dir(&generated_path)
        .map_err(|e| format!("Failed to read generated directory: {}", e))?
    {
        let cat_entry = cat_entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
        let cat_path = cat_entry.path();
        if !cat_path.is_dir() {
            continue;
        }

        let category = match cat_path.file_name().and_then(|s| s.to_str()) {
            Some(n) => n,
            None => {
                continue;
            }
        };

        if is_ignored_component(category) {
            continue;
        }

        let mut found_subdirs = Vec::new();
        for addon_entry in std::fs::read_dir(&cat_path)
            .map_err(|e| format!("Failed to read category directory '{}': {}", category, e))?
        {
            let addon_entry = match addon_entry {
                Ok(e) => e,
                Err(_) => {
                    continue;
                }
            };
            let addon_path = addon_entry.path();
            if addon_path.is_dir() {
                if let Some(n) = addon_path.file_name().and_then(|s| s.to_str()) {
                    if !is_ignored_component(n) {
                        found_subdirs.push(addon_path);
                    }
                }
            }
        }

        if found_subdirs.is_empty() {
            let mut files: Vec<String> = Vec::new();
            for f in walk_files(&cat_path)? {
                if let Some(rel) = path_relative_to(&f, &cat_path) {
                    let components_ok = rel.iter().all(|comp| {
                        comp.to_str()
                            .map(|s| !is_ignored_component(s))
                            .unwrap_or(true)
                    });

                    if !components_ok {
                        continue;
                    }

                    files.push(to_unix_string(rel));
                }
            }

            let addon_type = if category.ends_with('s') && category.len() > 1 {
                category[..category.len() - 1].to_string()
            } else {
                category.to_string()
            };

            addons.push(AddonInfo {
                addon_type,
                name: category.into(),
                path: cat_path.to_string_lossy().to_string(),
                files,
            });
        } else {
            for addon_path in found_subdirs {
                // Determine if addon_path is itself an addon (contains plugin.toml/bank.toml)
                let is_plugin_manifest = addon_path.join("plugin.toml").exists();
                let is_bank_manifest = addon_path.join("bank.toml").exists();

                if is_plugin_manifest || is_bank_manifest {
                    // addon_path is the addon (flat layout)
                    let addon_name = match addon_path.file_name().and_then(|s| s.to_str()) {
                        Some(n) => n,
                        None => continue,
                    };

                    let mut files: Vec<String> = Vec::new();
                    for f in walk_files(&addon_path)? {
                        if let Some(rel) = path_relative_to(&f, &addon_path) {
                            let components_ok = rel.iter().all(|comp| {
                                comp.to_str()
                                    .map(|s| !is_ignored_component(s))
                                    .unwrap_or(true)
                            });

                            if !components_ok {
                                continue;
                            }

                            files.push(to_unix_string(rel));
                        }
                    }

                    let addon_type = if category.ends_with('s') && category.len() > 1 {
                        category[..category.len() - 1].to_string()
                    } else {
                        category.to_string()
                    };

                    addons.push(AddonInfo {
                        addon_type,
                        name: addon_name.into(),
                        path: addon_path.to_string_lossy().to_string(),
                        files,
                    });
                } else {
                    // Treat addon_path as a publisher directory and look for its immediate child addon dirs
                    // Layout expected: generated/<type>/<publisher>/<name> where <name> contains plugin.toml/bank.toml
                    if let Ok(pub_entries) = std::fs::read_dir(&addon_path) {
                        for pub_entry in pub_entries.flatten() {
                            let pub_path = pub_entry.path();
                            if !pub_path.is_dir() {
                                continue;
                            }

                            // Accept immediate child dirs that contain addon manifest files
                            if !(pub_path.join("plugin.toml").exists()
                                || pub_path.join("bank.toml").exists())
                            {
                                continue;
                            }

                            let addon_name = match pub_path.file_name().and_then(|s| s.to_str()) {
                                Some(n) => n,
                                None => continue,
                            };

                            let mut files: Vec<String> = Vec::new();
                            for f in walk_files(&pub_path)? {
                                if let Some(rel) = path_relative_to(&f, &pub_path) {
                                    let components_ok = rel.iter().all(|comp| {
                                        comp.to_str()
                                            .map(|s| !is_ignored_component(s))
                                            .unwrap_or(true)
                                    });

                                    if !components_ok {
                                        continue;
                                    }

                                    files.push(to_unix_string(rel));
                                }
                            }

                            let addon_type = if category.ends_with('s') && category.len() > 1 {
                                category[..category.len() - 1].to_string()
                            } else {
                                category.to_string()
                            };

                            addons.push(AddonInfo {
                                addon_type,
                                name: addon_name.into(),
                                path: pub_path.to_string_lossy().to_string(),
                                files,
                            });
                        }
                    }
                }
            }
        }
    }

    Ok(addons)
}
