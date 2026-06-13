# AI_SUMMARY — parser

> **Auto-generated 2026-06-13 04:38** — do not edit manually.
> Source: `tools/ai_docs/generate_ai_summary.py`
> For purpose, thread model and constraints, read `AI_CONTEXT.md`.

## Purpose
Parses Markdown + YAML frontmatter into `Cv` and `JobDescription` structs. This module is the **source of truth for the original CV content** — the output feeds directly into `core::validation` as the whitelist. A parsing bug that silently drops skills or bullets would weaken the anti-hallucination guarantee downstream.

## Common failure modes
- **Missing YAML block**: gray_matter returns empty frontmatter silently — add explicit error if `skills` key is absent
- **Bullet whitespace**: trailing spaces in YAML bullets don't match exact bullet in validation — trim consistently

## Hot files
- `mod.rs` — the entire parser (single file)

## Files & LOC
| File | LOC | |
|------|-----|--|
| `mod.rs` | 165 | |
| **Total** | **165** | |

## Rust API

## Rust Functions
- `parse_cv_file()`
- `parse_cv_markdown()`
- `parse_job_file()`
- `parse_job_text()`
