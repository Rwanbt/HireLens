use eframe::egui::{self, Color32, RichText, ScrollArea, TextEdit, TextStyle, Ui, Vec2};

use crate::core::diff::{compute_diff, DiffKind};
use crate::core::matching::SkillStatus;
use crate::core::AuditReport;
use crate::gui::app::{FileTarget, HireLensApp, Provider, Tab};
use crate::gui::state::{AdaptState, AuditState};
use crate::gui::theme::{
    ACCENT_PRIMARY, BG_CARD, BORDER_ACTIVE, BORDER_SUBTLE, GAP_LG, GAP_MD, GAP_SM, RADIUS_MD,
    RADIUS_SM, STATUS_ERROR, STATUS_INFO, STATUS_SUCCESS, STATUS_WARNING, TEXT_MUTED, TEXT_PRIMARY,
    TEXT_SECONDARY,
};
use crate::gui::widgets::chips::{error_line, skill_chip};
use crate::gui::widgets::gauge::{render_gauge, score_color};

pub(crate) fn render_header(app: &mut HireLensApp, ctx: &egui::Context) {
    let frame = egui::Frame::none()
        .fill(BG_CARD)
        .inner_margin(egui::Margin::symmetric(16.0, 0.0))
        .stroke(egui::Stroke::new(1.0, BORDER_SUBTLE));

    egui::TopBottomPanel::top("header")
        .exact_height(48.0)
        .frame(frame)
        .show(ctx, |ui| {
            ui.horizontal_centered(|ui| {
                render_logo(ui);

                // Right cluster: settings toggle (far right), provider status to its left.
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    render_settings_button(app, ui);
                    ui.add_space(GAP_LG);
                    render_provider_status(ui, app);
                });
            });
        });
}

/// Draws the HireLens "lens" mark as vector shapes (focus ring + center dot) so
/// it renders identically regardless of the platform's font glyph coverage.
fn render_logo(ui: &mut Ui) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(18.0), egui::Sense::hover());
    let painter = ui.painter_at(rect);
    let center = rect.center();
    painter.circle_stroke(center, 7.0, egui::Stroke::new(2.0, ACCENT_PRIMARY));
    painter.circle_filled(center, 2.5, ACCENT_PRIMARY);

    ui.add_space(GAP_SM);
    ui.label(
        RichText::new("HireLens")
            .size(19.0)
            .strong()
            .color(TEXT_PRIMARY),
    );
}

/// Frameless settings toggle (tertiary action), kept on the right of the header.
fn render_settings_button(app: &mut HireLensApp, ui: &mut Ui) {
    let label = if app.show_settings {
        "✖ Fermer"
    } else {
        "⚙️"
    };
    let button =
        egui::Button::new(RichText::new(label).size(15.0).color(TEXT_SECONDARY)).frame(false);
    if ui.add(button).clicked() {
        if app.show_settings {
            app.settings.save();
        }
        app.show_settings = !app.show_settings;
        app.settings_status = None;
    }
}

/// Live provider indicator: colored dot + label reflecting the active backend.
fn render_provider_status(ui: &mut Ui, app: &HireLensApp) {
    let (dot_color, label) = match app.provider {
        Provider::Offline => (TEXT_MUTED, "Offline (sans IA)"),
        Provider::Ollama => (STATUS_SUCCESS, "Ollama local"),
        Provider::LmStudio => (STATUS_SUCCESS, "LM Studio local"),
        Provider::OpenAi => (STATUS_INFO, "OpenAI"),
        Provider::Gemini => (STATUS_INFO, "Gemini"),
    };
    // right-to-left layout: label added first sits rightmost, dot lands to its
    // left → reads "● Label" left-to-right on screen.
    ui.label(RichText::new(label).size(12.0).color(TEXT_SECONDARY));
    ui.add_space(GAP_SM);
    ui.label(RichText::new("●").size(10.0).color(dot_color));
}

/// Below this central-panel width the CV/Job inputs collapse into a tabbed view.
const RESPONSIVE_BREAKPOINT: f32 = 800.0;

