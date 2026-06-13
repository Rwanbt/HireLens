# Changelog

All notable changes to HireLens are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

---

## [Unreleased] — 2026-06-13

### Added
- **Word-boundary skill matching** (`matching.rs`): `\bskill\b` regex prevents false positives like "rust" matching inside "frustrated" (Phase 8.1)
- **`CONTRIBUTING.md`**: developer guide covering the anti-hallucination invariant, provider extension protocol, commit format, and glossary
- **`hirelens.example.toml`**: annotated configuration reference with env var documentation
- **`CHANGELOG.md`**: this file

### Changed
- `LlmRouter::new_local_with_fallback()` now reads Ollama/LM Studio URLs from `Config` instead of hardcoded defaults — respects `OLLAMA_BASE_URL`, `LMSTUDIO_BASE_URL`, and `hirelens.toml` (Phase 8.4)
- `Cache::key()` includes the provider name in the SHA-256 hash to prevent cross-provider cache collisions (Phase 8.5)
- `FallbackProvider::is_connection_error()` uses `reqwest::Error::is_connect()` via `downcast_ref` before falling back to string matching (Phase 8.2)
- `LlmProviderKind` gains `as_str()` method; `LlmRouter` exposes `provider_name()` for cache key construction

### Fixed
- OAuth2 PKCE `state` parameter verification confirmed present in `google.rs` (Phase 8.3)
- Code style: `cargo fmt` applied to all source files; `.gitattributes` added to enforce LF line endings

---

## [0.1.0-beta] — 2026-06-13 (Phases 6–7)

### Added
- **GUI egui polish** (Phase 6): always-enabled action buttons with deferred validation warning; ATS score in adapted CV title; visual separator in export toolbar; 4-second auto-clear export feedback via `Instant`; Reset button; Gemini option disabled when not configured; settings sections default-open only for active provider
- **Web UI refonte** (Phase 7): inline error bar replaces `alert()`; `localStorage` persistence for CV and job offer; `.md` download via `Blob + URL.createObjectURL`; empty-state placeholder; Gemini marked as GUI-only in select; responsive textareas; JS restructured into Utilitaires / Rendu / API / Événements sections

---

## [0.1.0-alpha] — 2026-06-13 (Phases 4–5)

### Security
- Anti-hallucination hardening: `allowed_skills` snapshot captured **before** `enrich_skills()` (C1 — closes critical whitelist-widening flaw)
- `validate_adaptation()` rejects empty/whitespace-only skill strings (C2)
- OAuth2 tokens migrated from plain JSON files to OS keyring (S1+S2)
- `oauth_server.rs`: 404 on non-callback paths; robust callback wait loop (S5)
- Web server binds `127.0.0.1` instead of `0.0.0.0` (M1)
- Internal errors masked from HTTP clients via `friendly_error()` (M2)

### Added
- `FallbackProvider` test suite: fallback on connection error, no fallback on auth error, exhaustion path (T1)
- `validate_adaptation()` edge-case tests: empty skill, unknown experience_id, paraphrase rejection (T2)
- `compute_audit` tests: Present/Missing/Weak classification (T3)
- `format_audit_report` test: Why section labels (T4)

---

## [0.1.0-dev] — 2026-06-13 (Phases 1–3)

### Added
- CLI `audit` / `adapt` / `build` commands
- Anti-hallucination pipeline: `validate_adaptation()` in `core/validation.rs`
- ATS scoring with skill match ratio and explanations (`core/ats.rs`, `core/matching.rs`)
- Four LLM providers: OpenAI, Ollama, LM Studio, Gemini (OAuth2 PKCE, GUI-only)
- `trait LlmProvider` with `LlmRouter` — provider-agnostic interface (ADR-0002)
- egui/eframe GUI with settings panel, file dialogs, export toolbar (ADR-0003)
- PDF export via Typst with `trait PdfRenderer` (ADR-0005)
- Web mode: `hirelens serve` — Axum HTTP server with single-page UI
- `FallbackProvider`: Ollama → LM Studio → error, never cloud fallback
- CI: `cargo fmt` + `cargo clippy -D warnings` + `cargo test` + `cargo deny` + `cargo audit`
- ADRs 0001–0005 in `docs/adr/`
- `ARCHITECTURE.md`, `KNOWN_FAILURE_PATTERNS.md`, `METRICS.md`
