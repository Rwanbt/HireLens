use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;

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

    /// Local-only mode with automatic fallback: Ollama → LM Studio → offline (skip LLM).
    /// Never falls back to cloud providers.
    pub fn new_local_with_fallback() -> Self {
        let providers: Vec<Arc<dyn LlmProvider>> = vec![
            Arc::new(OllamaProvider::default()),
            Arc::new(LmStudioProvider::default()),
        ];
        Self { provider: Arc::new(FallbackProvider { providers }) }
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

// ──────────────────────────────────────────────────────────────
// FallbackProvider — tries providers in order on connection errors
// ──────────────────────────────────────────────────────────────

struct FallbackProvider {
    providers: Vec<Arc<dyn LlmProvider>>,
}

impl FallbackProvider {
    fn is_connection_error(e: &anyhow::Error) -> bool {
        let msg = e.to_string();
        msg.contains("Connection refused")
            || msg.contains("tcp connect")
            || msg.contains("10061")
            || msg.contains("error sending request")
    }
}

#[async_trait]
impl LlmProvider for FallbackProvider {
    async fn extract_skills(&self, request: ExtractSkillsRequest) -> Result<ExtractSkillsResponse> {
        let mut last_err = anyhow::anyhow!("no local providers available");
        for provider in &self.providers {
            match provider.extract_skills(request.clone()).await {
                Ok(r) => return Ok(r),
                Err(e) if Self::is_connection_error(&e) => last_err = e,
                Err(e) => return Err(e),
            }
        }
        Err(last_err)
    }

    async fn generate_adaptation(
        &self,
        request: AdaptationRequest,
    ) -> Result<AdaptationResponse> {
        let mut last_err = anyhow::anyhow!("no local providers available");
        for provider in &self.providers {
            match provider.generate_adaptation(request.clone()).await {
                Ok(r) => return Ok(r),
                Err(e) if Self::is_connection_error(&e) => last_err = e,
                Err(e) => return Err(e),
            }
        }
        Err(last_err)
    }
}