const CV_HINT: &str = "Collez votre CV ici — Markdown ou YAML frontmatter\n\
    (compétences, expériences, formation…)";

const JOB_HINT: &str = "Collez l'offre d'emploi ici\n\
    (intitulé du poste + description complète…)";

pub(crate) fn render_inputs(app: &mut HireLensApp, ui: &mut Ui, ctx: &egui::Context) {
    if ui.available_width() > RESPONSIVE_BREAKPOINT {
        let col_w = (ui.available_width() - GAP_MD) / 2.0;
        ui.horizontal_top(|ui| {
            render_cv_panel(app, ui, col_w, ctx);
            ui.add_space(GAP_MD);
            render_job_panel(app, ui, col_w, ctx);
        });
    } else {
        render_tab_nav(ui, app);
        ui.add_space(GAP_SM);
        let full_w = ui.available_width();
        match app.active_tab {
            Tab::Cv => render_cv_panel(app, ui, full_w, ctx),
            Tab::Job => render_job_panel(app, ui, full_w, ctx),
        }
    }
}

/// Tab switcher shown only in the narrow (stacked) layout.
fn render_tab_nav(ui: &mut Ui, app: &mut HireLensApp) {
    ui.horizontal(|ui| {
        if ui
            .selectable_label(
                app.active_tab == Tab::Cv,
                RichText::new("📄  CV").size(14.0),
            )
            .clicked()
        {
            app.active_tab = Tab::Cv;
        }
        ui.add_space(GAP_SM);
        if ui
            .selectable_label(
                app.active_tab == Tab::Job,
                RichText::new("💼  Offre").size(14.0),
            )
            .clicked()
        {
            app.active_tab = Tab::Job;
        }
    });
}

fn render_cv_panel(app: &mut HireLensApp, ui: &mut Ui, col_w: f32, ctx: &egui::Context) {
    ui.vertical(|ui| {
        ui.set_width(col_w);
        card_frame().show(ui, |ui| {
            if panel_header(ui, "📄  Votre CV", app.file_rx.is_none()) {
                app.start_open_file(FileTarget::Cv, ctx);
            }
            ui.add_space(GAP_SM);
            let response = ui.add(
                TextEdit::multiline(&mut app.cv_text)
                    .desired_width(ui.available_width())
                    .desired_rows(15)
                    .hint_text(CV_HINT)
                    .font(TextStyle::Monospace)
                    .frame(false),
            );
            focus_ring(ui, &response);
        });
    });
}

fn render_job_panel(app: &mut HireLensApp, ui: &mut Ui, col_w: f32, ctx: &egui::Context) {
    ui.vertical(|ui| {
        ui.set_width(col_w);
        card_frame().show(ui, |ui| {
            if panel_header(ui, "💼  Offre d'emploi", app.file_rx.is_none()) {
                app.start_open_file(FileTarget::Job, ctx);
            }
            ui.add_space(GAP_SM);
            let response = ui.add(
                TextEdit::multiline(&mut app.job_text)
                    .desired_width(ui.available_width())
                    .desired_rows(15)
                    .hint_text(JOB_HINT)
                    .frame(false),
            );
            focus_ring(ui, &response);
        });
    });
}

/// Renders a panel title plus a right-aligned "Ouvrir fichier" button.
/// Returns `true` on the frame the button is clicked.
fn panel_header(ui: &mut Ui, title: &str, file_enabled: bool) -> bool {
    let mut clicked = false;
    ui.horizontal(|ui| {
        ui.label(RichText::new(title).strong().color(TEXT_PRIMARY));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            clicked = ui
                .add_enabled(file_enabled, egui::Button::new("📂 Ouvrir fichier").small())
                .clicked();
        });
    });
    clicked
}

/// Shared card surface: filled rounded panel with a subtle border.
fn card_frame() -> egui::Frame {
    egui::Frame::none()
        .fill(BG_CARD)
        .rounding(egui::Rounding::same(RADIUS_MD))
        .stroke(egui::Stroke::new(1.0, BORDER_SUBTLE))
        .inner_margin(egui::Margin::same(GAP_MD))
}

