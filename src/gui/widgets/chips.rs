use eframe::egui::{Color32, Frame, Margin, RichText, Rounding, Ui};

use crate::gui::theme::STATUS_ERROR;

pub(crate) fn skill_chip(ui: &mut Ui, skill: &str, color: Color32) {
    let fill = Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 28);
    Frame::none()
        .fill(fill)
        .rounding(Rounding::same(10.0))
        .inner_margin(Margin {
            left: 7.0,
            right: 7.0,
            top: 2.0,
            bottom: 2.0,
        })
        .show(ui, |ui| {
            ui.label(RichText::new(skill).color(color).size(11.5));
        });
}

pub(crate) fn error_line(ui: &mut Ui, msg: &str) {
    let fill =
        Color32::from_rgba_unmultiplied(STATUS_ERROR.r(), STATUS_ERROR.g(), STATUS_ERROR.b(), 20);
    Frame::none()
        .fill(fill)
        .rounding(Rounding::same(6.0))
        .inner_margin(Margin::symmetric(12.0, 8.0))
        .show(ui, |ui| {
            ui.label(
                RichText::new(format!("❌  {msg}"))
                    .color(STATUS_ERROR)
                    .size(13.0),
            );
        });
    ui.add_space(6.0);
}
