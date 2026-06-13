use anyhow::{Context, Result};
use async_trait::async_trait;
use reqwest::Client;

use crate::llm::http_json::{adaptation_prompt, extract_skills_prompt, post_openai_compatible};
use crate::llm::{
    AdaptationRequest, AdaptationResponse, ExtractSkillsRequest, ExtractSkillsResponse, LlmProvider,
};
use crate::utils::config::Config;

pub struct OpenAiProvider {
    client: Client,
    api_key: String,
    model: String,
    base_url: String,
}

impl OpenAiProvider {
    pub fn from_env() -> Result<Self> {
        let config = Config::load().unwrap_or_default();
        let api_key = config
            .openai_api_key()
            .context("OPENAI_API_KEY is required for --provider openai")?;
        let model = config.openai_model();
        let base_url = config.openai_base_url();

        Ok(Self {
            client: Client::builder().timeout(config.timeout()).build()?,
            api_key,
            model,
            base_url,
        })
    }

    /// GUI mode: checks keyring first, then falls back to env / config file.
    pub fn from_keyring_or_env() -> Result<Self> {
        let config = Config::load().unwrap_or_default();
        let api_key = keyring::Entry::new(
            crate::auth::KEYRING_SERVICE,
            crate::auth::KEYRING_OPENAI_ACCOUNT,
        )
        .ok()
        .and_then(|e| e.get_password().ok())
        .or_else(|| config.openai_api_key())
        .context(
            "Clé API OpenAI non configurée. Ajoutez-la dans ⚙️ Paramètres → OpenAI.",
        )?;

        Ok(Self {
            client: Client::builder().timeout(config.timeout()).build()?,
            api_key,
            model: config.openai_model(),
            base_url: config.openai_base_url(),
        })
    }
}

#[async_trait]
impl LlmProvider for OpenAiProvider {
    async fn extract_skills(&self, request: ExtractSkillsRequest) -> Result<ExtractSkillsResponse> {
        let prompt = extract_skills_prompt(&request)?;
        post_openai_compatible(
            &self.client,
            &format!("{}/chat/completions", self.base_url.trim_end_matches('/')),
            Some(&self.api_key),
            &self.model,
            prompt,
        )
        .await
    }

    async fn generate_adaptation(&self, request: AdaptationRequest) -> Result<AdaptationResponse> {
        let prompt = adaptation_prompt(&request)?;
        post_openai_compatible(
            &self.client,
            &format!("{}/chat/completions", self.base_url.trim_end_matches('/')),
            Some(&self.api_key),
            &self.model,
            prompt,
        )
        .await
    }
}
