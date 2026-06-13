# AI_CONTEXT — utils

## Purpose
Two utilities: `config.rs` loads `hirelens.toml` + env var overrides into a typed `Config` struct; `cache.rs` stores and retrieves LLM JSON responses keyed by SHA-256. Neither module has business logic.

## Thread model
| Component | Thread | Notes |
|---|---|---|
| `Config::load()` | Any (sync) | Reads file once at startup |
| `Cache::get_or_insert_json()` | Spawned thread + tokio | Async — awaits the `make` future |

## Constraints
- Cache key must include namespace + all file paths + body — partial key = cache collision
- `Config` fields use `#[serde(deny_unknown_fields)]` — unknown TOML keys are rejected
- Cache directory is created lazily in `get_or_insert_json()` — no init step needed

## Forbidden
- Business logic (ATS scoring, validation) in config or cache
- Storing sensitive data (tokens, API keys) in the file cache

## Hot files
- `cache.rs` — `get_or_insert_json()` is the main entry point; test watches for regression
- `config.rs` — `Config::load()` fallback chain (file → defaults) must remain stable
