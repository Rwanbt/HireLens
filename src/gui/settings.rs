use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// ──────────────────────────────────────────────────────────────
// Structs
// ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct GuiSettings {
    pub(crate) ollama_url: String,
    pub(crate) ollama_model: String,
    pub(crate) lmstudio_url: String,
    pub(crate) lmstudio_model: String,
    pub(crate) gemini: GeminiSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct GeminiSettings {
    /// Google Cloud OAuth2 client_id for "Desktop app" type.
    pub(crate) client_id: String,
    /// Not truly secret for desktop apps — embedded per Google's installed-app flow.
    pub(crate) client_secret: String,
    pub(crate) model: String,
}

// ──────────────────────────────────────────────────────────────
// Defaults
// ──────────────────────────────────────────────────────────────

impl Default for GuiSettings {
    fn default() -> Self {
        Self {
            ollama_url: "http://localhost:11434".to_owned(),
            ollama_model: "llama3.1".to_owned(),
            lmstudio_url: "http://localhost:1234/v1".to_owned(),
            lmstudio_model: "local-model".to_owned(),
            gemini: GeminiSettings::default(),
        }
    }
}

impl Default for GeminiSettings {
    fn default() -> Self {
        Self {
            client_id: String::new(),
            client_secret: String::new(),
            model: "gemini-1.5-flash".to_owned(),
        }
    }
}

// ──────────────────────────────────────────────────────────────
// Persistence
// ──────────────────────────────────────────────────────────────

impl GuiSettings {
    pub(crate) fn load() -> Self {
        let path = settings_path();
        if !path.exists() {
            return Self::default();
        }
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| toml::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub(crate) fn save(&self) {
        let path = settings_path();
        if let Ok(text) = toml::to_string_pretty(self) {
            let _ = std::fs::write(path, text);
        }
    }
}

// ──────────────────────────────────────────────────────────────
// OpenAI keyring helpers
// ──────────────────────────────────────────────────────────────

impl GuiSettings {
    pub(crate) fn get_openai_key() -> Option<String> {
        keyring::Entry::new(
            crate::auth::KEYRING_SERVICE,
            crate::auth::KEYRING_OPENAI_ACCOUNT,
        )
        .ok()
        .and_then(|e| e.get_password().ok())
        .filter(|s| !s.is_empty())
    }

    pub(crate) fn set_openai_key(key: &str) -> Result<()> {
        let entry = keyring::Entry::new(
            crate::auth::KEYRING_SERVICE,
            crate::auth::KEYRING_OPENAI_ACCOUNT,
        )?;
        if key.is_empty() {
            let _ = entry.delete_credential();
        } else {
            entry.set_password(key)?;
        }
        Ok(())
    }

    pub(crate) fn delete_openai_key() {
        if let Ok(entry) = keyring::Entry::new(
            crate::auth::KEYRING_SERVICE,
            crate::auth::KEYRING_OPENAI_ACCOUNT,
        ) {
            let _ = entry.delete_credential();
        }
    }

    pub(crate) fn get_gemini_api_key() -> Option<String> {
        keyring::Entry::new(
            crate::auth::KEYRING_SERVICE,
            crate::auth::KEYRING_GEMINI_API_KEY,
        )
        .ok()
        .and_then(|e| e.get_password().ok())
        .filter(|s| !s.is_empty())
    }

    pub(crate) fn set_gemini_api_key(key: &str) -> Result<()> {
        let entry = keyring::Entry::new(
            crate::auth::KEYRING_SERVICE,
            crate::auth::KEYRING_GEMINI_API_KEY,
        )?;
        if key.is_empty() {
            let _ = entry.delete_credential();
        } else {
            entry.set_password(key)?;
        }
        Ok(())
    }

    pub(crate) fn delete_gemini_api_key() {
        if let Ok(entry) = keyring::Entry::new(
            crate::auth::KEYRING_SERVICE,
            crate::auth::KEYRING_GEMINI_API_KEY,
        ) {
            let _ = entry.delete_credential();
        }
    }
}

// ──────────────────────────────────────────────────────────────
// Path helper
// ──────────────────────────────────────────────────────────────

fn settings_path() -> PathBuf {
    // Portable: config lives next to the executable so a USB deployment "just works".
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("hirelens-gui.toml")))
        .unwrap_or_else(|| PathBuf::from("hirelens-gui.toml"))
}
