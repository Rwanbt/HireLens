use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;

use crate::llm::http_json::{adaptation_prompt, extract_skills_prompt, post_openai_compatible};
use crate::llm::{
    AdaptationRequest, AdaptationResponse, ExtractSkillsRequest, ExtractSkillsResponse, LlmProvider,
};

/// Google Gemini via the OpenAI-compatible endpoint.
/// Auth: OAuth2 Bearer token (access_token from the PKCE flow).
const BASE_URL: &str = "https://generativelanguage.googleapis.com/v1beta/openai";

pub struct GeminiProvider {
    client: Client,
    access_token: String,
    model: String,
}

impl GeminiProvider {
    pub fn with_token(access_token: String, model: String) -> Self {
        Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(60))
                .build()
                .unwrap_or_else(|_| Client::new()),
            access_token,
            model,
        }
    }
}

#[async_trait]
impl LlmProvider for GeminiProvider {
    async fn extract_skills(&self, request: ExtractSkillsRequest) -> Result<ExtractSkillsResponse> {
        let prompt = extract_skills_prompt(&request)?;
        post_openai_compatible(
            &self.client,
            &format!("{BASE_URL}/chat/completions"),
            Some(&self.access_token),
            &self.model,
            prompt,
        )
        .await
    }

    async fn generate_adaptation(&self, request: AdaptationRequest) -> Result<AdaptationResponse> {
        let prompt = adaptation_prompt(&request)?;
        post_openai_compatible(
            &self.client,
            &format!("{BASE_URL}/chat/completions"),
            Some(&self.access_token),
            &self.model,
            prompt,
        )
        .await
    }
}
