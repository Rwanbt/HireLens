# AI_CONTEXT — core

## Purpose
Business logic of HireLens: ATS skill scoring, skill normalization, adaptation validation, and pipeline orchestration. This is the **anti-hallucination enforcement layer** — every LLM output passes through `validate_adaptation()` before reaching the renderer. This module must never become a pass-through.

## Thread model
| Component | Thread | Notes |
|---|---|---|
| `compute_audit()` | Any (sync, pure) | No I/O, no side effects — freely callable |
| `validate_adaptation()` | Any (sync, pure) | Called inside spawned thread before render |
| `Pipeline::audit_text()` | Spawned thread + tokio current_thread | Async — awaits LLM calls |
| `Pipeline::adapt_text()` | Spawned thread + tokio current_thread | Async — awaits LLM calls + validation |

## Constraints
- `validate_adaptation()` MUST be called on every `AdaptationResponse` before rendering
- Skill comparison is case-insensitive via `normalize_skill()` — never raw string compare
- ATS score is 0–100 integer (`u8`) — never a float reaching the UI directly
- `Pipeline` owns the router — it is the only module that calls `LlmRouter`

## Forbidden
- Any path from `AdaptationResponse` to the renderer that skips `validate_adaptation()`
- Weakening the skill whitelist (e.g., fuzzy matching, partial match)
- Weakening the bullet traceback (e.g., substring match instead of exact match)
- Calling egui / GUI code from any function in this module

## Common patterns
```rust
// The only valid flow for adaptation:
let response = router.generate_adaptation(request).await?;
validate_adaptation(&cv, &response)?;  // NEVER skip this
let rendered = render_adapted(&cv, &response);
```

## Common failure modes
- **Silent weakening**: adding `|| config.lenient_mode` to a validation check
- **Type confusion**: `AdaptationResponse.prioritized_skills` are *proposals*, not validated — only treated as validated AFTER `validate_adaptation()` returns `Ok`
- **Empty skill set**: if `cv.skills` is empty, `validate_adaptation()` rejects all proposed skills — expected behavior

## Hot files
- `validation.rs` — the anti-hallucination boundary; highest risk for regressions
- `ats.rs` — ATS scoring; pure function, well-tested
- `pipeline.rs` — orchestration; owns the LLM calls + validation call chain

## See also
- ADR-0001 (anti-hallucination via Rust validation)
- ADR-0002 (LlmProvider trait)
