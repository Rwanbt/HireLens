use eframe::egui::{self, Color32, RichText, ScrollArea, TextEdit, TextStyle, Ui, Vec2};

use crate::core::matching::SkillStatus;
use crate::core::AuditReport;
use crate::gui::app::{FileTarget, HireLensApp, Provider};
use crate::gui::state::{AdaptState, AuditState};
use crate::gui::widgets::chips::{badge, error_line, skill_chip};
use crate::gui::widgets::gauge::render_gauge;
use crate::gui::{COL_BLUE, COL_GREEN, COL_MUTED, COL_RED, COL_YELLOW};

pub(crate) fn render_header(app: &mut HireLensApp, ctx: &egui::Context) {
    egui::TopBottomPanel::top("header")
        .exact_height(42.0)
        .show(ctx, |ui| {
            ui.horizontal_centered(|ui| {
                ui.add_space(12.0);
                ui.label(RichText::new("🔍  HireLens").size(20.0).strong());
                ui.add_space(12.0);
                badge(ui, "Anti-Hallucination", COL_BLUE);
                badge(ui, "Multi-Provider", COL_BLUE);
                badge(ui, "Offline Ready", COL_GREEN);

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.add_space(12.0);
                    let label = if app.show_settings {
                        "✖ Fermer"
                    } else {
                        "⚙️"
                    };
                    if ui.button(RichText::new(label).size(13.0)).clicked() {
                        if app.show_settings {
                            app.settings.save();
                        }
                        app.show_settings = !app.show_settings;
                        app.settings_status = None;
                    }
                });
            });
        });
}

pub(crate) fn render_inputs(app: &mut HireLensApp, ui: &mut Ui, ctx: &egui::Context) {
    let avail = ui.available_width();
    let col_w = (avail - 14.0) / 2.0;

    ui.horizontal_top(|ui| {
        // ── CV ──
        ui.vertical(|ui| {
            ui.set_width(col_w);
            ui.horizontal(|ui| {
                ui.label(RichText::new("📄  Votre CV").strong());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .add_enabled(
                            app.file_rx.is_none(),
                            egui::Button::new("📂 Ouvrir fichier").small(),
                        )
                        .clicked()
                    {
                        app.start_open_file(FileTarget::Cv, ctx);
                    }
                });
            });
            ui.add_space(4.0);
            ui.add(
                TextEdit::multiline(&mut app.cv_text)
                    .desired_width(col_w)
                    .desired_rows(15)
                    .hint_text(
                        "Collez votre CV ici (Markdown ou texte)…\n\
                        \n\
                        Exemple avec frontmatter :\n\
                        ---\n\
                        name: Alice Martin\n\
                        skills:\n\
                          - Rust\n\
                          - Docker\n\
                        experience:\n\
                          - id: exp-1\n\
                            company: Acme Corp\n\
                            role: Backend Engineer\n\
                            bullets:\n\
                              - Développé des microservices…\n\
                        ---\n\
                        \n\
                        Ou format Markdown simple :\n\
                        ## Skills\n\
                        - Rust\n\
                        ## Experience\n\
                        ### Acme Corp\n\
                        - Développé des microservices…",
                    )
                    .font(TextStyle::Monospace)
                    .code_editor(),
            );
        });

        ui.add_space(14.0);

        // ── Offre ──
        ui.vertical(|ui| {
            ui.set_width(col_w);
            ui.horizontal(|ui| {
                ui.label(RichText::new("💼  Offre d'emploi").strong());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .add_enabled(
                            app.file_rx.is_none(),
                            egui::Button::new("📂 Ouvrir fichier").small(),
                        )
                        .clicked()
                    {
                        app.start_open_file(FileTarget::Job, ctx);
                    }
                });
            });
            ui.add_space(4.0);
            ui.add(
                TextEdit::multiline(&mut app.job_text)
                    .desired_width(col_w)
                    .desired_rows(15)
                    .hint_text(
                        "Collez l'offre d'emploi ici…\n\
                        \n\
                        Exemple :\n\
                        Senior Backend Engineer\n\
                        \n\
                        Nous recherchons un ingénieur\n\
                        Rust avec expérience Docker,\n\
                        Kubernetes et CI/CD…",
                    ),
            );
        });
    });
}

