use std::sync::mpsc;

use eframe::egui::{
    self, Align2, Color32, FontId, Frame, Margin, Pos2, RichText, Rounding, ScrollArea,
    Sense, Stroke, TextEdit, TextStyle, Ui, Vec2,
};

use crate::core::{AuditReport, Pipeline, PipelineOptions};
use crate::llm::{LlmProviderKind, LlmRouter};

// ═══════════════════════════ Colors ═══════════════════════════

const COL_GREEN: Color32 = Color32::from_rgb(63, 185, 80);
const COL_RED: Color32 = Color32::from_rgb(248, 81, 73);
const COL_YELLOW: Color32 = Color32::from_rgb(210, 153, 34);
const COL_ORANGE: Color32 = Color32::from_rgb(251, 146, 60);
const COL_BLUE: Color32 = Color32::from_rgb(88, 166, 255);
const COL_MUTED: Color32 = Color32::from_rgb(139, 148, 158);

// ═══════════════════════════ Provider ═══════════════════════════

#[derive(Default, PartialEq, Clone, Copy)]
enum Provider {
    #[default]
    Offline,
    Ollama,
    LmStudio,
    OpenAi,
}

impl Provider {
    fn label(self) -> &'static str {
        match self {
            Self::Offline => "🔌 Offline (sans IA)",
            Self::Ollama => "🦙 Ollama (local)",
            Self::LmStudio => "🏠 LM Studio (local)",
            Self::OpenAi => "✨ OpenAI",
        }
    }

    fn to_kind(self) -> (LlmProviderKind, bool) {
        match self {
            Self::Offline => (LlmProviderKind::Ollama, true),
            Self::Ollama => (LlmProviderKind::Ollama, false),
            Self::LmStudio => (LlmProviderKind::LmStudio, false),
            Self::OpenAi => (LlmProviderKind::OpenAi, false),
        }
    }
}

// ═══════════════════════════ Task states ═══════════════════════════

enum AuditState {
    Idle,
    Loading,
    Done(AuditReport),
    Error(String),
}

enum AdaptState {
    Idle,
    Loading,
    Done { markdown: String, audit: AuditReport },
    Error(String),
}

// ═══════════════════════════ App ═══════════════════════════

pub struct HireLensApp {
    cv_text: String,
    job_text: String,
    provider: Provider,

    audit_state: AuditState,
    audit_rx: Option<mpsc::Receiver<Result<AuditReport, String>>>,

    adapt_state: AdaptState,
    adapt_rx: Option<mpsc::Receiver<Result<(String, AuditReport), String>>>,

    save_status: Option<String>,
}

impl Default for HireLensApp {
    fn default() -> Self {
        Self {
            cv_text: String::new(),
            job_text: String::new(),
            provider: Provider::Offline,
            audit_state: AuditState::Idle,
            audit_rx: None,
            adapt_state: AdaptState::Idle,
            adapt_rx: None,
            save_status: None,
        }
    }
}

impl HireLensApp {
    fn is_loading(&self) -> bool {
        self.audit_rx.is_some() || self.adapt_rx.is_some()
    }

    fn poll_results(&mut self) {
        if let Some(rx) = &self.audit_rx {
            if let Ok(result) = rx.try_recv() {
                self.audit_state = match result {
                    Ok(report) => AuditState::Done(report),
                    Err(msg) => AuditState::Error(msg),
                };
                self.audit_rx = None;
            }
        }
        if let Some(rx) = &self.adapt_rx {
            if let Ok(result) = rx.try_recv() {
                self.adapt_state = match result {
                    Ok((markdown, audit)) => AdaptState::Done { markdown, audit },
                    Err(msg) => AdaptState::Error(msg),
                };
                self.adapt_rx = None;
            }
        }
    }

