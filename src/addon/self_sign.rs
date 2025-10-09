use crate::utils::signing;

type SignResult = (Option<String>, Option<String>, Option<String>);

/// Sign both raw and gzipped shas (byte slices) and return (sig_raw_b64, sig_gz_b64, pub_b64)
pub fn sign_two_shas(sha_raw: &[u8], sha_gz: &[u8]) -> Result<SignResult, String> {
    // Attempt to sign raw sha
    let mut sig_raw_b64: Option<String> = None;
    let mut sig_gz_b64: Option<String> = None;
    let mut pub_b64: Option<String> = None;

    match signing::load_key_bytes() {
        Ok(_) => {
            if let Ok((s_raw, p_raw)) = signing::sign_bytes(sha_raw) {
                sig_raw_b64 = Some(s_raw);
                pub_b64 = Some(p_raw);
            }
            if let Ok((s_gz, p_gz)) = signing::sign_bytes(sha_gz) {
                sig_gz_b64 = Some(s_gz);
                // prefer pub from raw; otherwise set from gz
                if pub_b64.is_none() {
                    pub_b64 = Some(p_gz);
                }
            }
        }
        Err(_) => {
            // no key present, return None for all
            return Ok((None, None, None));
        }
    }

    Ok((sig_raw_b64, sig_gz_b64, pub_b64))
}
