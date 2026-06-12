use std::path::PathBuf;
use std::time::Duration;

use serde::Deserialize;

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub provider: Option<String>,
    pub offline: Option<bool>,
    pub cache: Option<bool>,
    pub cache_dir: Option<PathBuf>,
    pub timeout_seconds: Option<u64>,
    pub openai_api_key: Option<String>,
    pub openai_model: Option<String>,
    pub openai: OpenAiConfig,
    pub ollama: LocalProviderConfig,
    pub lmstudio: LocalProviderConfig,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct OpenAiConfig {
    pub api_key: Option<String>,
    pub model: Option<String>,
    pub base_url: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct LocalProviderConfig {
    pub model: Option<String>,
    pub base_url: Option<String>,
}

impl Config {
    pub fn load() -> anyhow::Result<Self> {
        let path = config_path();
        if !path.exists() {
            return Ok(Self::default());
        }
        let text = std::fs::read_to_string(path)?;
        Ok(toml::from_str(&text)?)
    }

    #[cfg(test)]
    pub fn parse(text: &str) -> anyhow::Result<Self> {
        Ok(toml::from_str(text)?)
    }

    pub fn cache_enabled(&self) -> bool {
        self.cache.unwrap_or(true)
    }

    pub fn cache_dir(&self) -> PathBuf {
        self.cache_dir
            .clone()
            .unwrap_or_else(|| PathBuf::from(".cache"))
    }

    pub fn timeout(&self) -> Duration {
        Duration::from_secs(self.timeout_seconds.unwrap_or(60))
    }

    pub fn openai_api_key(&self) -> Option<String> {
        std::env::var("OPENAI_API_KEY")
            .ok()
            .or_else(|| self.openai.api_key.clone())
            .or_else(|| self.openai_api_key.clone())
    }

    pub fn openai_model(&self) -> String {
        std::env::var("OPENAI_MODEL")
            .ok()
            .or_else(|| self.openai.model.clone())
            .or_else(|| self.openai_model.clone())
            .unwrap_or_else(|| "gpt-4o-mini".to_owned())
    }

    pub fn openai_base_url(&self) -> String {
        std::env::var("OPENAI_BASE_URL")
            .ok()
            .or_else(|| self.openai.base_url.clone())
            .unwrap_or_else(|| "https://api.openai.com/v1".to_owned())
    }

    pub fn ollama_base_url(&self) -> String {
        std::env::var("OLLAMA_BASE_URL")
            .ok()
            .or_else(|| self.ollama.base_url.clone())
            .unwrap_or_else(|| "http://localhost:11434".to_owned())
    }

    pub fn ollama_model(&self) -> String {
        std::env::var("OLLAMA_MODEL")
            .ok()
            .or_else(|| self.ollama.model.clone())
            .unwrap_or_else(|| "llama3.1".to_owned())
    }

    pub fn lmstudio_base_url(&self) -> String {
        std::env::var("LMSTUDIO_BASE_URL")
            .ok()
            .or_else(|| self.lmstudio.base_url.clone())
            .unwrap_or_else(|| "http://localhost:1234/v1".to_owned())
    }

    pub fn lmstudio_model(&self) -> String {
        std::env::var("LMSTUDIO_MODEL")
            .ok()
            .or_else(|| self.lmstudio.model.clone())
            .unwrap_or_else(|| "local-model".to_owned())
    }
}

fn config_path() -> PathBuf {
    if let Ok(path) = std::env::var("HIRELENS_CONFIG") {
        return PathBuf::from(path);
    }

    let local = PathBuf::from("hirelens.toml");
    if local.exists() {
        return local;
    }

    dirs::config_dir()
        .map(|dir| dir.join("hirelens").join("config.toml"))
        .unwrap_or(local)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_nested_provider_config() {
        let config = Config::parse(
            r#"
provider = "lmstudio"
offline = true
cache = false
cache_dir = ".hirelens-cache"
timeout_seconds = 12

[openai]
model = "gpt-test"
base_url = "https://example.test/v1"

[ollama]
model = "llama-test"
base_url = "http://localhost:11434"

[lmstudio]
model = "studio-test"
base_url = "http://localhost:1234/v1"
"#,
        )
        .expect("config should parse");

        assert_eq!(config.provider.as_deref(), Some("lmstudio"));
        assert_eq!(config.offline, Some(true));
        assert!(!config.cache_enabled());
        assert_eq!(config.cache_dir(), PathBuf::from(".hirelens-cache"));
        assert_eq!(config.timeout(), Duration::from_secs(12));
        assert_eq!(config.openai_model(), "gpt-test");
        assert_eq!(config.ollama_model(), "llama-test");
        assert_eq!(config.lmstudio_model(), "studio-test");
    }

    #[test]
    fn rejects_unknown_config_fields() {
        let error = Config::parse("surprise = true").expect_err("unknown field should fail");

        assert!(error.to_string().contains("unknown field"));
    }
}
