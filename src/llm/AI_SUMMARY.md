# AI_SUMMARY — llm

> **Auto-generated 2026-06-14 15:03** — do not edit manually.
> Source: `tools/ai_docs/generate_ai_summary.py`
> For purpose, thread model and constraints, read `AI_CONTEXT.md`.

## Purpose
LLM provider abstraction layer: `trait LlmProvider` defines the contract, `LlmRouter` selects the implementation at runtime. All providers communicate with models via **structured JSON only** — no free-text rendering. This module must never produce output that reaches the UI without passing through `core::validation`.

## Common failure modes
- **JSON parse failure**: LLM returns non-JSON or wraps JSON in Markdown fences — fix: strip fences in HTTP layer
- **Unknown fields**: `#[serde(deny_unknown_fields)]` rejects extra fields from new model versions — fix: add or remove fields from the response structs
- **Gemini token expiry**: `get_valid_access_token()` handles refresh automatically — never cache the token manually

## Hot files
- `provider.rs` — `trait LlmProvider` + all shared request/response types
- `router.rs` — the dispatch point; dual constructor (CLI/GUI)
- `gemini.rs` — OAuth2 flow integration

## Files & LOC
| File | LOC | |
|------|-----|--|
| `gemini.rs` | 49 | |
| `http_json.rs` | 146 | |
| `lmstudio.rs` | 62 | |
| `mod.rs` | 14 | |
| `ollama.rs` | 80 | |
| `openai.rs` | 71 | |
| `provider.rs` | 58 | |
| `router.rs` | 242 | |
| **Total** | **722** | |

## Rust API
- `AdaptationRequest` (struct)
- `AdaptationResponse` (struct)
- `ExtractSkillsRequest` (struct)
- `ExtractSkillsResponse` (struct)
- `GeminiProvider` (struct)
- `GuiRouterOptions` (struct)
- `LlmRouter` (struct)
- `LmStudioProvider` (struct)
- `OllamaProvider` (struct)
- `OpenAiProvider` (struct)
- `SelectedBullet` (struct)
- `LlmProviderKind` (enum)
- `LlmProvider` (trait)

## Rust Functions
- `adaptation_prompt()`
- `extract_skills_prompt()`
- `offline_adaptation()`
- `offline_extract_skills()`
- `parse_json_content()`
- `post_openai_compatible()`
