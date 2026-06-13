# AI_SUMMARY — utils

> **Auto-generated 2026-06-13 15:14** — do not edit manually.
> Source: `tools/ai_docs/generate_ai_summary.py`
> For purpose, thread model and constraints, read `AI_CONTEXT.md`.

## Purpose
Two utilities: `config.rs` loads `hirelens.toml` + env var overrides into a typed `Config` struct; `cache.rs` stores and retrieves LLM JSON responses keyed by SHA-256. Neither module has business logic.

## Hot files
- `cache.rs` — `get_or_insert_json()` is the main entry point; test watches for regression
- `config.rs` — `Config::load()` fallback chain (file → defaults) must remain stable

## Files & LOC
| File | LOC | |
|------|-----|--|
| `cache.rs` | 100 | |
| `config.rs` | 139 | |
| `mod.rs` | 2 | |
| **Total** | **241** | |

## Rust API
- `Cache` (struct)
- `Config` (struct)
- `LocalProviderConfig` (struct)
- `OpenAiConfig` (struct)