/// Paints an accent focus ring around a just-focused input.
fn focus_ring(ui: &Ui, response: &egui::Response) {
    if response.has_focus() {
        ui.painter().rect_stroke(
            response.rect.expand(2.0),
            egui::Rounding::same(RADIUS_SM),
            egui::Stroke::new(1.5, BORDER_ACTIVE),
        );
    }
}

pub(crate) fn render_controls(app: &mut HireLensApp, ui: &mut Ui, ctx: &egui::Context) {
    use crate::gui::state::{AdaptState, AuditState};

    ui.horizontal(|ui| {
        // 6.3 — Gemini disabled when not usable (API key / token / OAuth client)
        let gemini_configured = app.gemini_available;
        egui::ComboBox::from_id_salt("provider_combo")
            .selected_text(RichText::new(app.provider.label()).color(TEXT_SECONDARY))
            .width(180.0)
            .show_ui(ui, |ui| {
                for p in [
                    Provider::Offline,
                    Provider::Ollama,
                    Provider::LmStudio,
                    Provider::OpenAi,
                ] {
                    ui.selectable_value(&mut app.provider, p, p.label());
                }
                let resp = ui.add_enabled(
                    gemini_configured,
                    egui::SelectableLabel::new(
                        app.provider == Provider::Gemini,
                        Provider::Gemini.label(),
                    ),
                );
                if resp.clicked() {
                    app.provider = Provider::Gemini;
                }
                resp.on_disabled_hover_text("Configurez Gemini dans ⚙️ Paramètres");
            });

        ui.add_space(12.0);

        if app.is_loading() {
            ui.spinner();
            ui.label(
                RichText::new("  Traitement en cours…")
                    .italics()
                    .color(TEXT_SECONDARY),
            );
        } else {
            let has_input = !app.cv_text.trim().is_empty() && !app.job_text.trim().is_empty();

            // 6.1 — always-enabled buttons; track first failed attempt
            let analyze_btn = ui.add(
                egui::Button::new(
                    RichText::new("🔍  Analyser")
                        .size(13.0)
                        .color(TEXT_SECONDARY),
                )
                .min_size(Vec2::new(120.0, 30.0))
                .fill(Color32::TRANSPARENT)
                .stroke(egui::Stroke::new(1.0, BORDER_SUBTLE)),
            );
            if analyze_btn.clicked() {
                if has_input {
                    app.start_audit(ctx);
                } else {
                    app.tried_without_input = true;
                }
            }

            ui.add_space(6.0);

            let optimize_btn = ui.add(
                egui::Button::new(
                    RichText::new("✨  Optimiser le CV")
                        .size(13.0)
                        .color(Color32::WHITE),
                )
                .min_size(Vec2::new(155.0, 30.0))
                .fill(ACCENT_PRIMARY)
                .rounding(egui::Rounding::same(RADIUS_SM)),
            );
            if optimize_btn.clicked() {
                if has_input {
                    app.start_adapt(ctx);
                } else {
                    app.tried_without_input = true;
                }
            }

            // 6.4 — Reset button
            ui.add_space(GAP_MD);
            let reset_btn = egui::Button::new(
                RichText::new("🔄 Réinitialiser")
                    .size(13.0)
                    .color(TEXT_MUTED),
            )
            .frame(false);
            if ui.add(reset_btn).clicked() {
                app.cv_text.clear();
                app.job_text.clear();
                app.audit_state = AuditState::Idle;
                app.adapt_state = AdaptState::Idle;
                app.tried_without_input = false;
                app.save_status = None;
                app.export_feedback = None;
            }

            // 6.1 — warning shown only after a failed click attempt
            if app.tried_without_input && !has_input {
                ui.add_space(8.0);
                ui.label(
                    RichText::new("⚠  Remplissez les deux champs")
                        .size(12.0)
                        .color(STATUS_WARNING),
                );
            }
        }
    });
}

