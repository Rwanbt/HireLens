# AI_CONTEXT — gui

## Purpose
egui/eframe user interface for HireLens. Displays inputs, triggers operations, shows results. This module must remain a **thin presentation layer** — it calls `core::` functions and renders their outputs. It must never re-implement business logic or validation.

## Thread model
| Component | Thread | Notes |
|---|---|---|
| `HireLensApp::update()` | egui main thread | ~60fps, must never block |
| `start_audit()` / `start_adapt()` | `std::thread::spawn` + tokio current_thread | Spawns thread for async LLM ops |
| `start_open_file()` / `start_save_md()` | `std::thread::spawn` | Blocks on `rfd::FileDialog` — must be in spawned thread |
| `start_google_auth()` | `std::thread::spawn` | Blocks on OAuth2 server redirect |
| `poll_results()` | egui main thread | Called at start of every `update()` — drains mpsc channels |

## Constraints
- `update()` must never block — all I/O and async go via `std::thread::spawn()` + `mpsc::channel`
- `ctx.request_repaint()` must be called in every spawned thread after `tx.send()`
- Design tokens (colors, radii, spacing) live in `gui/theme.rs` — never inline hex or magic spacing in views
- `state.rs` holds app state; `app.rs` orchestrates — keep them separate
- Widgets in `widgets/` are reusable — check before creating a new one

## Forbidden
- Blocking calls (file I/O, network, `thread::sleep`) directly in `update()`
- Business logic (ATS scoring, skill normalization, validation) in any `gui/` file
- Direct `LlmRouter` construction in views — goes through `app.rs` methods only
- `Arc<Mutex<T>>` for result passing — use `mpsc::channel` instead

## Common patterns
```rust
// Canonical spawned-thread + mpsc pattern:
pub(crate) fn start_operation(&mut self, ctx: &egui::Context) {
    let (tx, rx) = mpsc::channel();
    self.result_rx = Some(rx);
    let ctx = ctx.clone();
    std::thread::spawn(move || {
        let result = /* blocking or tokio::runtime::Builder::new_current_thread()... */;
        let _ = tx.send(result);
        ctx.request_repaint();  // wake egui
    });
}
// In poll_results(): drain with try_recv(), update state, set rx to None
```

## Common failure modes
- **Frozen UI**: blocking call added to `update()` — any file I/O, `.await`, `thread::sleep`
- **Stale result**: forgetting `ctx.request_repaint()` in spawned thread — UI never updates
- **Double send**: calling `start_*()` when `*_rx` is already `Some` — check `is_loading()` first

## Hot files
- `app.rs` — HireLensApp state + all start_* methods + poll_results
- `mod.rs` — run(), `custom_visuals()` (dark theme setup)
- `theme.rs` — design tokens (colors, radii, spacing): single source of truth
- `views/main_view.rs` — main rendering logic

## See also
- ADR-0003 (egui/eframe choice)
