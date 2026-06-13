mod app;
pub(crate) mod html_export;
mod state;
mod views;
mod widgets;

pub use app::HireLensApp;

pub(crate) const COL_GREEN: eframe::egui::Color32 = eframe::egui::Color32::from_rgb(63, 185, 80);
pub(crate) const COL_RED: eframe::egui::Color32 = eframe::egui::Color32::from_rgb(248, 81, 73);
pub(crate) const COL_YELLOW: eframe::egui::Color32 = eframe::egui::Color32::from_rgb(210, 153, 34);
pub(crate) const COL_ORANGE: eframe::egui::Color32 = eframe::egui::Color32::from_rgb(251, 146, 60);
pub(crate) const COL_BLUE: eframe::egui::Color32 = eframe::egui::Color32::from_rgb(88, 166, 255);
pub(crate) const COL_MUTED: eframe::egui::Color32 = eframe::egui::Color32::from_rgb(139, 148, 158);

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
            cc.egui_ctx.set_visuals(eframe::egui::Visuals::dark());
            Ok(Box::new(HireLensApp::default()))
        }),
    )
    .map_err(|e| anyhow::anyhow!("GUI error: {e}"))
}
