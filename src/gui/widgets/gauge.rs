use eframe::egui::{self, Align2, Color32, FontId, Pos2, Sense, Stroke, Ui, Vec2};

use crate::gui::{COL_GREEN, COL_MUTED, COL_ORANGE, COL_RED, COL_YELLOW};

pub(crate) fn render_gauge(ui: &mut Ui, score: u8) {
    let size = Vec2::splat(130.0);
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    let painter = ui.painter_at(rect);
    let center = rect.center();
    let radius = 52.0_f32;
    let stroke_w = 9.0_f32;

    painter.circle_stroke(
        center,
        radius,
        Stroke::new(stroke_w, Color32::from_gray(48)),
    );

    let color = score_color(score);
    if score > 0 {
        let filled = score as f32 / 100.0;
        let start = -std::f32::consts::FRAC_PI_2;
        let end = start + filled * std::f32::consts::TAU;
        let n = 90usize;
        let pts: Vec<Pos2> = (0..=n)
            .map(|i| {
                let t = i as f32 / n as f32;
                let a = start + t * (end - start);
                center + radius * egui::vec2(a.cos(), a.sin())
            })
            .collect();
        painter.add(egui::Shape::line(pts, Stroke::new(stroke_w, color)));
    }

    painter.text(
        center - egui::vec2(0.0, 7.0),
        Align2::CENTER_CENTER,
        score.to_string(),
        FontId::proportional(32.0),
        color,
    );
    painter.text(
        center + egui::vec2(0.0, 15.0),
        Align2::CENTER_CENTER,
        "/ 100",
        FontId::proportional(11.0),
        COL_MUTED,
    );
}

pub(crate) fn score_color(score: u8) -> Color32 {
    if score >= 80 {
        COL_GREEN
    } else if score >= 60 {
        COL_YELLOW
    } else if score >= 40 {
        COL_ORANGE
    } else {
        COL_RED
    }
}
