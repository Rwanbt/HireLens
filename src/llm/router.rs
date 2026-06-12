use std::sync::Arc;

use anyhow::Result;

use super::provider::{
    AdaptationRequest, AdaptationResponse, ExtractSkillsRequest, ExtractSkillsResponse,
    LlmProvider, LlmProviderKind,
};
use crate::llm::{lmstudio::LmStudioProvider, ollama::OllamaProvider, openai::OpenAiProvider};

#[derive(Clone)]
pub struct LlmRouter {
    provider: Arc<dyn LlmProvider>,
}

impl LlmRouter {
    pub fn new(kind: LlmProviderKind) -> Result<Self> {
        let provider: Arc<dyn LlmProvider> = match kind {
            LlmProviderKind::OpenAi => Arc::new(OpenAiProvider::from_env()?),
            LlmProviderKind::Ollama => Arc::new(OllamaProvider::default()),
            LlmProviderKind::LmStudio => Arc::new(LmStudioProvider::default()),
        };

        Ok(Self { provider })
    }

    pub async fn extract_skills(
        &self,
        request: ExtractSkillsRequest,
    ) -> Result<ExtractSkillsResponse> {
        self.provider.extract_skills(request).await
    }

    pub async fn generate_adaptation(
        &self,
        request: AdaptationRequest,
    ) -> Result<AdaptationResponse> {
        self.provider.generate_adaptation(request).await
    }
}
