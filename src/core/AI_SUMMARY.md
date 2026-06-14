# AI_SUMMARY — core

> **Auto-generated 2026-06-14 13:51** — do not edit manually.
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
| `ats.rs` | 167 | |
| `diff.rs` | 56 | |
| `matching.rs` | 118 | |
| `mod.rs` | 36 | |
| `pipeline.rs` | 303 | |
| `skills.rs` | 174 | |
| `validation.rs` | 142 | |
| **Total** | **996** | |

## Rust API
- `AdaptedCv` (struct)
- `AtsScore` (struct)
- `AuditReport` (struct)
- `Cv` (struct)
- `DiffLine` (struct)
- `Education` (struct)
- `Experience` (struct)
- `JobDescription` (struct)
- `Pipeline` (struct)
- `PipelineOptions` (struct)
- `ScoreReason` (struct)
- `SkillSignal` (struct)
- `DiffKind` (enum)
- `SkillStatus` (enum)

## Rust Functions
- `compute_audit()`
- `compute_diff()`
- `count_skill_occurrences()`
- `diff_markdown()`
- `extract_local_skills()`
- `keyword_coverage()`
- `merge_skills()`
- `normalize_skill()`
- `skill_set()`
- `validate_adaptation()`
