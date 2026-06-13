# AI_SUMMARY — core

> **Auto-generated 2026-06-13 15:15** — do not edit manually.
> Source: `tools/ai_docs/generate_ai_summary.py`
> For purpose, thread model and constraints, read `AI_CONTEXT.md`.

## Purpose
Business logic of HireLens: ATS skill scoring, skill normalization, adaptation validation, and pipeline orchestration. This is the **anti-hallucination enforcement layer** — every LLM output passes through `validate_adaptation()` before reaching the renderer. This module must never become a pass-through.

## Common failure modes
- **Silent weakening**: adding `|| config.lenient_mode` to a validation check
- **Type confusion**: `AdaptationResponse.prioritized_skills` are *proposals*, not validated — only treated as validated AFTER `validate_adaptation()` returns `Ok`
- **Empty skill set**: if `cv.skills` is empty, `validate_adaptation()` rejects all proposed skills — expected behavior

## Hot files
- `validation.rs` — the anti-hallucination boundary; highest risk for regressions
- `ats.rs` — ATS scoring; pure function, well-tested
- `pipeline.rs` — orchestration; owns the LLM calls + validation call chain

## Files & LOC
| File | LOC | |
|------|-----|--|
| `ats.rs` | 120 | |
| `matching.rs` | 74 | |
| `mod.rs` | 35 | |
| `pipeline.rs` | 293 | |
| `skills.rs` | 177 | |
| `validation.rs` | 142 | |
| **Total** | **841** | |

## Rust API
- `AdaptedCv` (struct)
- `AtsScore` (struct)
- `AuditReport` (struct)
- `Cv` (struct)
- `Education` (struct)
- `Experience` (struct)
- `JobDescription` (struct)
- `Pipeline` (struct)
- `PipelineOptions` (struct)
- `ScoreReason` (struct)
- `SkillSignal` (struct)
- `SkillStatus` (enum)

## Rust Functions
- `compute_audit()`
- `count_skill_occurrences()`
- `diff_markdown()`
- `extract_local_skills()`
- `merge_skills()`
- `normalize_skill()`
- `skill_set()`
- `validate_adaptation()`
