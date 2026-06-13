use std::sync::Arc;

use anyhow::Result;

use super::provider::{
    AdaptationRequest, AdaptationResponse, ExtractSkillsRequest, ExtractSkillsResponse,
    LlmProvider, LlmProviderKind,
};
use crate::llm::{
    gemini::GeminiProvider, lmstudio::LmStudioProvider, ollama::OllamaProvider,
    openai::OpenAiProvider,
};

/// Options for GUI-mode router creation. Contains all user-configurable settings
/// so `from_gui` needs no access to the gui module.
pub struct GuiRouterOptions {
    pub ollama_url: String,
    pub ollama_model: String,
    pub lmstudio_url: String,
    pub lmstudio_model: String,
    pub gemini_model: String,
    pub gemini_client_id: String,
    pub gemini_client_secret: String,
}

#[derive(Clone)]
pub struct LlmRouter {
    provider: Arc<dyn LlmProvider>,
}

impl LlmRouter {
    /// CLI mode: reads API key from env / config file.
    pub fn new(kind: LlmProviderKind) -> Result<Self> {
        let provider: Arc<dyn LlmProvider> = match kind {
            LlmProviderKind::OpenAi => Arc::new(OpenAiProvider::from_env()?),
            LlmProviderKind::Ollama => Arc::new(OllamaProvider::default()),
            LlmProviderKind::LmStudio => Arc::new(LmStudioProvider::default()),
            LlmProviderKind::Gemini => {
                anyhow::bail!("Use LlmRouter::from_gui for Gemini in GUI mode")
            }
        };
        Ok(Self { provider })
    }

    /// GUI mode: uses settings panel values + keyring / stored OAuth2 token.
    pub async fn from_gui(kind: LlmProviderKind, opts: &GuiRouterOptions) -> Result<Self> {
        let provider: Arc<dyn LlmProvider> = match kind {
            LlmProviderKind::Gemini => {
                let token = crate::auth::get_valid_access_token(
                    &opts.gemini_client_id,
                    &opts.gemini_client_secret,
                )
                .await?;
                Arc::new(GeminiProvider::with_token(token, opts.gemini_model.clone()))
            }
            LlmProviderKind::OpenAi => {
                Arc::new(OpenAiProvider::from_keyring_or_env()?)
            }
            LlmProviderKind::Ollama => Arc::new(OllamaProvider::with_settings(
                opts.ollama_url.clone(),
                opts.ollama_model.clone(),
            )),
            LlmProviderKind::LmStudio => Arc::new(LmStudioProvider::with_settings(
                opts.lmstudio_url.clone(),
                opts.lmstudio_model.clone(),
            )),
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
