# AI_CONTEXT — parser

## Purpose
Parses Markdown + YAML frontmatter into `Cv` and `JobDescription` structs. This module is the **source of truth for the original CV content** — the output feeds directly into `core::validation` as the whitelist. A parsing bug that silently drops skills or bullets would weaken the anti-hallucination guarantee downstream.

## Thread model
| Component | Thread | Notes |
|---|---|---|
| `parse_cv()` / `parse_job()` | Any (sync, pure) | No I/O — operates on `&str` |

## Constraints
- Parser output must faithfully preserve all `skills` and `bullets` from the input
- `Cv::skills` must be the complete, unfiltered list — normalization happens in `core::skills`
- `Experience::bullets` must be exact verbatim strings — `validate_adaptation()` uses exact match
- YAML parsing errors must surface as `Err` — never silently produce an empty `Cv`

## Forbidden
- Silently dropping any skill or bullet during parsing
- Normalizing or lowercasing during parsing (that's `core::skills::normalize_skill`'s job)
- LLM calls from this module

## Common failure modes
- **Missing YAML block**: gray_matter returns empty frontmatter silently — add explicit error if `skills` key is absent
- **Bullet whitespace**: trailing spaces in YAML bullets don't match exact bullet in validation — trim consistently

## Hot files
- `mod.rs` — the entire parser (single file)

## See also
- ADR-0001 (anti-hallucination — parser feeds the whitelist)
