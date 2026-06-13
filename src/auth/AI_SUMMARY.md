# AI_SUMMARY — auth

> **Auto-generated 2026-06-13 13:58** — do not edit manually.
> Source: `tools/ai_docs/generate_ai_summary.py`
> For purpose, thread model and constraints, read `AI_CONTEXT.md`.

## Purpose
Google OAuth2 PKCE flow for Gemini authentication: generates PKCE challenge, opens browser, runs local redirect server, exchanges code for tokens, stores and refreshes via OS keyring. This module is Gemini-specific and has no dependency on `core::` or `llm::`.

## Common failure modes
- **Port conflict**: `oauth_server` binds 8080 — fails if another process holds it
- **Token expiry race**: `get_valid_access_token()` refreshes automatically; if refresh_token is revoked, flow must restart from scratch
- **PKCE mismatch**: if `code_challenge` and `code_verifier` are regenerated between steps — use the same instance

## Hot files
- `google.rs` — top-level flow entry points
- `oauth_server.rs` — local HTTP server for redirect_uri
- `token_store.rs` — keyring read/write

## Files & LOC
| File | LOC | |
|------|-----|--|
| `google.rs` | 166 | |
| `mod.rs` | 9 | |
| `oauth_server.rs` | 90 | |
| `pkce.rs` | 15 | |
| `token_store.rs` | 40 | |
| **Total** | **320** | |

## Rust API
- `CallbackServer` (struct)
- `PkceChallenge` (struct)
- `StoredToken` (struct)

## Rust Functions
- `clear_token()`
- `generate()`
- `get_valid_access_token()`
- `load_token()`
- `save_token()`
- `start_google_oauth_sync()`
