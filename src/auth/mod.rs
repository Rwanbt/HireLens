mod google;
mod oauth_server;
mod pkce;
mod token_store;

pub use google::{embedded_client, get_valid_access_token, start_google_oauth_sync};
pub use token_store::{clear_token, load_token};

/// Keyring service name — shared between auth and llm layers.
pub const KEYRING_SERVICE: &str = "hirelens";
pub const KEYRING_OPENAI_ACCOUNT: &str = "openai_api_key";
pub const KEYRING_GEMINI_ACCOUNT: &str = "gemini_oauth_tokens";
pub const KEYRING_GEMINI_API_KEY: &str = "gemini_api_key";
