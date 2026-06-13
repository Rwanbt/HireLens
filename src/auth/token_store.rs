use serde::{Deserialize, Serialize};
use std::path::PathBuf;

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

pub fn load_token() -> Option<StoredToken> {
    let path = token_path();
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
}

pub fn save_token(token: &StoredToken) {
    let path = token_path();
    if let Ok(json) = serde_json::to_string_pretty(token) {
        let _ = std::fs::write(&path, json.as_bytes());
        // Restrict permissions on Unix; Windows relies on folder/user ACL.
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
        }
    }
}

pub fn clear_token() {
    let _ = std::fs::remove_file(token_path());
}

fn token_path() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("hirelens-auth.json")))
        .unwrap_or_else(|| PathBuf::from("hirelens-auth.json"))
}