    fn start_audit(&mut self, ctx: &egui::Context) {
        let cv = self.cv_text.clone();
        let job = self.job_text.clone();
        let (kind, offline) = self.provider.to_kind();
        let ctx = ctx.clone();

        let (tx, rx) = mpsc::channel();
        self.audit_rx = Some(rx);
        self.audit_state = AuditState::Loading;
        self.save_status = None;

        std::thread::spawn(move || {
            let result = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|e| e.to_string())
                .and_then(|rt| {
                    rt.block_on(async {
                        if cv.trim().is_empty() {
                            return Err("Le CV est vide.".to_owned());
                        }
                        if job.trim().is_empty() {
                            return Err("L'offre d'emploi est vide.".to_owned());
                        }
                        let router = LlmRouter::new(kind).map_err(|e| e.to_string())?;
                        let pipeline = Pipeline::new(router);
                        pipeline
                            .audit_text(
                                &cv,
                                &job,
                                PipelineOptions { offline, use_cache: true },
                            )
                            .await
                            .map_err(|e| friendly_error(e.to_string()))
                    })
                });
            let _ = tx.send(result);
            ctx.request_repaint();
        });
    }

    fn start_adapt(&mut self, ctx: &egui::Context) {
        let cv = self.cv_text.clone();
        let job = self.job_text.clone();
        let (kind, offline) = self.provider.to_kind();
        let ctx = ctx.clone();

        let (tx, rx) = mpsc::channel();
        self.adapt_rx = Some(rx);
        self.adapt_state = AdaptState::Loading;
        self.save_status = None;

        std::thread::spawn(move || {
            let result = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|e| e.to_string())
                .and_then(|rt| {
                    rt.block_on(async {
                        if cv.trim().is_empty() {
                            return Err("Le CV est vide.".to_owned());
                        }
                        if job.trim().is_empty() {
                            return Err("L'offre d'emploi est vide.".to_owned());
                        }
                        let router = LlmRouter::new(kind).map_err(|e| e.to_string())?;
                        let pipeline = Pipeline::new(router);
                        pipeline
                            .adapt_text(
                                &cv,
                                &job,
                                PipelineOptions { offline, use_cache: true },
                            )
                            .await
                            .map(|adapted| (adapted.rendered_markdown, adapted.audit))
                            .map_err(|e| friendly_error(e.to_string()))
                    })
                });
            let _ = tx.send(result);
            ctx.request_repaint();
        });
    }
}

// ═══════════════════════════ eframe::App ═══════════════════════════

impl eframe::App for HireLensApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_results();
        if self.is_loading() {
            ctx.request_repaint_after(std::time::Duration::from_millis(80));
        }

        render_header(ctx);

        egui::CentralPanel::default().show(ctx, |ui| {
            ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
                ui.add_space(12.0);
                render_inputs(self, ui);
                ui.add_space(12.0);
                render_controls(self, ui, ctx);
                ui.add_space(8.0);
                render_results(self, ui);
            });
        });
    }
}

// ═══════════════════════════ Render sections ═══════════════════════════

fn render_header(ctx: &egui::Context) {
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
            });
        });
}

