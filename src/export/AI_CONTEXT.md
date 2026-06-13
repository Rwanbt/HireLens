# AI_CONTEXT — export

## Purpose
Renders a validated `Cv` (or adapted CV) to Markdown, PDF (via Pandoc), or HTML. This module is the **last stage of the pipeline** — it only receives data that has already passed `validate_adaptation()`. It must never re-introduce LLM output.

## Thread model
| Component | Thread | Notes |
|---|---|---|
| `render_markdown()` | Any (sync, pure) | Template rendering — no I/O |
| PDF via Pandoc | Spawned thread (tokio process) | Async subprocess |

## Constraints
- Input is always a post-validation `Cv` — never raw `AdaptationResponse`
- Pandoc PDF export is optional — fail gracefully if Pandoc is not installed
- HTML export (`gui/html_export.rs`) is self-contained — no external deps

## Forbidden
- Accepting raw `AdaptationResponse` as input
- Calling LLM or network from this module
- Failing hard if Pandoc is absent — degrade gracefully with a clear error message

## Hot files
- `mod.rs` — render entry points

## See also
- ADR-0001 (export is downstream of validation — only receives clean data)