pub(crate) fn render_results(app: &mut HireLensApp, ui: &mut Ui, ctx: &egui::Context) {
    let audit_report: Option<AuditReport> = if let AuditState::Done(r) = &app.audit_state {
        Some(r.clone())
    } else {
        None
    };
    let audit_error: Option<String> = if let AuditState::Error(e) = &app.audit_state {
        Some(e.clone())
    } else {
        None
    };
    let adapt_data: Option<(String, AuditReport)> =
        if let AdaptState::Done { markdown, audit } = &app.adapt_state {
            Some((markdown.clone(), audit.clone()))
        } else {
            None
        };
    let adapt_error: Option<String> = if let AdaptState::Error(e) = &app.adapt_state {
        Some(e.clone())
    } else {
        None
    };
    let audit_is_idle = matches!(app.audit_state, AuditState::Idle);
    let adapt_is_idle = matches!(app.adapt_state, AdaptState::Idle);

    // P5.1 — empty state until an analysis or optimization is requested.
    if audit_is_idle && adapt_is_idle {
        render_results_empty_state(ui);
        return;
    }

    if let Some(msg) = audit_error {
        ui.add_space(GAP_SM);
        error_line(ui, &msg);
    } else if let Some(report) = &audit_report {
        ui.add_space(GAP_SM);
        card_frame().show(ui, |ui| render_audit_panel(ui, report));
    }

    if let Some(msg) = adapt_error {
        ui.add_space(GAP_SM);
        error_line(ui, &msg);
    } else if let Some((markdown, audit)) = adapt_data {
        if audit_is_idle {
            ui.add_space(GAP_SM);
            card_frame().show(ui, |ui| render_audit_panel(ui, &audit));
        }
        ui.add_space(GAP_SM);
        card_frame().show(ui, |ui| {
            render_adapted_panel(app, ui, ctx, &markdown, &audit)
        });
    }
}

/// Placeholder shown in the results area before any analysis is run.
fn render_results_empty_state(ui: &mut Ui) {
    ui.add_space(48.0);
    ui.vertical_centered(|ui| {
        let (rect, _) = ui.allocate_exact_size(Vec2::splat(54.0), egui::Sense::hover());
        let painter = ui.painter_at(rect);
        let center = rect.center();
        painter.circle_stroke(center, 20.0, egui::Stroke::new(3.0, BORDER_SUBTLE));
        painter.circle_filled(center, 6.0, BORDER_SUBTLE);

        ui.add_space(GAP_MD);
        ui.label(
            RichText::new("Les résultats apparaîtront ici")
                .size(15.0)
                .color(TEXT_SECONDARY),
        );
        ui.add_space(4.0);
        ui.label(
            RichText::new("Collez votre CV et une offre, puis cliquez sur Analyser ou Optimiser")
                .size(12.0)
                .color(TEXT_MUTED),
        );
    });
    ui.add_space(48.0);
}

/// A labelled mini progress bar: title on the left, percentage on the right,
/// and a 6 px colored track below (color follows the gauge thresholds).
fn render_sub_score(ui: &mut Ui, label: &str, ratio: f32) {
    let ratio = ratio.clamp(0.0, 1.0);
    ui.horizontal(|ui| {
        ui.label(RichText::new(label).size(11.0).color(TEXT_SECONDARY));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(
                RichText::new(format!("{:.0}%", ratio * 100.0))
                    .size(11.0)
                    .color(TEXT_SECONDARY),
            );
        });
    });
    ui.add_space(3.0);

    let height = 6.0;
    let (rect, _) = ui.allocate_exact_size(
        Vec2::new(ui.available_width(), height),
        egui::Sense::hover(),
    );
    let painter = ui.painter_at(rect);
    let rounding = egui::Rounding::same(height / 2.0);
    painter.rect_filled(rect, rounding, BORDER_SUBTLE);
    let fill_w = rect.width() * ratio;
    if fill_w > 0.5 {
        let fill_rect = egui::Rect::from_min_size(rect.min, Vec2::new(fill_w, height));
        painter.rect_filled(fill_rect, rounding, ratio_color(ratio));
    }
}

/// Maps a 0..1 ratio to the shared gauge color scale.
fn ratio_color(ratio: f32) -> Color32 {
    score_color((ratio * 100.0).round().clamp(0.0, 100.0) as u8)
}

