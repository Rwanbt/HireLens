# Contributing to HireLens

## Quick start

```powershell
git clone https://github.com/Rwanbt/HireLens
cd HireLens
cargo build
cargo test                        # 39 tests — must all pass before any PR
cargo clippy --all-targets -- -D warnings
```

## The one rule you must not break

`src/core/validation.rs` — `validate_adaptation()` — is the **anti-hallucination boundary**.

Every skill or bullet the LLM proposes must exist verbatim in the original CV before it can reach the output. This is the product's core invariant. **Do not weaken it.** Any change that relaxes this check requires an ADR (`docs/adr/`) and explicit human approval.

See [ADR-0001](docs/adr/0001-anti-hallucination-validation.md) for the full rationale.

## Pre-commit checklist

Run these before every commit:

```powershell
cargo fmt --check          # format
cargo clippy --all-targets -- -D warnings   # lint — zero warnings
cargo test                 # all tests green
```

CI enforces all three, plus `cargo deny` and `cargo audit`. A PR that breaks any gate will not be merged.

## Adding a new LLM provider

1. Create `src/llm/<name>.rs` — implement `trait LlmProvider` (two async methods: `extract_skills` and `generate_adaptation`).
2. Add a variant to `LlmProviderKind` in `src/llm/provider.rs` and a branch in `as_str()`.
3. Wire it in `LlmRouter::new()` (CLI) and `LlmRouter::from_gui()` (GUI) in `src/llm/router.rs`.
4. Add a row to the provider table in `README.md` and a commented example in `hirelens.example.toml`.

If the provider requires interactive auth (like Gemini OAuth2), mark it as GUI-only in the CLI router (`anyhow::bail!`). See [ADR-0002](docs/adr/0002-llm-provider-trait.md) and [ADR-0004](docs/adr/0004-gemini-oauth2-gui-only.md).

## Code conventions

| Rule | Detail |
|------|--------|
| **No `unwrap()`** | Use `?`, `map_err()`, or `anyhow::bail!`. Every surviving `unwrap()` needs `// SAFETY: <reason>`. |
| **No magic numbers** | Named constants only. |
| **Functions ≤ 80 LOC** | Extract sub-functions before growing further. |
| **English everywhere** | Code, comments, commit messages. |
| **Comments = WHY** | Never describe what the code does — the names already do that. |
| **No dead code** | Delete it; don't comment it out. `git log -S "name"` recovers anything. |

## Commit format

```
<type>(<scope>): <description>
```

Types: `feat`, `fix`, `refactor`, `perf`, `docs`, `test`, `chore`

Examples:
```
feat(llm): add Anthropic Claude provider
fix(validation): reject empty skill strings
chore(deps): bump reqwest to 0.12.5
```

## PR size

Keep PRs to **≤ 400 lines changed**. Larger changes must be split into sequential autonomous PRs, each independently buildable and mergeable.

## Architecture

Before a significant change, read [ARCHITECTURE.md](ARCHITECTURE.md). Key constraints:

- **Pipeline direction**: UI → Core → Types. Never reverse.
- **`validate_adaptation()` must run** before any adapted content reaches the renderer.
- **CV skill snapshot before enrichment**: `allowed_skills` must be captured before `enrich_skills()` is called. See `pipeline.rs:adapt()`.
- **No LLM fallback to cloud**: `FallbackProvider` tries Ollama then LM Studio — never OpenAI or Gemini.

## Glossary

| Term | Definition in HireLens |
|------|----------------------|
| **ATS** | Applicant Tracking System — the automated filter that scores CVs against a job description before a human sees them. |
| **Skill signal** | A (skill, occurrence_count) pair extracted from a job posting's raw text. |
| **Allowed skills** | The set of skills present in the *original* CV, captured before any LLM enrichment. Only these may appear in the adapted output. |
| **Validation** | The post-LLM check in `validate_adaptation()` that rejects any skill or bullet not found verbatim in the original CV. |
| **Offline mode** | Runs the full pipeline using only skills declared in the CV's frontmatter — zero LLM calls. |
| **Adaptation** | The process of reordering/selecting existing CV bullets to match a specific job posting. No new content is invented. |
