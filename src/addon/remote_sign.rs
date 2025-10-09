use crate::utils::{api::get_forge_api_base_url, auth};
use reqwest::Client;
use serde_json::json;

pub async fn register_signature_with_server(
    addon_id: &str,
    signature_b64: &str,
    public_b64: &str,
    archive_sha: &str,
) -> Result<serde_json::Value, String> {
    let sign_url = format!("{}/v1/addon/sign/{}", get_forge_api_base_url(), addon_id);
    let token = auth::load_session_token()?;
    let client = Client::new();
    let payload = json!({
        "public_key": public_b64,
        "signature": signature_b64,
        "archive_sha256": archive_sha
    });
    let res = client
        .post(&sign_url)
        .bearer_auth(token)
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("Failed to call sign endpoint: {}", e))?;
    if !res.status().is_success() {
        return Err(format!(
            "Failed to register signature: HTTP {}",
            res.status()
        ));
    }
    let body = res
        .text()
        .await
        .map_err(|e| format!("Failed to read response: {}", e))?;
    let json: serde_json::Value =
        serde_json::from_str(&body).map_err(|e| format!("Failed to parse response JSON: {}", e))?;
    Ok(json)
}
