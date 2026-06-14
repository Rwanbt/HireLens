# AI_SUMMARY — core

> **Auto-generated 2026-06-14 23:17** — do not edit manually.
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
| `ats.rs` | 389 | |
| `diff.rs` | 56 | |
| `matching.rs` | 260 | |
| `mod.rs` | 39 | |
| `offline_match.rs` | 266 | |
| `pipeline.rs` | 320 | |
| `similarity.rs` | 74 | |
| `skills.rs` | 448 | |
| `text.rs` | 116 | |
| `validation.rs` | 142 | |
| **Total** | **2110** | |

## Rust API
- `AdaptedCv` (struct)
- `AtsScore` (struct)
- `AuditReport` (struct)
- `Cv` (struct)
- `DiffLine` (struct)
- `Education` (struct)
- `Experience` (struct)
- `JobDescription` (struct)
- `OfflineBullet` (struct)
- `OfflineMatchResult` (struct)
- `Pipeline` (struct)
- `PipelineOptions` (struct)
- `RequirementWeight` (struct)
- `ScoreReason` (struct)
- `SkillSignal` (struct)
- `DiffKind` (enum)
- `SkillStatus` (enum)

## Rust Functions
- `compute_audit()`
- `compute_diff()`
- `count_skill_occurrences()`
- `diff_markdown()`
- `extract_keywords()`
- `extract_local_skills()`
- `fold_accents()`
- `is_stopword()`
- `keyword_coverage()`
- `lexical_similarity()`
- `merge_skills()`
- `normalize_skill()`
- `run()`
- `skill_set()`
- `tokenize()`
- `tokenize_words()`
- `validate_adaptation()`
- `weighted_requirements()`