pub(crate) fn render_controls(app: &mut HireLensApp, ui: &mut Ui, ctx: &egui::Context) {
    use crate::gui::state::{AdaptState, AuditState};

    ui.horizontal(|ui| {
        ui.label(RichText::new("Provider :").color(COL_MUTED));

        // 6.3 — Gemini disabled when not configured
        let gemini_configured = !app.settings.gemini.client_id.is_empty();
        egui::ComboBox::from_id_salt("provider_combo")
            .selected_text(app.provider.label())
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
                    .color(COL_MUTED),
            );
        } else {
            let has_input = !app.cv_text.trim().is_empty() && !app.job_text.trim().is_empty();

            // 6.1 — always-enabled buttons; track first failed attempt
            let analyze_btn = ui.add(
                egui::Button::new(RichText::new("🔍  Analyser").size(14.0))
                    .min_size(Vec2::new(130.0, 32.0)),
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
                egui::Button::new(RichText::new("✨  Optimiser le CV").size(14.0))
                    .min_size(Vec2::new(160.0, 32.0))
                    .fill(Color32::from_rgb(30, 90, 45)),
            );
            if optimize_btn.clicked() {
                if has_input {
                    app.start_adapt(ctx);
                } else {
                    app.tried_without_input = true;
                }
            }

            // 6.4 — Reset button
            ui.add_space(12.0);
            if ui
                .button(RichText::new("🔄 Réinitialiser").size(13.0))
                .clicked()
            {
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
                        .color(COL_YELLOW),
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

    if let Some(msg) = audit_error {
        ui.separator();
        ui.add_space(6.0);
        error_line(ui, &msg);
    } else if let Some(report) = &audit_report {
        ui.separator();
        ui.add_space(6.0);
        render_audit_panel(ui, report);
    }

    if let Some(msg) = adapt_error {
        if audit_is_idle {
            ui.separator();
            ui.add_space(6.0);
        }
        error_line(ui, &msg);
    } else if let Some((markdown, audit)) = adapt_data {
        if audit_is_idle {
            ui.separator();
            ui.add_space(6.0);
            render_audit_panel(ui, &audit);
        }
        ui.separator();
        ui.add_space(6.0);
        render_adapted_panel(app, ui, ctx, &markdown, &audit);
    }
}

fn render_audit_panel(ui: &mut Ui, report: &AuditReport) {
    ui.horizontal_top(|ui| {
        ui.vertical(|ui| {
            ui.set_width(150.0);
            render_gauge(ui, report.score.score);
            ui.add_space(4.0);
            ui.centered_and_justified(|ui| {
                ui.label(
                    RichText::new(format!(
                        "{:.0}% match",
                        report.score.skill_match_ratio * 100.0
                    ))
                    .size(12.0)
                    .color(COL_MUTED),
                );
            });
        });

        ui.add_space(20.0);

        ui.vertical(|ui| {
            ui.label(
                RichText::new("✅  Compétences matchées")
                    .strong()
                    .color(COL_GREEN),
            );
            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                if report.matched_skills.is_empty() {
                    ui.label(
                        RichText::new("aucune")
                            .italics()
                            .size(12.0)
                            .color(COL_MUTED),
                    );
                }
                for s in &report.matched_skills {
                    skill_chip(ui, s, COL_GREEN);
                }
            });

            ui.add_space(10.0);

            ui.label(
                RichText::new("❌  Compétences manquantes")
                    .strong()
                    .color(COL_RED),
            );
            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                if report.missing_skills.is_empty() {
                    ui.label(
                        RichText::new("aucune")
                            .italics()
                            .size(12.0)
                            .color(COL_MUTED),
                    );
                }
                for s in &report.missing_skills {
                    skill_chip(ui, s, COL_RED);
                }
            });

            ui.add_space(10.0);

            egui::CollapsingHeader::new(
                RichText::new("▸ Détail complet des compétences")
                    .size(12.0)
                    .color(COL_MUTED),
            )
            .default_open(false)
            .show(ui, |ui| {
                ui.label(
                    RichText::new("CV — toutes les compétences détectées")
                        .size(11.0)
                        .color(COL_MUTED),
                );
                ui.horizontal_wrapped(|ui| {
                    for s in &report.cv_skills {
                        skill_chip(ui, s, COL_YELLOW);
                    }
                });
                ui.add_space(6.0);
                ui.label(
                    RichText::new("Offre — compétences requises")
                        .size(11.0)
                        .color(COL_MUTED),
                );
                ui.horizontal_wrapped(|ui| {
                    for s in &report.job_skills {
                        skill_chip(ui, s, COL_YELLOW);
                    }
                });
            });

            if !report.explanations.is_empty() {
                ui.add_space(10.0);
                egui::CollapsingHeader::new(
                    RichText::new("▸ Pourquoi ce score ?")
                        .size(12.0)
                        .color(COL_MUTED),
                )
                .default_open(true)
                .show(ui, |ui| {
                    for r in &report.explanations {
                        let (label, color) = match r.status {
                            SkillStatus::Present => ("✅ présent", COL_GREEN),
                            SkillStatus::Missing => ("❌ manquant", COL_RED),
                            SkillStatus::Weak => ("⚠ faible", COL_YELLOW),
                        };
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(&r.skill).size(12.0).color(color));
                            ui.label(
                                RichText::new(format!("— {} ({} occ.)", label, r.occurrences))
                                    .size(11.0)
                                    .color(COL_MUTED),
                            );
                        });
                    }
                });
            }
        });
    });

    ui.add_space(8.0);
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
                COL_GREEN
            } else {
                COL_RED
            };
            ui.add_space(8.0);
            ui.label(RichText::new(status).size(12.0).color(color));
        }
    });

    ui.add_space(8.0);

    // 6.5 — no max_height cap; outer ScrollArea in app.rs handles overflow
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

    ui.add_space(8.0);
}
