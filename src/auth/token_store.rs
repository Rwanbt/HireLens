use serde::{Deserialize, Serialize};

use super::{KEYRING_GEMINI_ACCOUNT, KEYRING_SERVICE};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredToken {
    pub access_token: String,
    pub refresh_token: Option<String>,
    /// Unix timestamp (seconds) at which the token expires.
    pub expires_at: u64,
}

impl StoredToken {
    pub fn is_expired(&self) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        // 60-second buffer to avoid using a token that expires during the request.
        now >= self.expires_at.saturating_sub(60)
    }

    pub fn seconds_until_expiry(&self) -> i64 {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        (self.expires_at as i64) - (now as i64)
    }
}

// S1+S2 — OAuth2 tokens live in the OS keyring (Credential Manager / Keychain /
// Secret Service), never as a plaintext JSON file on disk. This also removes the
// TOCTOU file-permission concern (S3) entirely. Same backend as the OpenAI key.
fn entry() -> Option<keyring::Entry> {
    keyring::Entry::new(KEYRING_SERVICE, KEYRING_GEMINI_ACCOUNT).ok()
}

pub fn load_token() -> Option<StoredToken> {
    let json = entry()?.get_password().ok()?;
    serde_json::from_str(&json).ok()
}

pub fn save_token(token: &StoredToken) {
    if let (Some(entry), Ok(json)) = (entry(), serde_json::to_string(token)) {
        let _ = entry.set_password(&json);
    }
}

pub fn clear_token() {
    if let Some(entry) = entry() {
        let _ = entry.delete_credential();
    }
}
