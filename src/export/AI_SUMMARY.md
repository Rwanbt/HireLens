# AI_SUMMARY — export

> **Auto-generated 2026-06-13 13:53** — do not edit manually.
> Source: `tools/ai_docs/generate_ai_summary.py`
> For purpose, thread model and constraints, read `AI_CONTEXT.md`.

## Purpose
Renders a validated `Cv` (or adapted CV) to Markdown, PDF (via Pandoc), or HTML. This module is the **last stage of the pipeline** — it only receives data that has already passed `validate_adaptation()`. It must never re-introduce LLM output.

## Hot files
- `mod.rs` — render entry points

## Files & LOC
| File | LOC | |
|------|-----|--|
| `mod.rs` | 155 | |
| `typst_render.rs` | 209 | |
| **Total** | **364** | |

## Rust API
- `MarkdownExporter` (struct)
- `PdfExporter` (struct)
- `TypstRenderer` (struct)
- `PdfRenderer` (trait)

## Rust Functions
- `export_pdf()`
- `markdown_to_typst()`
- `render_cv()`
