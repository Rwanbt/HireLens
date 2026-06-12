use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;

use crate::llm::http_json::{adaptation_prompt, extract_skills_prompt, post_openai_compatible};
use crate::llm::{
    AdaptationRequest, AdaptationResponse, ExtractSkillsRequest, ExtractSkillsResponse, LlmProvider,
};
use crate::utils::config::Config;

pub struct LmStudioProvider {
    client: Client,
    base_url: String,
    model: String,
}

impl Default for LmStudioProvider {
    fn default() -> Self {
        let config = Config::load().unwrap_or_default();
        Self {
            client: Client::builder()
                .timeout(config.timeout())
                .build()
                .unwrap_or_else(|_| Client::new()),
            base_url: config.lmstudio_base_url(),
            model: config.lmstudio_model(),
        }
    }
}

#[async_trait]
impl LlmProvider for LmStudioProvider {
    async fn extract_skills(&self, request: ExtractSkillsRequest) -> Result<ExtractSkillsResponse> {
        let prompt = extract_skills_prompt(&request)?;
        post_openai_compatible(
            &self.client,
            &format!("{}/chat/completions", self.base_url.trim_end_matches('/')),
            None,
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
            None,
            &self.model,
            prompt,
        )
        .await
    }
}
