use anyhow::{anyhow, Result};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::llm::http_json::{adaptation_prompt, extract_skills_prompt, parse_json_content};
use crate::llm::{
    AdaptationRequest, AdaptationResponse, ExtractSkillsRequest, ExtractSkillsResponse, LlmProvider,
};
use crate::utils::config::Config;

pub struct OllamaProvider {
    client: Client,
    base_url: String,
    model: String,
}

impl Default for OllamaProvider {
    fn default() -> Self {
        let config = Config::load().unwrap_or_default();
        Self {
            client: Client::builder()
                .timeout(config.timeout())
                .build()
                .unwrap_or_else(|_| Client::new()),
            base_url: config.ollama_base_url(),
            model: config.ollama_model(),
        }
    }
}

#[derive(Debug, Serialize)]
struct OllamaRequest {
    model: String,
    prompt: String,
    stream: bool,
    format: String,
}

#[derive(Debug, Deserialize)]
struct OllamaResponse {
    response: String,
}

impl OllamaProvider {
    async fn generate_json<T>(&self, prompt: String) -> Result<T>
    where
        T: serde::de::DeserializeOwned,
    {
        let payload = OllamaRequest {
            model: self.model.clone(),
            prompt,
            stream: false,
            format: "json".to_owned(),
        };
        let url = format!("{}/api/generate", self.base_url.trim_end_matches('/'));
        let response = self
            .client
            .post(url)
            .json(&payload)
            .send()
            .await?
            .error_for_status()?;
        let body: OllamaResponse = response.json().await?;
        if body.response.trim().is_empty() {
            return Err(anyhow!("Ollama returned an empty response"));
        }
        parse_json_content(&body.response)
    }
}

#[async_trait]
impl LlmProvider for OllamaProvider {
    async fn extract_skills(&self, request: ExtractSkillsRequest) -> Result<ExtractSkillsResponse> {
        self.generate_json(extract_skills_prompt(&request)?).await
    }

    async fn generate_adaptation(&self, request: AdaptationRequest) -> Result<AdaptationResponse> {
        self.generate_json(adaptation_prompt(&request)?).await
    }
}
