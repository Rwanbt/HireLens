mod http_json;
mod lmstudio;
mod ollama;
mod openai;
mod provider;
mod router;

pub(crate) use http_json::{offline_adaptation, offline_extract_skills};
#[cfg(test)]
pub use provider::SelectedBullet;
pub use provider::{
    AdaptationRequest, AdaptationResponse, ExtractSkillsRequest, ExtractSkillsResponse,
    LlmProvider, LlmProviderKind,
};
pub use router::LlmRouter;