fn render_audit_panel(ui: &mut Ui, report: &AuditReport) {
    ui.horizontal_top(|ui| {
        ui.vertical(|ui| {
            ui.set_width(160.0);
            render_gauge(ui, report.score.score);
            ui.add_space(GAP_MD);
            // P6.3 — ATS score breakdown across the three computed dimensions.
            render_sub_score(ui, "Compétences", report.score.skill_match_ratio);
            ui.add_space(GAP_SM);
            render_sub_score(ui, "Mots-clés", report.score.keyword_score);
            ui.add_space(GAP_SM);
            render_sub_score(ui, "Structure", report.score.structure_score);
        });

        ui.add_space(20.0);

        ui.vertical(|ui| {
            ui.label(
                RichText::new("✅  Compétences matchées")
                    .strong()
                    .color(STATUS_SUCCESS),
            );
            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                if report.matched_skills.is_empty() {
                    ui.label(
                        RichText::new("aucune")
                            .italics()
                            .size(12.0)
                            .color(TEXT_SECONDARY),
                    );
                }
                for s in &report.matched_skills {
                    skill_chip(ui, s, STATUS_SUCCESS);
                }
            });

            ui.add_space(10.0);

            ui.label(
                RichText::new("❌  Compétences manquantes")
                    .strong()
                    .color(STATUS_ERROR),
            );
            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                if report.missing_skills.is_empty() {
                    ui.label(
                        RichText::new("aucune")
                            .italics()
                            .size(12.0)
                            .color(TEXT_SECONDARY),
                    );
                }
                for s in &report.missing_skills {
                    skill_chip(ui, s, STATUS_ERROR);
                }
            });

            ui.add_space(10.0);

            egui::CollapsingHeader::new(
                RichText::new("▸ Détail complet des compétences")
                    .size(12.0)
                    .color(TEXT_SECONDARY),
            )
            .default_open(false)
            .show(ui, |ui| {
                ui.label(
                    RichText::new("CV — toutes les compétences détectées")
                        .size(11.0)
                        .color(TEXT_SECONDARY),
                );
                ui.horizontal_wrapped(|ui| {
                    for s in &report.cv_skills {
                        skill_chip(ui, s, STATUS_WARNING);
                    }
                });
                ui.add_space(6.0);
                ui.label(
                    RichText::new("Offre — compétences requises")
                        .size(11.0)
                        .color(TEXT_SECONDARY),
                );
                ui.horizontal_wrapped(|ui| {
                    for s in &report.job_skills {
                        skill_chip(ui, s, STATUS_WARNING);
                    }
                });
            });

            if !report.explanations.is_empty() {
                ui.add_space(10.0);
                egui::CollapsingHeader::new(
                    RichText::new("▸ Pourquoi ce score ?")
                        .size(12.0)
                        .color(TEXT_SECONDARY),
                )
                .default_open(true)
                .show(ui, |ui| {
                    for r in &report.explanations {
                        let (label, color) = match r.status {
                            SkillStatus::Present => ("✅ présent", STATUS_SUCCESS),
                            SkillStatus::Missing => ("❌ manquant", STATUS_ERROR),
                            SkillStatus::Weak => ("⚠ faible", STATUS_WARNING),
                        };
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(&r.skill).size(12.0).color(color));
                            ui.label(
                                RichText::new(format!("— {} ({} occ.)", label, r.occurrences))
                                    .size(11.0)
                                    .color(TEXT_SECONDARY),
                            );
                        });
                    }
                });
            }
        });
    });
}

