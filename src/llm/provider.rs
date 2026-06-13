use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::core::{Cv, JobDescription};

#[derive(Clone, Copy, Debug)]
pub enum LlmProviderKind {
    OpenAi,
    Ollama,
    LmStudio,
    Gemini,
}

impl LlmProviderKind {
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "openai" => Some(Self::OpenAi),
            "ollama" => Some(Self::Ollama),
            "lmstudio" | "lm-studio" | "lm_studio" => Some(Self::LmStudio),
            "gemini" => Some(Self::Gemini),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExtractSkillsRequest {
    pub source_name: String,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExtractSkillsResponse {
    pub skills: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdaptationRequest {
    pub cv: Cv,
    pub job: JobDescription,
    pub allowed_skills: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdaptationResponse {
    pub prioritized_skills: Vec<String>,
    pub selected_bullets: Vec<SelectedBullet>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SelectedBullet {
    pub experience_id: String,
    pub bullet: String,
}

#[async_trait]
pub trait LlmProvider: Send + Sync {
    async fn extract_skills(
        &self,
        request: ExtractSkillsRequest,
    ) -> anyhow::Result<ExtractSkillsResponse>;

    async fn generate_adaptation(
        &self,
        request: AdaptationRequest,
    ) -> anyhow::Result<AdaptationResponse>;
}
