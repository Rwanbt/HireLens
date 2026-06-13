use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use rand::Rng;
use sha2::{Digest, Sha256};

pub struct PkceChallenge {
    pub code_verifier: String,
    pub code_challenge: String,
}

/// Generates a PKCE code_verifier (43 URL-safe chars) and its S256 challenge.
pub fn generate() -> PkceChallenge {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill(&mut bytes);
    let code_verifier = URL_SAFE_NO_PAD.encode(bytes);

    let hash = Sha256::digest(code_verifier.as_bytes());
    let code_challenge = URL_SAFE_NO_PAD.encode(hash.as_slice());

    PkceChallenge {
        code_verifier,
        code_challenge,
    }
}
