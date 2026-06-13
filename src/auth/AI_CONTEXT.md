# AI_CONTEXT — auth

## Purpose
Google OAuth2 PKCE flow for Gemini authentication: generates PKCE challenge, opens browser, runs local redirect server, exchanges code for tokens, stores and refreshes via OS keyring. This module is Gemini-specific and has no dependency on `core::` or `llm::`.

## Thread model
| Component | Thread | Notes |
|---|---|---|
| `start_google_oauth_sync()` | Spawned thread (from gui/app.rs) | Blocking — runs HTTP server, waits for redirect |
| `get_valid_access_token()` | Spawned thread + tokio | Async — checks expiry, refreshes if needed |
| `token_store::save()` / `load()` | Any | Synchronous keyring OS calls |

## Constraints
- Tokens stored in OS keyring only (`keyring` crate) — never in plain-text files or env vars
- `oauth_server` runs on `localhost:8080` — must not conflict with other local services
- PKCE `code_verifier` is ephemeral — never stored or logged
- `client_secret` comes from `GuiSettings` (user-configured) — never hardcoded

## Forbidden
- Storing tokens in files (`.env`, `hirelens.toml`, plain JSON)
- Logging access tokens or refresh tokens
- Calling this module from the CLI path (Gemini is GUI-only — see ADR-0004)

## Common failure modes
- **Port conflict**: `oauth_server` binds 8080 — fails if another process holds it
- **Token expiry race**: `get_valid_access_token()` refreshes automatically; if refresh_token is revoked, flow must restart from scratch
- **PKCE mismatch**: if `code_challenge` and `code_verifier` are regenerated between steps — use the same instance

## Hot files
- `google.rs` — top-level flow entry points
- `oauth_server.rs` — local HTTP server for redirect_uri
- `token_store.rs` — keyring read/write

## See also
- ADR-0004 (Gemini GUI-only OAuth2 PKCE)
