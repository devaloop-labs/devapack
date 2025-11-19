use crate::utils::{
    fs as ufs,
    logger::{LogLevel, Logger},
    spinner,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use wasmparser::{ExternalKind, Parser, Payload};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct PluginSection {
    name: String,
    publisher: String,
    #[serde(default)]
    _description: Option<String>,
    #[serde(default)]
    version: Option<String>,
    #[serde(default)]
    access: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct ExportEntryToml {
    name: String,
    kind: String, // func | global | memory | table
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct PluginTomlDoc {
    plugin: PluginSection,
    #[serde(default)]
    exports: Vec<ExportEntryToml>,
}

pub fn build_plugin(
    path: &str,
    release: &bool,
    cwd: &str,
    require_signature: bool,
    show_summary: bool,
) -> Result<(), String> {
    let plugin_dir = spinner::run_step(
        &format!("Resolving plugin directory for '{}'", path),
        |dir: &std::path::PathBuf| format!("Using {}", dir.to_string_lossy()),
        || resolve_plugin_dir(cwd, path),
    )?;

    let plugin_toml_path = plugin_dir.join("plugin.toml");

    spinner::run_unit_step(
        &format!("Checking manifest at {}", plugin_toml_path.display()),
        "Manifest located",
        || {
            if plugin_toml_path.exists() {
                Ok(())
            } else {
                Err(format!(
                    "plugin.toml not found in {}",
                    plugin_dir.to_string_lossy()
                ))
            }
        },
    )?;

    let plugin_doc: PluginTomlDoc = spinner::run_step(
        "Parsing plugin manifest",
        |_| "Manifest parsed".to_string(),
        || {
            let txt = fs::read_to_string(&plugin_toml_path)
                .map_err(|e| format!("Failed to read plugin.toml: {}", e))?;
            toml::from_str(&txt).map_err(|e| format!("Invalid TOML: {}", e))
        },
    )?;

    let publisher = plugin_doc.plugin.publisher.trim().to_string();
    let name = plugin_doc.plugin.name.trim().to_string();

    spinner::run_unit_step(
        "Validating manifest metadata",
        "Manifest metadata valid",
        || {
            if publisher.is_empty() || name.is_empty() {
                Err(
                    "Fields [plugin].publisher and [plugin].name are required in plugin.toml"
                        .into(),
                )
            } else {
                Ok(())
            }
        },
    )?;

    let out_root = Path::new(cwd).join("output").join("plugin");
    spinner::run_unit_step(
        &format!("Ensuring output directory {}", out_root.display()),
        "Output directory ready",
        || {
            fs::create_dir_all(&out_root)
                .map_err(|e| format!("Failed to create output directory: {}", e))
        },
    )?;

    spinner::run_unit_step(
        "Running cargo build (wasm32-unknown-unknown)",
        "Compilation finished",
        || {
            let mut cmd = std::process::Command::new("cargo");
            cmd.current_dir(&plugin_dir);
            cmd.arg("build");
            cmd.arg("--target");
            cmd.arg("wasm32-unknown-unknown");
            if *release {
                cmd.arg("--release");
            }
            let status = cmd
                .status()
                .map_err(|e| format!("Failed to run cargo build: {}", e))?;
            if !status.success() {
                return Err(format!("cargo build failed for plugin: exit={}", status));
            }
            Ok(())
        },
    )?;

    // Produce archive as <publisher>.<name>.tar.gz (no .devaplugin suffix)
    let out_file = out_root.join(format!("{}.{}.tar.gz", publisher, name));

    spinner::run_unit_step(
        &format!(
            "Packaging artifact {}",
            out_file
                .file_name()
                .and_then(|f| f.to_str())
                .unwrap_or("archive")
        ),
        "Archive created",
        || {
            create_plugin_tar_gz_wasm_only(
                &plugin_toml_path,
                &out_file,
                &name,
                &publisher,
                plugin_doc.plugin._description.clone(),
                &plugin_dir,
                *release,
            )
        },
    )?;

    if require_signature {
        // signature file uses the same base name and `.tar.gz.sig` suffix
        let sig_path = out_root.join(format!("{}.{}.tar.gz.sig", publisher, name));
        spinner::run_unit_step(
            &format!("Checking signature at {}", sig_path.display()),
            "Signature present",
            || {
                if sig_path.exists() {
                    Ok(())
                } else {
                    Err(format!(
                        "Signing required but signature file not found at {}",
                        sig_path.display()
                    ))
                }
            },
        )?;
    }

    if show_summary {
        Logger::new().log_message(
            LogLevel::Success,
            &format!("Plugin built at {}", out_file.to_string_lossy()),
        );

        if let Err(e) = print_artifact_summary(&out_file) {
            Logger::new().log_message(
                LogLevel::Warning,
                &format!("Failed to print summary: {}", e),
            );
        }
    }

    Ok(())
}
pub fn build_all_plugins(release: &bool, cwd: &str, require_signature: bool) -> Result<(), String> {
    let plugins_root = Path::new(cwd).join("generated").join("plugins");
    if !plugins_root.exists() {
        return Err(format!(
            "Plugins directory not found: {}",
            plugins_root.to_string_lossy()
        ));
    }
    // Recursively find all plugin.toml files under generated/plugins and collect their parent dirs.
    let mut dirs: Vec<PathBuf> = Vec::new();
    let files = ufs::walk_files(&plugins_root)?;
    for f in files {
        if f.file_name()
            .and_then(|s| s.to_str())
            .map(|s| s == "plugin.toml")
            .unwrap_or(false)
        {
            if let Some(parent) = f.parent() {
                dirs.push(parent.to_path_buf());
            }
        }
    }
    // Deduplicate and sort
    dirs.sort();
    dirs.dedup();
    if dirs.is_empty() {
        return Err("No plugins to build (generated/plugins is empty)".into());
    }

    let mut errors: Vec<String> = Vec::new();
    let mut successes: Vec<String> = Vec::new();
    let total = dirs.len();
    for p in dirs {
        let p_str = p.to_string_lossy().to_string();
        match build_plugin(&p_str, release, cwd, require_signature, true) {
            Ok(_) => successes.push(p_str.clone()),
            Err(e) => errors.push(format!("{} -> {}", p_str, e)),
        }
    }

    // Summary info
    Logger::new().log_message(LogLevel::Info, &format!("{} addons built", successes.len()));

    let mut trace_lines: Vec<String> = Vec::new();
    for s in &successes {
        trace_lines.push(format!("Built: {}", s));
    }
    for e in &errors {
        trace_lines.push(format!("Failed: {}", e));
    }
    let trace_refs: Vec<&str> = trace_lines.iter().map(|s| s.as_str()).collect();
    Logger::new().log_message_with_trace(LogLevel::Info, "Build details:", trace_refs);

    if errors.is_empty() {
        Logger::new().log_message(
            LogLevel::Success,
            &format!("Build complete: {} plugin(s) built", total),
        );
        Ok(())
    } else {
        let joined = errors.join("\n - ");
        Err(format!(
            "Some plugins failed ({}/{}):\n - {}",
            errors.len(),
            total,
            joined
        ))
    }
}

fn resolve_plugin_dir(cwd: &str, input: &str) -> Result<PathBuf, String> {
    let candidate = Path::new(cwd).join(input);
    if candidate.is_file()
        && candidate
            .file_name()
            .map(|f| f == "plugin.toml")
            .unwrap_or(false)
    {
        return Ok(candidate.parent().unwrap().to_path_buf());
    }
    if candidate.is_dir() && candidate.join("plugin.toml").exists() {
        return Ok(candidate);
    }

    if let Some(rest) = input.strip_prefix("plugin.") {
        // Support `plugin.<publisher>.<name>` which maps to generated/plugins/<publisher>/<name>
        let root = Path::new(cwd).join("generated").join("plugins");

        // If rest contains a dot, assume publisher.name form
        if rest.contains('.') {
            let mut parts = rest.splitn(2, '.');
            let publisher = parts.next().unwrap_or("");
            let name = parts.next().unwrap_or("");
            let nested = root.join(publisher).join(name);
            if nested.join("plugin.toml").exists() {
                return Ok(nested);
            }
        } else {
            // alias: plugin.<name> -> search for directories named <name> under any publisher
            if let Ok(mut matches) = (|| -> Result<Vec<PathBuf>, std::io::Error> {
                let mut found = Vec::new();
                for pub_entry in fs::read_dir(&root)? {
                    let ppub = pub_entry?.path();
                    if ppub.is_dir() {
                        let child = ppub.join(rest);
                        if child.join("plugin.toml").exists() {
                            found.push(child);
                        }
                    }
                }
                Ok(found)
            })() {
                return match matches.len() {
                    1 => Ok(matches.remove(0)),
                    0 => Err(format!(
                        "No plugin matched alias plugin.{} under {}",
                        rest,
                        root.to_string_lossy()
                    )),
                    _ => Err(format!(
                        "Multiple plugins matched plugin.{}; use 'plugin.<publisher>.<name>'",
                        rest
                    )),
                };
            }
        }
        return Err(format!(
            "Alias not found: {}; expected under {}",
            input,
            root.to_string_lossy()
        ));
    }
    Err(format!(
        "Invalid path: {} (no plugin.toml found)",
        candidate.to_string_lossy()
    ))
}

#[allow(dead_code)]
fn create_plugin_zip(
    plugin_toml_path: &Path,
    out_zip: &Path,
    name: &str,
    publisher: &str,
    description: Option<String>,
    plugin_dir: &Path,
) -> Result<(), String> {
    let file =
        fs::File::create(out_zip).map_err(|e| format!("Failed to create output file: {}", e))?;
    let mut zip = zip::ZipWriter::new(file);
    let options =
        zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    // Add plugin.toml at root
    zip.start_file("plugin.toml", options)
        .map_err(|e| format!("Failed to zip plugin.toml: {}", e))?;
    let mut toml_bytes = Vec::new();
    fs::File::open(plugin_toml_path)
        .and_then(|mut f| f.read_to_end(&mut toml_bytes))
        .map_err(|e| format!("Failed to read plugin.toml: {}", e))?;
    zip.write_all(&toml_bytes)
        .map_err(|e| format!("Failed to write plugin.toml to zip: {}", e))?;

    // Add README.md (from plugin dir if present, else default)
    zip.start_file("README.md", options)
        .map_err(|e| format!("Failed to add README.md: {}", e))?;
    let readme_path = plugin_dir.join("README.md");
    if readme_path.exists() {
        let mut buf = Vec::new();
        fs::File::open(&readme_path)
            .and_then(|mut f| f.read_to_end(&mut buf))
            .map_err(|e| format!("Failed to read README.md: {}", e))?;
        zip.write_all(&buf)
            .map_err(|e| format!("Failed to write README.md: {}", e))?;
    } else {
        let readme = default_readme_plugin(publisher, name, description.as_deref());
        zip.write_all(readme.as_bytes())
            .map_err(|e| format!("Failed to write README.md: {}", e))?;
    }

    // Add LICENSE (from plugin dir if present, else default MIT)
    zip.start_file("LICENSE", options)
        .map_err(|e| format!("Failed to add LICENSE: {}", e))?;
    let license_path = plugin_dir.join("LICENSE");
    if license_path.exists() {
        let mut buf = Vec::new();
        fs::File::open(&license_path)
            .and_then(|mut f| f.read_to_end(&mut buf))
            .map_err(|e| format!("Failed to read LICENSE: {}", e))?;
        zip.write_all(&buf)
            .map_err(|e| format!("Failed to write LICENSE: {}", e))?;
    } else {
        let license = default_mit_license(publisher);
        zip.write_all(license.as_bytes())
            .map_err(|e| format!("Failed to write LICENSE: {}", e))?;
    }

    // Add source tree: Cargo.toml, src/, and any other files in plugin_dir except target/
    let files = ufs::walk_files(plugin_dir)?;
    for p in files {
        if !p.is_file() {
            continue;
        }
        // Skip target directory files
        if p.components().any(|c| c.as_os_str() == "target") {
            continue;
        }
        // Compute path relative to plugin_dir and write under `source/` in the zip
        let rel_os = ufs::path_relative_to(&p, plugin_dir).unwrap_or_else(|| {
            p.file_name()
                .map(PathBuf::from)
                .unwrap_or_else(PathBuf::new)
        });
        let rel = ufs::to_unix_string(&rel_os);
        let mut data = Vec::new();
        fs::File::open(&p)
            .and_then(|mut f| f.read_to_end(&mut data))
            .map_err(|e| format!("Failed to read file: {}", e))?;
        // Write files directly at the archive root while preserving relative paths
        // (e.g. `src/lib.rs` will appear as `src/lib.rs` at the archive root).
        let zip_path = rel.clone();
        zip.start_file(zip_path.clone(), options)
            .map_err(|e| format!("Failed to add {}: {}", zip_path, e))?;
        zip.write_all(&data)
            .map_err(|e| format!("Failed to write {}: {}", zip_path, e))?;
    }

    zip.finish()
        .map_err(|e| format!("Failed to finalize zip: {}", e))?;
    let _ = fs::metadata(out_zip).map_err(|e| format!("Failed to stat zip: {}", e))?;
    Ok(())
}

#[allow(dead_code)]
fn default_readme_plugin(publisher: &str, name: &str, description: Option<&str>) -> String {
    let desc = description.unwrap_or("Plugin for Devalang.");
    format!(
        "# {}.{} Plugin\n\n{}\n\nContents:\n- plugin.toml\n- sources (placed at archive root, e.g. src/..., Cargo.toml)\n- LICENSE\n\nBuilt with devapack.\n",
        publisher, name, desc
    )
}

fn default_mit_license(publisher: &str) -> String {
    format!(
        "MIT License\n\nCopyright (c) {}\n\nPermission is hereby granted, free of charge, to any person obtaining a copy\n of this software and associated documentation files (the \"Software\"), to deal\n in the Software without restriction, including without limitation the rights\n to use, copy, modify, merge, publish, distribute, sublicense, and/or sell\n copies of the Software, and to permit persons to whom the Software is\n furnished to do so, subject to the following conditions:\n\nThe above copyright notice and this permission notice shall be included in all\n copies or substantial portions of the Software.\n\nTHE SOFTWARE IS PROVIDED \"AS IS\", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR\n IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,\n FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE\n publisherS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER\n LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,\n OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE\n SOFTWARE.\n",
        publisher
    )
}

#[allow(dead_code)]
fn create_plugin_tar_gz_native(
    plugin_toml_path: &Path,
    out_zip: &Path,
    name: &str,
    publisher: &str,
    _description: Option<String>,
    plugin_dir: &Path,
    release: bool,
) -> Result<(), String> {
    // Localiser la bibliothèque native (DLL sur Windows, SO sur Linux, DYLIB sur macOS)
    let profile = if release { "release" } else { "debug" };

    // Déterminer l'extension selon la plateforme
    #[cfg(target_os = "windows")]
    let lib_ext = "dll";
    #[cfg(target_os = "linux")]
    let lib_ext = "so";
    #[cfg(target_os = "macos")]
    let lib_ext = "dylib";

    // Sur certaines plateformes, la bibliothèque peut avoir le préfixe "lib"
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    let lib_prefix = "lib";
    #[cfg(target_os = "windows")]
    let lib_prefix = "";

    let lib_name = format!("{}{}.{}", lib_prefix, name.replace('-', "_"), lib_ext);
    let lib_path = plugin_dir.join("target").join(profile).join(&lib_name);

    if !lib_path.exists() {
        return Err(format!(
            "Native library not found: {}",
            lib_path.to_string_lossy()
        ));
    }

    // Si un ancien artifact existe, le supprimer pour éviter les erreurs de verrouillage Windows
    if out_zip.exists() {
        fs::remove_file(out_zip)
            .map_err(|e| format!("Failed to remove existing output file: {}", e))?;
    }

    // Lire le plugin.toml original pour préserver les métadonnées [plugin]
    let toml_txt = fs::read_to_string(plugin_toml_path)
        .map_err(|e| format!("Failed to read plugin.toml: {}", e))?;

    let plugin_doc: Option<PluginTomlDoc> = toml::from_str(&toml_txt).ok();

    // Scanner les sources du plugin pour les macros export_plugin!(name, ...)
    let mut attribute_exports: Vec<String> = Vec::new();
    let src_root = plugin_dir.join("src");
    if src_root.exists() {
        if let Ok(files) = ufs::walk_files(&src_root) {
            for f in files {
                if !f.is_file() {
                    continue;
                }
                if let Some(ext) = f.extension().and_then(|s| s.to_str()) {
                    if ext != "rs" {
                        continue;
                    }
                } else {
                    continue;
                }
                if let Ok(s) = fs::read_to_string(&f) {
                    // Chercher export_plugin!(name, ...) ou export_plugin_ext!(name, ...)
                    let mut pos = 0usize;
                    while let Some(idx) = s[pos..].find("export_plugin") {
                        let start = pos + idx + "export_plugin".len();
                        // Vérifier si c'est export_plugin! ou export_plugin_ext!
                        let rest = &s[start..];
                        if !rest.starts_with("!") && !rest.starts_with("_ext!") {
                            pos = start;
                            continue;
                        }

                        // Sauter jusqu'à la parenthèse ouvrante
                        if let Some(paren_idx) = s[start..].find('(') {
                            let name_start = start + paren_idx + 1;
                            // Trouver la virgule ou la parenthèse fermante
                            if let Some(comma_idx) = s[name_start..].find(',') {
                                let name = s[name_start..name_start + comma_idx].trim();
                                if !name.is_empty() {
                                    attribute_exports.push(name.to_string());
                                }
                                pos = name_start + comma_idx;
                            } else {
                                break;
                            }
                        } else {
                            break;
                        }
                    }
                }
            }
        }
    }

    // Dédupliquer et trier pour une sortie stable
    attribute_exports.sort();
    attribute_exports.dedup();

    // Reconstruire le contenu plugin.toml : conserver la section [plugin] et remplacer exports
    let mut out_toml = String::new();
    if let Some(doc) = plugin_doc {
        // Construire l'en-tête du plugin
        out_toml.push_str("[plugin]\n");
        out_toml.push_str(&format!("name = \"{}\"\n", doc.plugin.name));
        out_toml.push_str(&format!("publisher = \"{}\"\n", doc.plugin.publisher));
        if let Some(d) = doc.plugin._description {
            out_toml.push_str(&format!("description = \"{}\"\n", d));
        }
        if let Some(v) = doc.plugin.version {
            out_toml.push_str(&format!("version = \"{}\"\n", v));
        }
        if let Some(a) = doc.plugin.access {
            out_toml.push_str(&format!("access = \"{}\"\n", a));
        }
    } else {
        // Fallback : écrire les lignes d'en-tête plugin.toml originales
        if let Some(idx) = toml_txt.find("[[exports]]") {
            out_toml.push_str(&toml_txt[..idx]);
        } else {
            out_toml.push_str(&toml_txt);
        }
    }

    for name_export in attribute_exports {
        out_toml.push_str("\n[[exports]]\n");
        out_toml.push_str(&format!("name = \"{}\"\nkind = \"func\"\n", name_export));
    }

    // Réécrire le plugin.toml source pour que generated/plugins/<publisher>/<name>/plugin.toml
    // soit mis à jour pour refléter les exports reconstruits
    fs::write(plugin_toml_path, &out_toml)
        .map_err(|e| format!("Failed to write plugin.toml back to source: {}", e))?;

    use flate2::{Compression, write::GzEncoder};
    use std::fs::File;
    use tar::Builder;

    let f = File::create(out_zip).map_err(|e| format!("Failed to create output file: {}", e))?;
    let enc = GzEncoder::new(f, Compression::default());
    let mut tar = Builder::new(enc);

    // plugin.toml
    tar.append_path_with_name(plugin_toml_path, "plugin.toml")
        .map_err(|e| format!("Failed to add plugin.toml to tar: {}", e))?;

    // LICENSE
    let license_path = plugin_dir.join("LICENSE");
    if license_path.exists() {
        tar.append_path_with_name(&license_path, "LICENSE")
            .map_err(|e| format!("Failed to add LICENSE to tar: {}", e))?;
    } else {
        let license = default_mit_license(publisher);
        let mut header = tar::Header::new_gnu();
        header.set_size(license.len() as u64);
        header.set_cksum();
        tar.append_data(&mut header, "LICENSE", license.as_bytes())
            .map_err(|e| format!("Failed to append LICENSE data: {}", e))?;
    }

    // Bibliothèque native à la racine
    tar.append_path_with_name(&lib_path, &lib_name)
        .map_err(|e| format!("Failed to add native library to tar: {}", e))?;

    tar.finish()
        .map_err(|e| format!("Failed to finalize tar: {}", e))?;
    Ok(())
}

#[allow(dead_code)]
fn create_plugin_tar_gz_wasm_only(
    plugin_toml_path: &Path,
    out_zip: &Path,
    name: &str,
    publisher: &str,
    _description: Option<String>,
    plugin_dir: &Path,
    release: bool,
) -> Result<(), String> {
    // locate wasm artifact
    let profile = if release { "release" } else { "debug" };
    let wasm_path = plugin_dir
        .join("target")
        .join("wasm32-unknown-unknown")
        .join(profile)
        .join(format!("{}.wasm", name));
    if !wasm_path.exists() {
        return Err(format!(
            "WASM artifact not found: {}",
            wasm_path.to_string_lossy()
        ));
    }

    // If an old artifact exists, remove it first to avoid Windows file-lock errors.
    if out_zip.exists() {
        fs::remove_file(out_zip)
            .map_err(|e| format!("Failed to remove existing output file: {}", e))?;
    }

    // Read wasm bytes early so we can detect exported symbols and build plugin.toml
    let mut wasm_bytes = Vec::new();
    fs::File::open(&wasm_path)
        .and_then(|mut f| f.read_to_end(&mut wasm_bytes))
        .map_err(|e| format!("Failed to read wasm: {}", e))?;

    // Read and parse original plugin.toml to preserve [plugin] metadata.
    let toml_txt = fs::read_to_string(plugin_toml_path)
        .map_err(|e| format!("Failed to read plugin.toml: {}", e))?;

    let plugin_doc: Option<PluginTomlDoc> = toml::from_str(&toml_txt).ok();

    // Scan plugin sources for export_plugin!(name, ...) or export_plugin_ext!(name, ...) macros
    let mut attribute_exports: Vec<String> = Vec::new();
    let src_root = plugin_dir.join("src");
    if src_root.exists() {
        if let Ok(files) = ufs::walk_files(&src_root) {
            for f in files {
                if !f.is_file() {
                    continue;
                }
                if let Some(ext) = f.extension().and_then(|s| s.to_str()) {
                    if ext != "rs" {
                        continue;
                    }
                } else {
                    continue;
                }
                if let Ok(s) = fs::read_to_string(&f) {
                    // Search for export_plugin!(name, ...) or export_plugin_ext!(name, ...) or export_plugin_with_state!(name, ...)
                    let mut pos = 0usize;
                    while let Some(idx) = s[pos..].find("export_plugin") {
                        let start = pos + idx + "export_plugin".len();
                        // Check if it's export_plugin! or export_plugin_ext! or export_plugin_with_state!
                        let rest = &s[start..];
                        if !rest.starts_with("!")
                            && !rest.starts_with("_ext!")
                            && !rest.starts_with("_with_state!")
                        {
                            pos = start;
                            continue;
                        }

                        // Skip to opening parenthesis
                        if let Some(paren_idx) = s[start..].find('(') {
                            let name_start = start + paren_idx + 1;
                            // Find comma or closing paren
                            if let Some(comma_idx) = s[name_start..].find(',') {
                                let name = s[name_start..name_start + comma_idx].trim();
                                if !name.is_empty() {
                                    attribute_exports.push(name.to_string());
                                }
                                pos = name_start + comma_idx;
                            } else {
                                break;
                            }
                        } else {
                            break;
                        }
                    }
                }
            }
        }
    }

    // Parse wasm exports and collect relevant exported function names
    let mut exported_funcs: Vec<String> = Vec::new();
    for payload in Parser::new(0).parse_all(&wasm_bytes).flatten() {
        if let Payload::ExportSection(reader) = payload {
            for exp in reader.into_iter().flatten() {
                if exp.kind == ExternalKind::Func {
                    let name = exp.name.to_string();
                    // include setters and any names declared via attribute
                    if name.starts_with("set_") || attribute_exports.iter().any(|a| a == &name) {
                        exported_funcs.push(name);
                    }
                }
            }
        }
    }

    // Deduplicate and sort for stable output
    exported_funcs.sort();
    exported_funcs.dedup();

    // Rebuild plugin.toml content: keep [plugin] section and replace exports with the detected ones
    let mut out_toml = String::new();
    if let Some(doc) = plugin_doc {
        // Build plugin header
        out_toml.push_str("[plugin]\n");
        out_toml.push_str(&format!("name = \"{}\"\n", doc.plugin.name));
        out_toml.push_str(&format!("publisher = \"{}\"\n", doc.plugin.publisher));
        if let Some(d) = doc.plugin._description {
            out_toml.push_str(&format!("description = \"{}\"\n", d));
        }
        if let Some(v) = doc.plugin.version {
            out_toml.push_str(&format!("version = \"{}\"\n", v));
        }
        if let Some(a) = doc.plugin.access {
            out_toml.push_str(&format!("access = \"{}\"\n", a));
        }
    } else {
        // Fallback: write original plugin.toml header lines (up to first [[exports]] or EOF)
        if let Some(idx) = toml_txt.find("[[exports]]") {
            out_toml.push_str(&toml_txt[..idx]);
        } else {
            out_toml.push_str(&toml_txt);
        }
    }

    for name in exported_funcs {
        out_toml.push_str("\n[[exports]]\n");
        out_toml.push_str(&format!("name = \"{}\"\nkind = \"func\"\n", name));
    }

    // Overwrite the source plugin.toml so generated/plugins/<publisher>/<name>/plugin.toml
    // is updated to reflect the reconstructed exports.
    fs::write(plugin_toml_path, &out_toml)
        .map_err(|e| format!("Failed to write plugin.toml back to source: {}", e))?;

    use flate2::{Compression, write::GzEncoder};
    use std::fs::File;
    use tar::Builder;

    let f = File::create(out_zip).map_err(|e| format!("Failed to create output file: {}", e))?;
    let enc = GzEncoder::new(f, Compression::default());
    let mut tar = Builder::new(enc);

    // plugin.toml
    tar.append_path_with_name(plugin_toml_path, "plugin.toml")
        .map_err(|e| format!("Failed to add plugin.toml to tar: {}", e))?;

    // LICENSE
    let license_path = plugin_dir.join("LICENSE");
    if license_path.exists() {
        tar.append_path_with_name(&license_path, "LICENSE")
            .map_err(|e| format!("Failed to add LICENSE to tar: {}", e))?;
    } else {
        let license = default_mit_license(publisher);
        let mut header = tar::Header::new_gnu();
        header.set_size(license.len() as u64);
        header.set_cksum();
        tar.append_data(&mut header, "LICENSE", license.as_bytes())
            .map_err(|e| format!("Failed to append LICENSE data: {}", e))?;
    }

    // wasm artifact at root
    let wasm_name = format!("{}.wasm", name);
    let mut header = tar::Header::new_gnu();
    header.set_size(wasm_bytes.len() as u64);
    header.set_cksum();
    tar.append_data(&mut header, &wasm_name, &wasm_bytes[..])
        .map_err(|e| format!("Failed to append wasm data: {}", e))?;

    tar.finish()
        .map_err(|e| format!("Failed to finalize tar: {}", e))?;
    Ok(())
}

fn print_artifact_summary(path: &Path) -> Result<(), String> {
    use std::fs::File;
    // compute size
    let meta = fs::metadata(path).map_err(|e| format!("Failed to stat artifact: {}", e))?;
    let size = meta.len();
    // compute sha256
    let mut f = File::open(path).map_err(|e| format!("Failed to open artifact: {}", e))?;
    let mut buf = Vec::new();
    f.read_to_end(&mut buf)
        .map_err(|e| format!("Failed to read artifact for sha: {}", e))?;
    let mut hasher = Sha256::new();
    hasher.update(&buf);
    let sha = hasher.finalize();
    let sha_hex = hex::encode(sha);

    let file_name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");

    let payload = json!({
        "meta": {
            "archive_name": file_name,
            "archive": path.to_string_lossy().to_string(),
            "archive_size": size,
            "checksums": { "sha256": sha_hex }
        }
    });

    crate::addon::summary::print_addon_summary(&payload, Path::new("local"));
    Ok(())
}
