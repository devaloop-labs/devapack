use std::path::PathBuf;

use crate::{
    types::addon::AddonSubmissionData,
    utils::{
        api::get_forge_api_base_url,
        fs::{get_user_home, is_ignored_component, path_relative_to, walk_files},
    },
};
use base64::{Engine as _, engine::general_purpose};
use ed25519_dalek::{Keypair, Signer};
use flate2::Compression;
use flate2::GzBuilder;
use flate2::read::GzDecoder;
use hex;
use reqwest::multipart::{Form, Part};
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use tar::Builder as TarBuilder;

pub async fn post_update_addon_to_forge_api(
    addon_data: &AddonSubmissionData,
) -> Result<
    (
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
    ),
    String,
> {
    let client = reqwest::Client::new();
    let addon_id = match &addon_data.id {
        Some(id) => id,
        None => {
            return Err("Addon ID is required for update.".to_string());
        }
    };

    let forge_api_url = format!("{}/v1/addon/update/{}", get_forge_api_base_url(), addon_id);

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

    // Build multipart form: metadata fields + multiple files
    let mut form = Form::new()
        .text("name", addon_data.name.clone())
        .text("type", addon_data.addon_type.clone())
        .text("publisher", addon_data.publisher.clone())
        .text("version", addon_data.version.clone())
        .text("access", addon_data.access.clone())
        .text("user_session", user_session_token.to_string());

    // prepare holders for signature/pubkey/sha to return to caller
    let mut ret_signature: Option<String> = None;
    let mut ret_pubkey: Option<String> = None;
    let mut ret_sha: Option<String> = None;

    // Create a single tar.gz archive in memory containing all files under addon_data.path
    let base_path = PathBuf::from(&addon_data.path);
    if base_path.exists() && base_path.is_dir() {
        // Create tar builder writing into a gzip encoder over a Vec<u8>
        let mut tar_buf: Vec<u8> = Vec::new();
        let enc = GzBuilder::new()
            .mtime(0)
            .write(&mut tar_buf, Compression::default());
        let mut tar = TarBuilder::new(enc);

        for f in walk_files(&base_path)? {
            if f.is_file() {
                if let Some(rel) = path_relative_to(&f, &base_path) {
                    let skip = rel
                        .iter()
                        .any(|comp| comp.to_str().map(is_ignored_component).unwrap_or(false));

                    if skip {
                        continue;
                    }

                    // Append file to tar with its relative path (unix separators)
                    let mut file = std::fs::File::open(&f)
                        .map_err(|e| format!("Failed to open file '{}': {}", f.display(), e))?;
                    let mut header_path = rel.to_string_lossy().into_owned();
                    // Ensure unix separators in tar
                    header_path = header_path.replace('\\', "/");
                    tar.append_file(header_path, &mut file).map_err(|e| {
                        format!("Failed to append file to tar '{}': {}", f.display(), e)
                    })?;
                }
            }
        }

        // Finish tar and gzip encoder
        let enc = tar
            .into_inner()
            .map_err(|e| format!("Failed to finish tar: {}", e))?;
        enc.finish()
            .map_err(|e| format!("Failed to finish gzip: {}", e))?;

        // tar_buf now contains the gzipped tar archive
        let part = Part::bytes(tar_buf).file_name("source.tar.gz".to_string());
        form = form.part("files", part);

        // Try to attach the built addon archive from output/.
        // New format: output/<type>/<publisher>.<name>.tar.gz
        // Keep backward compatibility with legacy suffixes
        let cwd_path = crate::utils::fs::get_cwd()?;
        let out_dir = cwd_path.join("output").join(&addon_data.addon_type);
        if out_dir.exists() && out_dir.is_dir() {
            if let Ok(entries) = std::fs::read_dir(&out_dir) {
                for entry in entries.flatten() {
                    let p = entry.path();
                    if p.is_file() {
                        if let Some(fname) = p.file_name().and_then(|s| s.to_str()) {
                            // Prefer .tar.gz archives, accept legacy names as fallback
                            if fname.ends_with(".tar.gz")
                                || fname.ends_with(".devabank.tar.gz")
                                || fname.ends_with(".devaplugin.tar.gz")
                                || fname.ends_with(".devabank")
                                || fname.ends_with(".devaplugin")
                            {
                                if let Ok(mut f) = std::fs::File::open(&p) {
                                    let mut file_bytes: Vec<u8> = Vec::new();
                                    if f.read_to_end(&mut file_bytes).is_ok() {
                                        let (raw_buf, gz_buf): (Vec<u8>, Vec<u8>) = if fname
                                            .ends_with(".tar.gz")
                                            || fname.ends_with(".gz")
                                            || fname.ends_with(".devabank.tar.gz")
                                            || fname.ends_with(".devaplugin.tar.gz")
                                        {
                                            // file is already gzipped : use file bytes as gz_buf and decompress for raw_buf
                                            let gz = file_bytes.clone();
                                            let mut dec = GzDecoder::new(&gz[..]);
                                            let mut raw = Vec::new();
                                            if dec.read_to_end(&mut raw).is_err() {
                                                // fallback: treat file as raw (no decompression)
                                                (file_bytes.clone(), gz)
                                            } else {
                                                (raw, gz)
                                            }
                                        } else {
                                            // file is raw: gzip it
                                            let mut gz_buf: Vec<u8> = Vec::new();
                                            let mut enc = GzBuilder::new()
                                                .mtime(0)
                                                .write(&mut gz_buf, Compression::default());
                                            if enc.write_all(&file_bytes).is_err()
                                                || enc.finish().is_err()
                                            {
                                                (file_bytes.clone(), Vec::new())
                                            } else {
                                                (file_bytes.clone(), gz_buf)
                                            }
                                        };

                                        if gz_buf.is_empty() {
                                            continue;
                                        }

                                        // Compute SHA256 of raw archive bytes (before gzip)
                                        let mut hasher = Sha256::new();
                                        hasher.update(&raw_buf);
                                        let sha = hasher.finalize();
                                        let sha_hex = hex::encode(sha);

                                        // Also compute SHA256 of the gzipped bytes (what we actually send)
                                        let mut hasher_gz = Sha256::new();
                                        hasher_gz.update(&gz_buf);
                                        let sha_gz = hasher_gz.finalize();
                                        let sha_gz_hex = hex::encode(sha_gz);

                                        // Sign using shared helper (if key exists)
                                        let (
                                            signature_b64_opt,
                                            _signature_gz_b64_opt,
                                            pubkey_b64_opt,
                                        ) = crate::addon::self_sign::sign_two_shas(&sha, &sha_gz)
                                            .unwrap_or_default();

                                        // Attach the archive (gzipped)
                                        let part = Part::bytes(gz_buf.clone())
                                            .file_name("archive.tar.gz".to_string());
                                        form = form.part("files", part);

                                        // Attach signature fields
                                        if let Some(sig_b64) = signature_b64_opt.clone() {
                                            form = form.text("signature", sig_b64);
                                        }
                                        // gz signature
                                        if let Ok(home2) = crate::utils::fs::get_user_home() {
                                            let key_path2 = home2
                                                .join(".devalang")
                                                .join("keys")
                                                .join("ed25519.key");
                                            if key_path2.exists() {
                                                if let Ok(bytes2) = std::fs::read(&key_path2) {
                                                    if bytes2.len() == 64 {
                                                        if let Ok(kp2) =
                                                            Keypair::from_bytes(&bytes2)
                                                        {
                                                            let sig_gz_b64 =
                                                                general_purpose::STANDARD.encode(
                                                                    kp2.sign(&sha_gz).to_bytes(),
                                                                );
                                                            form = form
                                                                .text("signature_gzip", sig_gz_b64);
                                                        }
                                                    } else if bytes2.len() == 32 {
                                                        if let Ok(sk2) =
                                                            ed25519_dalek::SecretKey::from_bytes(
                                                                &bytes2,
                                                            )
                                                        {
                                                            let public2 =
                                                                ed25519_dalek::PublicKey::from(
                                                                    &sk2,
                                                                );
                                                            let kp2 = Keypair {
                                                                secret: sk2,
                                                                public: public2,
                                                            };
                                                            let sig_gz_b64 =
                                                                general_purpose::STANDARD.encode(
                                                                    kp2.sign(&sha_gz).to_bytes(),
                                                                );
                                                            form = form
                                                                .text("signature_gzip", sig_gz_b64);
                                                        }
                                                    }
                                                }
                                            }
                                        }

                                        if let Some(pub_b64) = pubkey_b64_opt.clone() {
                                            form = form.text("public_key", pub_b64);
                                        }
                                        form = form.text("archive_sha256", sha_hex.clone());
                                        form = form.text("archive_gzip_sha256", sha_gz_hex.clone());

                                        // store to return (raw)
                                        ret_signature = signature_b64_opt.clone();
                                        ret_pubkey = pubkey_b64_opt.clone();
                                        ret_sha = Some(sha_hex.clone());
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    let response = client
        .post(forge_api_url)
        .headers({
            let mut headers = reqwest::header::HeaderMap::new();
            headers.insert(
                "Authorization",
                format!("Bearer {}", user_session_token).parse().unwrap(),
            );
            headers
        })
        .multipart(form)
        .send()
        .await
        .map_err(|e| format!("Failed to send request: {}", e))?;

    if !response.status().is_success() {
        // Try to parse a structured JSON error from the API and present it nicely.
        let body = response.text().await.unwrap_or_default();
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&body) {
            // Prefer the top-level message if present
            let main_msg = json.get("message").and_then(|v| v.as_str()).unwrap_or("");

            let mut out = String::new();
            if !main_msg.is_empty() {
                out.push_str(main_msg);
            }

            // If payload.errors is an array, list each as `-> CODE : message`
            if let Some(payload) = json.get("payload") {
                if let Some(errors) = payload.get("errors").and_then(|e| e.as_array()) {
                    for err in errors {
                        let code = err
                            .get("code")
                            .and_then(|v| v.as_str())
                            .unwrap_or("UNKNOWN");
                        let msg = err.get("message").and_then(|v| v.as_str()).unwrap_or("");
                        if out.is_empty() {
                            out.push_str(&format!("{} : {}", code, msg));
                        } else {
                            out.push_str(&format!("\n-> {} : {}", code, msg));
                        }
                    }
                }
            }

            // Fallback to raw body when nothing meaningful extracted
            if out.is_empty() {
                return Err(body);
            }

            return Err(out);
        }

        return Err(body);
    }

    let body = response.text().await.unwrap_or_default();
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&body) {
        let fetched_addon_id = json
            .get("payload")
            .unwrap_or(&serde_json::Value::Null)
            .get("addon_id")
            .map(|v| v.to_string())
            .ok_or("Response JSON missing 'addon_id' field".to_string())?;

        Ok((Some(fetched_addon_id), ret_signature, ret_pubkey, ret_sha))
    } else {
        Err("Failed to parse response JSON".to_string())
    }
}
