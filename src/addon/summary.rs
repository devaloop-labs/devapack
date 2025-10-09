use crate::utils::logger::{LogLevel, Logger};
use serde_json::Value;
use std::path::Path;

pub fn print_addon_summary(response_json: &Value, key_path: &Path) {
    let logger = Logger::new();
    // Helpers to try multiple nested paths and return a string representation
    fn get_path<'a>(v: &'a Value, path: &[&str]) -> Option<&'a Value> {
        let mut cur = v;
        for p in path {
            cur = cur.get(p)?;
        }
        Some(cur)
    }

    fn get_any_str(v: &Value, candidates: &[&[&str]]) -> Option<String> {
        for c in candidates {
            if let Some(val) = get_path(v, c) {
                if val.is_string() {
                    return val.as_str().map(|s| s.to_string());
                } else {
                    return Some(val.to_string());
                }
            }
        }
        None
    }

    logger.log_message(
        LogLevel::Info,
        &format!("🔑 Using key : {}", key_path.display()),
    );

    // Friendly message if present
    if let Some(msg) = get_any_str(response_json, &[&["message"], &["msg"]]) {
        logger.log_message(LogLevel::Info, &format!("💬 Message : {}", msg));
    }

    if let Some(signer) = get_any_str(response_json, &[&["signer"], &["meta", "signer"]]) {
        logger.log_message(LogLevel::Info, &format!("👤 Signer    : {}", signer));
    }
    if let Some(fp) = get_any_str(response_json, &[&["fingerprint"], &["meta", "fingerprint"]]) {
        logger.log_message(LogLevel::Info, &format!("🔗 Fingerprint: {}", fp));
    }

    // Archive info: show path/size/tarball as a single section with optional trace details
    let mut archive_lines: Vec<String> = Vec::new();
    if let Some(archive) = get_any_str(
        response_json,
        &[
            &["archive_path"],
            &["meta", "archive"],
            &["meta", "archive_name"],
        ],
    ) {
        archive_lines.push(format!("Path    : {}", archive));
    }
    if let Some(size) = get_any_str(
        response_json,
        &[
            &["archive_size"],
            &["meta", "archive_size"],
            &["meta", "tarball_compressed_size"],
            &["size"],
        ],
    ) {
        archive_lines.push(format!("Size    : {}", size));
    }
    if let Some(tarball) = get_any_str(
        response_json,
        &[&["tarball_name"], &["meta", "archive_name"]],
    ) {
        archive_lines.push(format!("Tarball : {}", tarball));
    }
    if !archive_lines.is_empty() {
        let refs: Vec<&str> = archive_lines.iter().map(|s| s.as_str()).collect();
        logger.log_message_with_trace(LogLevel::Info, "📦 Archive", refs);
    }

    // Checksums as trace list (support multiple checksum types in the future)
    if let Some(sha) = get_any_str(
        response_json,
        &[
            &["archive_sha256"],
            &["checksum"],
            &["meta", "checksums", "sha256"],
            &["meta", "checksum"],
        ],
    ) {
        let mut lines: Vec<String> = Vec::new();
        lines.push(format!("SHA256 : {}", sha));
        let refs: Vec<&str> = lines.iter().map(|s| s.as_str()).collect();
        logger.log_message_with_trace(LogLevel::Info, "🧾 Checksums", refs);
    }

    // Signature details (many APIs nest these under meta.signature)
    // Signature: primary message contains status (if available); signed_at goes into trace
    if let Some(sig_status) = get_any_str(
        response_json,
        &[
            &["signature_status"],
            &["meta", "signature", "status"],
            &["meta", "signature", "state"],
        ],
    ) {
        let mut trace_items: Vec<String> = Vec::new();
        if let Some(signed_at) = get_any_str(
            response_json,
            &[
                &["meta", "signature", "signed_at"],
                &["meta", "signature", "signedAt"],
                &["signed_at"],
                &["signature", "signed_at"],
            ],
        ) {
            trace_items.push(format!("Signed at: {}", signed_at));
        }
        let refs: Vec<&str> = trace_items.iter().map(|s| s.as_str()).collect();
        logger.log_message(
            LogLevel::Info,
            &format!("🔐 Signature (Ed25519) • Status: {}", sig_status),
        );
        if !refs.is_empty() {
            logger.log_message_with_trace(LogLevel::Info, "Signature details:", refs);
        }
    } else if let Some(signed_at) = get_any_str(
        response_json,
        &[
            &["meta", "signature", "signed_at"],
            &["signature", "signed_at"],
        ],
    ) {
        logger.log_message(LogLevel::Info, "🔐 Signature (Ed25519)");
        logger.log_message_with_trace(
            LogLevel::Info,
            "Signature details:",
            vec![signed_at.as_str()],
        );
    }

    // Manifest
    if let Some(manifest) = response_json
        .get("manifest")
        .or_else(|| response_json.get("meta").and_then(|m| m.get("manifest")))
    {
        let mut lines: Vec<String> = Vec::new();
        if let Some(name) = manifest.get("name") {
            lines.push(format!(
                "  • name         : \"{}\"",
                name.as_str().unwrap_or("")
            ));
        }
        if let Some(ver) = manifest.get("version") {
            lines.push(format!(
                "  • version      : \"{}\"",
                ver.as_str().unwrap_or("")
            ));
        }
        if let Some(desc) = manifest.get("description") {
            lines.push(format!(
                "  • description : \"{}\"",
                desc.as_str().unwrap_or("")
            ));
        }
        if let Some(access) = manifest.get("access") {
            lines.push(format!(
                "  • access : \"{}\"",
                access.as_str().unwrap_or("")
            ));
        }

        if lines.is_empty() {
            logger.log_message(LogLevel::Info, "📋 Manifest (plugin.toml)  • None");
        } else {
            let refs: Vec<&str> = lines.iter().map(|s| s.as_str()).collect();
            logger.log_message_with_trace(LogLevel::Info, "📋 Manifest (plugin.toml)", refs);
        }
    }

    // Warnings
    if let Some(warn) = response_json
        .get("warnings")
        .or_else(|| response_json.get("meta").and_then(|m| m.get("warnings")))
    {
        if warn.is_array() {
            let arr = warn.as_array().unwrap();
            if arr.is_empty() {
                logger.log_message(LogLevel::Warning, "⚠️ Warnings • None");
            } else {
                let mut lines: Vec<String> = Vec::new();
                for w in arr {
                    lines.push(format!("  • {}", w.as_str().unwrap_or(&w.to_string())));
                }
                let refs: Vec<&str> = lines.iter().map(|s| s.as_str()).collect();
                logger.log_message_with_trace(LogLevel::Warning, "⚠️ Warnings", refs);
            }
        } else {
            logger.log_message(LogLevel::Warning, &format!("⚠️ Warnings • {}", warn));
        }
    }
}
