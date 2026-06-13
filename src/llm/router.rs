use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;

use crate::utils::config::Config;

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
    /// Used as a cache-key discriminator so Ollama and OpenAI results are never mixed.
    name: String,
}

impl LlmRouter {
    pub fn provider_name(&self) -> &str {
        &self.name
    }
}

impl LlmRouter {
    /// CLI mode: reads API key from env / config file.
    pub fn new(kind: LlmProviderKind) -> Result<Self> {
        let name = kind.as_str().to_owned();
        let provider: Arc<dyn LlmProvider> = match kind {
            LlmProviderKind::OpenAi => Arc::new(OpenAiProvider::from_env()?),
            LlmProviderKind::Ollama => Arc::new(OllamaProvider::default()),
            LlmProviderKind::LmStudio => Arc::new(LmStudioProvider::default()),
            LlmProviderKind::Gemini => {
                anyhow::bail!("Use LlmRouter::from_gui for Gemini in GUI mode")
            }
        };
        Ok(Self { provider, name })
    }

    /// GUI mode: uses settings panel values + keyring / stored OAuth2 token.
    pub async fn from_gui(kind: LlmProviderKind, opts: &GuiRouterOptions) -> Result<Self> {
        let name = kind.as_str().to_owned();
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
        Ok(Self { provider, name })
    }

    /// Local-only mode with automatic fallback: Ollama → LM Studio → offline (skip LLM).
    /// Never falls back to cloud providers. Reads URLs/models from Config (env vars or file).
    pub fn new_local_with_fallback() -> Self {
        let config = Config::load().unwrap_or_default();
        let providers: Vec<Arc<dyn LlmProvider>> = vec![
            Arc::new(OllamaProvider::with_settings(
                config.ollama_base_url(),
                config.ollama_model(),
            )),
            Arc::new(LmStudioProvider::with_settings(
                config.lmstudio_base_url(),
                config.lmstudio_model(),
            )),
        ];
        Self { provider: Arc::new(FallbackProvider { providers }), name: "local-fallback".to_owned() }
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
        // Prefer the typed reqwest API when available; fall back to string matching
        // for non-reqwest errors (e.g. plain anyhow!("Connection refused") in tests).
        if let Some(reqwest_err) = e.downcast_ref::<reqwest::Error>() {
            return reqwest_err.is_connect();
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    enum Behavior {
        Ok,
        ConnError,
        AuthError,
    }

    struct MockProvider {
        behavior: Behavior,
        calls: Arc<AtomicUsize>,
    }

    impl MockProvider {
        fn spawn(behavior: Behavior) -> (Arc<dyn LlmProvider>, Arc<AtomicUsize>) {
            let calls = Arc::new(AtomicUsize::new(0));
            let provider = Arc::new(MockProvider { behavior, calls: calls.clone() });
            (provider, calls)
        }

        fn outcome<T>(&self, ok: T) -> Result<T> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            match self.behavior {
                Behavior::Ok => Ok(ok),
                // a connection error must trigger fallback to the next provider
                Behavior::ConnError => Err(anyhow::anyhow!("Connection refused (os error 10061)")),
                // an auth error must propagate immediately, never be masked
                Behavior::AuthError => Err(anyhow::anyhow!("401 Unauthorized")),
            }
        }
    }

    #[async_trait]
    impl LlmProvider for MockProvider {
        async fn extract_skills(
            &self,
            _request: ExtractSkillsRequest,
        ) -> Result<ExtractSkillsResponse> {
            self.outcome(ExtractSkillsResponse { skills: vec!["rust".into()] })
        }

        async fn generate_adaptation(
            &self,
            _request: AdaptationRequest,
        ) -> Result<AdaptationResponse> {
            self.outcome(AdaptationResponse {
                prioritized_skills: Vec::new(),
                selected_bullets: Vec::new(),
            })
        }
    }

    fn request() -> ExtractSkillsRequest {
        ExtractSkillsRequest { source_name: "CV".into(), text: "rust".into() }
    }

    #[tokio::test]
    async fn fallback_triggers_on_connection_error() {
        let (first, first_calls) = MockProvider::spawn(Behavior::ConnError);
        let (second, second_calls) = MockProvider::spawn(Behavior::Ok);
        let fallback = FallbackProvider { providers: vec![first, second] };

        let result = fallback.extract_skills(request()).await;

        assert!(result.is_ok());
        assert_eq!(first_calls.load(Ordering::SeqCst), 1, "first provider tried");
        assert_eq!(second_calls.load(Ordering::SeqCst), 1, "fell back to second");
    }

    #[tokio::test]
    async fn fallback_skipped_on_auth_error() {
        let (first, first_calls) = MockProvider::spawn(Behavior::AuthError);
        let (second, second_calls) = MockProvider::spawn(Behavior::Ok);
        let fallback = FallbackProvider { providers: vec![first, second] };

        let result = fallback.extract_skills(request()).await;

        assert!(result.is_err(), "401 must not be hidden by fallback");
        assert_eq!(first_calls.load(Ordering::SeqCst), 1);
        assert_eq!(
            second_calls.load(Ordering::SeqCst),
            0,
            "second provider must NOT be tried on auth error"
        );
    }

    #[tokio::test]
    async fn fallback_exhausted_returns_connection_error() {
        let (first, _) = MockProvider::spawn(Behavior::ConnError);
        let (second, _) = MockProvider::spawn(Behavior::ConnError);
        let fallback = FallbackProvider { providers: vec![first, second] };

        let error = fallback
            .extract_skills(request())
            .await
            .expect_err("both providers down → error");

        assert!(FallbackProvider::is_connection_error(&error));
    }
}
