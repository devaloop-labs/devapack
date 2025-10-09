use base64::{Engine as _, engine::general_purpose};
use ed25519_dalek::{Keypair, PublicKey, SecretKey, Signature, Signer};
use std::path::PathBuf;

pub fn key_path() -> Result<PathBuf, String> {
    let home = crate::utils::fs::get_user_home()?;
    Ok(home.join(".devalang").join("keys").join("ed25519.key"))
}

pub fn load_key_bytes() -> Result<Vec<u8>, String> {
    let kp = key_path()?;
    let bytes = std::fs::read(&kp).map_err(|e| format!("Failed to read key file: {}", e))?;
    Ok(bytes)
}

pub fn ensure_keypair() -> Result<(), String> {
    let keypth = key_path()?;
    if keypth.exists() {
        return Ok(());
    }
    std::fs::create_dir_all(keypth.parent().unwrap())
        .map_err(|e| format!("Failed to create keys dir: {}", e))?;
    // generate random seed
    let mut seed = [0u8; 32];
    getrandom::getrandom(&mut seed).map_err(|e| format!("Random failed: {}", e))?;
    let sk = SecretKey::from_bytes(&seed).map_err(|e| format!("SK derive failed: {}", e))?;
    let public = PublicKey::from(&sk);
    let kp_pair = Keypair { secret: sk, public };
    std::fs::write(&keypth, kp_pair.to_bytes())
        .map_err(|e| format!("Failed to write key file: {}", e))?;
    Ok(())
}

pub fn sign_bytes(bytes: &[u8]) -> Result<(String, String), String> {
    let key_bytes = load_key_bytes()?;
    if key_bytes.len() == 64 {
        let kp = Keypair::from_bytes(&key_bytes).map_err(|e| format!("Invalid keypair: {}", e))?;
        let sig: Signature = kp.sign(bytes);
        let sig_b64 = general_purpose::STANDARD.encode(sig.to_bytes());
        let pub_b64 = general_purpose::STANDARD.encode(kp.public.to_bytes());
        return Ok((sig_b64, pub_b64));
    } else if key_bytes.len() == 32 {
        let sk = SecretKey::from_bytes(&key_bytes).map_err(|e| format!("Invalid secret: {}", e))?;
        let public = PublicKey::from(&sk);
        let kp = Keypair { secret: sk, public };
        let sig: Signature = kp.sign(bytes);
        let sig_b64 = general_purpose::STANDARD.encode(sig.to_bytes());
        let pub_b64 = general_purpose::STANDARD.encode(kp.public.to_bytes());
        return Ok((sig_b64, pub_b64));
    }
    Err("Unsupported key length".to_string())
}