fn render_inputs(app: &mut HireLensApp, ui: &mut Ui) {
    let avail = ui.available_width();
    let col_w = (avail - 14.0) / 2.0;

    ui.horizontal_top(|ui| {
        // ── CV ──
        ui.vertical(|ui| {
            ui.set_width(col_w);
            ui.label(RichText::new("📄  Votre CV").strong());
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

        // ── Job ──
        ui.vertical(|ui| {
            ui.set_width(col_w);
            ui.label(RichText::new("💼  Offre d'emploi").strong());
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

fn render_controls(app: &mut HireLensApp, ui: &mut Ui, ctx: &egui::Context) {
    ui.horizontal(|ui| {
        ui.label(RichText::new("Provider :").color(COL_MUTED));
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
            });

        ui.add_space(12.0);

        if app.is_loading() {
            ui.spinner();
            ui.label(RichText::new("  Traitement en cours…").italics().color(COL_MUTED));
        } else {
            let has_input = !app.cv_text.trim().is_empty() && !app.job_text.trim().is_empty();

            let analyze_btn = ui.add_enabled(
                has_input,
                egui::Button::new(RichText::new("🔍  Analyser").size(14.0))
                    .min_size(Vec2::new(130.0, 32.0)),
            );
            if analyze_btn.clicked() {
                app.start_audit(ctx);
            }

            ui.add_space(6.0);

            let optimize_btn = ui.add_enabled(
                has_input,
                egui::Button::new(RichText::new("✨  Optimiser le CV").size(14.0))
                    .min_size(Vec2::new(160.0, 32.0))
                    .fill(Color32::from_rgb(30, 90, 45)),
            );
            if optimize_btn.clicked() {
                app.start_adapt(ctx);
            }

            if !has_input {
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

fn render_results(app: &mut HireLensApp, ui: &mut Ui) {
    // Extract audit state (no mutable borrow yet)
    let audit_report: Option<AuditReport> =
        if let AuditState::Done(r) = &app.audit_state { Some(r.clone()) } else { None };
    let audit_error: Option<String> =
        if let AuditState::Error(e) = &app.audit_state { Some(e.clone()) } else { None };

    // Extract adapt state (no mutable borrow yet)
    let adapt_data: Option<(String, AuditReport)> =
        if let AdaptState::Done { markdown, audit } = &app.adapt_state {
            Some((markdown.clone(), audit.clone()))
        } else {
            None
        };
    let adapt_error: Option<String> =
        if let AdaptState::Error(e) = &app.adapt_state { Some(e.clone()) } else { None };
    let audit_is_idle = matches!(app.audit_state, AuditState::Idle);

    // Render audit result
    if let Some(msg) = audit_error {
        ui.separator();
        ui.add_space(6.0);
        error_line(ui, &msg);
    } else if let Some(report) = &audit_report {
        ui.separator();
        ui.add_space(6.0);
        render_audit_panel(ui, report);
    }

    // Render adapt result
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
        // Now safe: all immutable borrows of app.adapt_state are released
        render_adapted_panel(app, ui, &audit, &markdown);
    }
}

// ═══════════════════════════ Audit panel ═══════════════════════════

fn render_audit_panel(ui: &mut Ui, report: &AuditReport) {
    ui.horizontal_top(|ui| {
        // Score gauge
        ui.vertical(|ui| {
            ui.set_width(150.0);
            render_gauge(ui, report.score.score);
            ui.add_space(4.0);
            ui.centered_and_justified(|ui| {
                ui.label(
                    RichText::new(format!("{:.0}% match", report.score.skill_match_ratio * 100.0))
                        .size(12.0)
                        .color(COL_MUTED),
                );
            });
        });

        ui.add_space(20.0);

        // Skills
        ui.vertical(|ui| {
            ui.label(RichText::new("✅  Compétences matchées").strong().color(COL_GREEN));
            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                if report.matched_skills.is_empty() {
                    ui.label(RichText::new("aucune").italics().size(12.0).color(COL_MUTED));
                }
                for s in &report.matched_skills {
                    skill_chip(ui, s, COL_GREEN);
                }
            });

            ui.add_space(10.0);

            ui.label(RichText::new("❌  Compétences manquantes").strong().color(COL_RED));
            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                if report.missing_skills.is_empty() {
                    ui.label(RichText::new("aucune").italics().size(12.0).color(COL_MUTED));
                }
                for s in &report.missing_skills {
                    skill_chip(ui, s, COL_RED);
                }
            });

            ui.add_space(10.0);

            egui::CollapsingHeader::new(
                RichText::new("▸ Détail complet des compétences").size(12.0).color(COL_MUTED),
            )
            .default_open(false)
            .show(ui, |ui| {
                ui.label(RichText::new("CV — toutes les compétences détectées").size(11.0).color(COL_MUTED));
                ui.horizontal_wrapped(|ui| {
                    for s in &report.cv_skills {
                        skill_chip(ui, s, COL_YELLOW);
                    }
                });
                ui.add_space(6.0);
                ui.label(RichText::new("Offre — compétences requises").size(11.0).color(COL_MUTED));
                ui.horizontal_wrapped(|ui| {
                    for s in &report.job_skills {
                        skill_chip(ui, s, COL_YELLOW);
                    }
                });
            });
        });
    });

    ui.add_space(8.0);
}

// ═══════════════════════════ Adapted CV panel ═══════════════════════════

fn render_adapted_panel(
    app: &mut HireLensApp,
    ui: &mut Ui,
    _audit: &AuditReport,
    markdown: &str,
) {
    ui.horizontal(|ui| {
        ui.label(RichText::new("✨  CV Optimisé").size(16.0).strong());
        ui.add_space(12.0);

        if ui
            .button(RichText::new("💾  Enregistrer .md").size(13.0))
            .clicked()
        {
            match std::fs::write("cv-optimise.md", markdown) {
                Ok(()) => {
                    app.save_status = Some("✅ Enregistré : cv-optimise.md".to_owned());
                }
                Err(e) => {
                    app.save_status = Some(format!("❌ Erreur : {e}"));
                }
            }
        }

        if let Some(status) = &app.save_status {
            let color = if status.starts_with('✅') { COL_GREEN } else { COL_RED };
            ui.label(RichText::new(status).size(12.0).color(color));
        }
    });

    ui.add_space(8.0);

    // Cloned to avoid borrow conflict — only the markdown string, ~few KB
    let mut display = markdown.to_owned();
    ScrollArea::vertical()
        .id_salt("cv_output_scroll")
        .max_height(400.0)
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

// ═══════════════════════════ Gauge ═══════════════════════════

fn render_gauge(ui: &mut Ui, score: u8) {
    let size = Vec2::splat(130.0);
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    let painter = ui.painter_at(rect);
    let center = rect.center();
    let radius = 52.0_f32;
    let stroke_w = 9.0_f32;

    // Background ring
    painter.circle_stroke(center, radius, Stroke::new(stroke_w, Color32::from_gray(48)));

    // Filled arc
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

fn score_color(score: u8) -> Color32 {
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

// ═══════════════════════════ Widgets ═══════════════════════════

fn skill_chip(ui: &mut Ui, skill: &str, color: Color32) {
    let fill = Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 28);
    Frame::none()
        .fill(fill)
        .rounding(Rounding::same(10.0))
        .inner_margin(Margin { left: 7.0, right: 7.0, top: 2.0, bottom: 2.0 })
        .show(ui, |ui| {
            ui.label(RichText::new(skill).color(color).size(11.5));
        });
}

fn badge(ui: &mut Ui, text: &str, color: Color32) {
    let fill = Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 22);
    Frame::none()
        .fill(fill)
        .rounding(Rounding::same(10.0))
        .inner_margin(Margin { left: 8.0, right: 8.0, top: 2.0, bottom: 2.0 })
        .show(ui, |ui| {
            ui.label(RichText::new(text).color(color).size(11.0).strong());
        });
}

fn error_line(ui: &mut Ui, msg: &str) {
    let fill = Color32::from_rgba_unmultiplied(248, 81, 73, 20);
    Frame::none()
        .fill(fill)
        .rounding(Rounding::same(6.0))
        .inner_margin(Margin::symmetric(12.0, 8.0))
        .show(ui, |ui| {
            ui.label(RichText::new(format!("❌  {msg}")).color(COL_RED).size(13.0));
        });
    ui.add_space(6.0);
}

// ═══════════════════════════ Error formatting ═══════════════════════════

fn friendly_error(msg: String) -> String {
    if msg.contains("10061") || msg.contains("Connection refused") || msg.contains("tcp connect") {
        if msg.contains("11434") {
            return "Ollama n'est pas lancé. Démarrez-le avec : ollama serve".to_owned();
        }
        if msg.contains("1234") {
            return "LM Studio n'est pas lancé. Ouvrez LM Studio et démarrez le serveur local."
                .to_owned();
        }
    }
    if msg.contains("OPENAI_API_KEY") || msg.contains("401") || msg.contains("Unauthorized") {
        return "Clé API OpenAI manquante ou invalide. Définissez OPENAI_API_KEY.".to_owned();
    }
    msg
}

// ═══════════════════════════ Entry point ═══════════════════════════

pub fn run() -> anyhow::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
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
            cc.egui_ctx.set_visuals(egui::Visuals::dark());
            Ok(Box::new(HireLensApp::default()))
        }),
    )
    .map_err(|e| anyhow::anyhow!("GUI error: {e}"))
}
