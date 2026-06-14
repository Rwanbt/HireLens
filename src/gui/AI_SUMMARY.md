# AI_SUMMARY — gui

> **Auto-generated 2026-06-14 15:07** — do not edit manually.
> Source: `tools/ai_docs/generate_ai_summary.py`
> For purpose, thread model and constraints, read `AI_CONTEXT.md`.

## Purpose
egui/eframe user interface for HireLens. Displays inputs, triggers operations, shows results. This module must remain a **thin presentation layer** — it calls `core::` functions and renders their outputs. It must never re-implement business logic or validation.

## Common failure modes
- **Frozen UI**: blocking call added to `update()` — any file I/O, `.await`, `thread::sleep`
- **Stale result**: forgetting `ctx.request_repaint()` in spawned thread — UI never updates
- **Double send**: calling `start_*()` when `*_rx` is already `Some` — check `is_loading()` first

## Hot files
- `app.rs` — HireLensApp state + all start_* methods + poll_results
- `mod.rs` — run(), `custom_visuals()` (dark theme setup)
- `theme.rs` — design tokens (colors, radii, spacing): single source of truth
- `views/main_view.rs` — main rendering logic

## Files & LOC
| File | LOC | |
|------|-----|--|
| `app.rs` | 239 | |
| `controller.rs` | 245 | |
| `html_export.rs` | 92 | |
| `mod.rs` | 47 | |
| `settings.rs` | 119 | |
| `state.rs` | 16 | |
| `theme.rs` | 22 | |
| **Total** | **780** | |

## Rust API
- `HireLensApp` (struct)

## Rust Functions
- `run()`
- `to_html()`
