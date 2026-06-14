mod app;
mod controller;
pub(crate) mod html_export;
pub(crate) mod settings;
mod state;
pub(crate) mod theme;
mod views;
mod widgets;

pub use app::HireLensApp;

use eframe::egui::{Color32, Rounding, Visuals};

pub fn run() -> anyhow::Result<()> {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1100.0, 800.0])
            .with_min_inner_size([800.0, 600.0])
            .with_title("HireLens — CV Optimizer")
            .with_app_id("hirelens"),
        ..Default::default()
    };

    eframe::run_native(
        "HireLens",
        options,
        Box::new(|cc| {
            cc.egui_ctx.set_visuals(custom_visuals());
            Ok(Box::new(HireLensApp::default()))
        }),
    )
    .map_err(|e| anyhow::anyhow!("GUI error: {e}"))
}

/// Builds the HireLens dark theme on top of egui's dark preset.
///
/// Overrides only the surfaces that carry the product identity (app/card
/// backgrounds, hover, accent selection). All token values live in [`theme`].
fn custom_visuals() -> Visuals {
    let mut visuals = Visuals::dark();
    visuals.window_fill = theme::BG_APP;
    visuals.panel_fill = theme::BG_APP;
    visuals.faint_bg_color = theme::BG_CARD;
    visuals.extreme_bg_color = theme::BG_APP;
    visuals.widgets.inactive.bg_fill = theme::BG_CARD;
    visuals.widgets.hovered.bg_fill = theme::BG_HOVER;
    visuals.widgets.active.bg_fill = theme::ACCENT_PRIMARY;
    visuals.selection.bg_fill = Color32::from_rgba_unmultiplied(
        theme::ACCENT_PRIMARY.r(),
        theme::ACCENT_PRIMARY.g(),
        theme::ACCENT_PRIMARY.b(),
        60,
    );
    visuals.window_rounding = Rounding::same(8.0);
    visuals
}
