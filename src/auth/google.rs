use anyhow::{Context, Result};
use serde::Deserialize;

use super::{
    oauth_server::CallbackServer,
    pkce,
    token_store::{load_token, save_token, StoredToken},
};

const SCOPE: &str = "https://www.googleapis.com/auth/generativelanguage";
const AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const TOKEN_URL: &str = "https://oauth2.googleapis.com/token";

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: u64,
    #[allow(dead_code)]
    token_type: Option<String>,
}

// ──────────────────────────────────────────────────────────────
// Public API
// ──────────────────────────────────────────────────────────────

/// Full PKCE OAuth2 flow. Blocking — call from `std::thread::spawn`.
/// Opens the system browser and waits for the local callback.
pub fn start_google_oauth_sync(client_id: &str, client_secret: &str) -> Result<()> {
    if client_id.is_empty() {
        anyhow::bail!(
            "Google OAuth2 non configuré — client_id manquant.\n\
             Ajoutez-le dans ⚙️ Paramètres → Gemini."
        );
    }

    let server = CallbackServer::bind().context("Impossible de démarrer le serveur callback")?;
    let redirect_uri = format!("http://127.0.0.1:{}/callback", server.port);

    let pkce_challenge = pkce::generate();
    let state = random_hex(16);

    let auth_url = build_auth_url(client_id, &redirect_uri, &pkce_challenge.code_challenge, &state);
    open::that(&auth_url).context("Impossible d'ouvrir le navigateur")?;

    let (code, returned_state) = server.wait_for_callback()?;

    if returned_state != state {
        anyhow::bail!("CSRF mismatch — paramètre state invalide");
    }

    // Token exchange uses async reqwest; create a mini runtime for it.
    let token = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(exchange_code(
            client_id,
            client_secret,
            &code,
            &redirect_uri,
            &pkce_challenge.code_verifier,
        ))?;

    save_token(&token);
    Ok(())
}

/// Returns a valid access token, refreshing if expired. Async — call from inside block_on.
pub async fn get_valid_access_token(client_id: &str, client_secret: &str) -> Result<String> {
    let stored = load_token().context(
        "Non connecté à Google Gemini — cliquez \"Connexion Google\" dans ⚙️ Paramètres.",
    )?;

    if !stored.is_expired() {
        return Ok(stored.access_token);
    }

    let refresh_token = stored
        .refresh_token
        .context("Token Gemini expiré — veuillez vous reconnecter dans ⚙️ Paramètres.")?;

    if client_id.is_empty() {
        anyhow::bail!("Google OAuth2 non configuré — client_id manquant dans ⚙️ Paramètres.");
    }

    let new_token = refresh_access_token(client_id, client_secret, &refresh_token).await?;
    save_token(&new_token);
    Ok(new_token.access_token)
}

// ──────────────────────────────────────────────────────────────
// Internal helpers
// ──────────────────────────────────────────────────────────────

async fn exchange_code(
    client_id: &str,
    client_secret: &str,
    code: &str,
    redirect_uri: &str,
    code_verifier: &str,
) -> Result<StoredToken> {
    let client = reqwest::Client::new();
    let resp = client
        .post(TOKEN_URL)
        .form(&[
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", redirect_uri),
            ("client_id", client_id),
            ("client_secret", client_secret),
            ("code_verifier", code_verifier),
        ])
        .send()
        .await
        .context("Échange de code OAuth2 échoué")?
        .error_for_status()
        .context("Réponse d'erreur du serveur OAuth2")?;

    let body = resp.text().await?;
    let token_resp: TokenResponse =
        serde_json::from_str(&body).context("Réponse token OAuth2 invalide")?;

    Ok(into_stored_token(
        token_resp.access_token,
        token_resp.refresh_token,
        token_resp.expires_in,
    ))
}

async fn refresh_access_token(
    client_id: &str,
    client_secret: &str,
    refresh_token: &str,
) -> Result<StoredToken> {
    let client = reqwest::Client::new();
    let resp = client
        .post(TOKEN_URL)
        .form(&[
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
            ("client_id", client_id),
            ("client_secret", client_secret),
        ])
        .send()
        .await
        .context("Rafraîchissement du token Gemini échoué")?
        .error_for_status()
        .context("Réponse d'erreur lors du refresh token")?;

    let body = resp.text().await?;
    let token_resp: TokenResponse = serde_json::from_str(&body)?;

    Ok(into_stored_token(
        token_resp.access_token,
        token_resp.refresh_token.or(Some(refresh_token.to_owned())),
        token_resp.expires_in,
    ))
}

fn into_stored_token(
    access_token: String,
    refresh_token: Option<String>,
    expires_in: u64,
) -> StoredToken {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    StoredToken { access_token, refresh_token, expires_at: now + expires_in }
}

fn build_auth_url(
    client_id: &str,
    redirect_uri: &str,
    code_challenge: &str,
    state: &str,
) -> String {
    format!(
        "{}?response_type=code&client_id={}&redirect_uri={}&scope={}&state={}&code_challenge={}&code_challenge_method=S256&access_type=offline&prompt=consent",
        AUTH_URL,
        percent_encode(client_id),
        percent_encode(redirect_uri),
        percent_encode(SCOPE),
        percent_encode(state),
        percent_encode(code_challenge),
    )
}

fn random_hex(bytes: usize) -> String {
    use rand::Rng;
    let mut buf = vec![0u8; bytes];
    rand::thread_rng().fill(buf.as_mut_slice());
    buf.iter().map(|b| format!("{b:02x}")).collect()
}

fn percent_encode(s: &str) -> String {
    s.bytes()
        .flat_map(|b| {
            if b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b'.' || b == b'~' {
                vec![b as char]
            } else {
                format!("%{b:02X}").chars().collect::<Vec<_>>()
            }
        })
        .collect()
}
