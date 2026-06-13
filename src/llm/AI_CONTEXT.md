# AI_CONTEXT — llm

## Purpose
LLM provider abstraction layer: `trait LlmProvider` defines the contract, `LlmRouter` selects the implementation at runtime. All providers communicate with models via **structured JSON only** — no free-text rendering. This module must never produce output that reaches the UI without passing through `core::validation`.

## Thread model
| Component | Thread | Notes |
|---|---|---|
| `LlmRouter::new()` | Any (sync constructor) | CLI mode — reads env/config |
| `LlmRouter::from_gui()` | Spawned thread + tokio current_thread | GUI mode — async, awaits OAuth2 token |
| `extract_skills()` / `generate_adaptation()` | Spawned thread + tokio | All providers are async |

## Constraints
- All providers return `AdaptationResponse` with `prioritized_skills: Vec<String>` and `selected_bullets: Vec<SelectedBullet>`
- Providers communicate only via structured JSON (`#[serde(deny_unknown_fields)]`)
- `LlmRouter::new(Gemini)` must always return `Err` — Gemini requires OAuth2, CLI-incompatible
- New providers must implement both `extract_skills()` AND `generate_adaptation()`

## Forbidden
- Free-text LLM output rendered directly (no Markdown, no plain text from model)
- Gemini in CLI mode (`LlmRouter::new()`)
- Storing API keys or tokens in module-level statics

## Common patterns
```rust
// Adding a new provider:
// 1. Create src/llm/myprovider.rs — impl LlmProvider
// 2. Add variant to LlmProviderKind in provider.rs
// 3. Wire in LlmRouter::new() (CLI) and from_gui() (GUI)
// 4. Add to README provider table + hirelens.example.toml
```

## Common failure modes
- **JSON parse failure**: LLM returns non-JSON or wraps JSON in Markdown fences — fix: strip fences in HTTP layer
- **Unknown fields**: `#[serde(deny_unknown_fields)]` rejects extra fields from new model versions — fix: add or remove fields from the response structs
- **Gemini token expiry**: `get_valid_access_token()` handles refresh automatically — never cache the token manually

## Hot files
- `provider.rs` — `trait LlmProvider` + all shared request/response types
- `router.rs` — the dispatch point; dual constructor (CLI/GUI)
- `gemini.rs` — OAuth2 flow integration

## See also
- ADR-0002 (LlmProvider trait + router)
- ADR-0004 (Gemini GUI-only OAuth2 PKCE)