fn render_adapted_panel(
    app: &mut HireLensApp,
    ui: &mut Ui,
    ctx: &egui::Context,
    markdown: &str,
    audit: &AuditReport,
) {
    // 6.5 — title with ATS score
    ui.label(
        RichText::new(format!(
            "✨  CV Optimisé  —  Score ATS : {}/100",
            audit.score.score
        ))
        .size(16.0)
        .strong(),
    );
    ui.add_space(6.0);

    // ── Export toolbar ──
    ui.horizontal(|ui| {
        let saving = app.is_saving();

        // 6.6 — file-export group
        if ui
            .add_enabled(
                !saving,
                egui::Button::new(RichText::new("💾  Enregistrer .md").size(13.0)),
            )
            .clicked()
        {
            app.start_save_md(markdown.to_owned(), ctx);
        }

        ui.add_space(4.0);

        if ui
            .add_enabled(
                !saving,
                egui::Button::new(RichText::new("🌐  Exporter HTML").size(13.0)),
            )
            .clicked()
        {
            app.start_export_html(markdown.to_owned(), ctx);
        }

        ui.add_space(4.0);

        if ui
            .add_enabled(
                !saving,
                egui::Button::new(RichText::new("📄  PDF").size(13.0)),
            )
            .on_hover_text("Exporter en PDF via Typst")
            .clicked()
        {
            app.start_export_pdf(markdown.to_owned(), ctx);
        }

        // 6.6 — visual separator before copy
        ui.separator();

        // 6.8 — copy sets export_feedback with Instant
        if ui.button(RichText::new("📋  Copier").size(13.0)).clicked() {
            ui.output_mut(|o| o.copied_text = markdown.to_owned());
            app.export_feedback = Some((
                "✅ Copié dans le presse-papiers".to_owned(),
                std::time::Instant::now(),
            ));
        }

        // 6.8 — status from export_feedback (auto-clears after 4s)
        if saving {
            ui.add_space(8.0);
            ui.spinner();
        } else if let Some((status, _)) = &app.export_feedback {
            let color = if status.starts_with('✅') {
                STATUS_SUCCESS
            } else {
                STATUS_ERROR
            };
            ui.add_space(8.0);
            ui.label(RichText::new(status).size(12.0).color(color));
        }
    });

    ui.add_space(GAP_SM);

    // P6.5 — toggle between the raw optimized markdown and a before/after diff.
    ui.horizontal(|ui| {
        if ui
            .selectable_label(!app.show_diff, RichText::new("📝 Texte brut").size(13.0))
            .clicked()
        {
            app.show_diff = false;
        }
        ui.add_space(GAP_SM);
        if ui
            .selectable_label(app.show_diff, RichText::new("🔍 Diff").size(13.0))
            .clicked()
        {
            app.show_diff = true;
        }
    });
    ui.add_space(GAP_SM);

    if app.show_diff {
        render_diff_view(ui, &app.cv_text, markdown);
    } else {
        // outer ScrollArea in app.rs handles overflow
        let mut display = markdown.to_owned();
        ScrollArea::vertical()
            .id_salt("cv_output_scroll")
            .max_height(f32::INFINITY)
            .show(ui, |ui| {
                ui.add(
                    TextEdit::multiline(&mut display)
                        .desired_width(ui.available_width())
                        .desired_rows(18)
                        .font(TextStyle::Monospace)
                        .interactive(false),
                );
            });
    }

    ui.add_space(GAP_SM);
}

/// P6.6 — renders the before/after diff between the original CV and the optimized
/// output. Green = added lines, red = removed; unchanged lines stay muted.
fn render_diff_view(ui: &mut Ui, original: &str, adapted: &str) {
    ScrollArea::vertical()
        .id_salt("cv_diff_scroll")
        .max_height(f32::INFINITY)
        .show(ui, |ui| {
            for line in compute_diff(original, adapted) {
                let (prefix, color, bg) = match line.kind {
                    DiffKind::Unchanged => (" ", TEXT_SECONDARY, Color32::TRANSPARENT),
                    DiffKind::Added => ("+", STATUS_SUCCESS, diff_tint(STATUS_SUCCESS)),
                    DiffKind::Removed => ("-", STATUS_ERROR, diff_tint(STATUS_ERROR)),
                };
                ui.label(
                    RichText::new(format!("{prefix} {}", line.text))
                        .monospace()
                        .size(12.0)
                        .color(color)
                        .background_color(bg),
                );
            }
        });
}

/// Faint background tint for changed diff lines.
fn diff_tint(color: Color32) -> Color32 {
    Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 26)
}
