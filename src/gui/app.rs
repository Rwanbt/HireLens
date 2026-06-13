use std::sync::mpsc;

use crate::core::{AuditReport, Pipeline, PipelineOptions};
use crate::llm::{LlmProviderKind, LlmRouter};

use super::state::{AdaptState, AuditState};

#[derive(Default, PartialEq, Clone, Copy)]
pub(crate) enum Provider {
    #[default]
    Offline,
    Ollama,
    LmStudio,
    OpenAi,
}

impl Provider {
    pub(crate) fn label(self) -> &'static str {
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

/// Identifies which text field an async file-open targets.
pub(crate) enum FileTarget {
    Cv,
    Job,
}

pub struct HireLensApp {
    pub(crate) cv_text: String,
    pub(crate) job_text: String,
    pub(crate) provider: Provider,

    pub(crate) audit_state: AuditState,
    pub(crate) audit_rx: Option<mpsc::Receiver<Result<AuditReport, String>>>,

    pub(crate) adapt_state: AdaptState,
    pub(crate) adapt_rx: Option<mpsc::Receiver<Result<(String, AuditReport), String>>>,

    pub(crate) save_status: Option<String>,

    /// Async file-open: receives (target, file content or None if cancelled/error).
    pub(crate) file_rx: Option<mpsc::Receiver<(FileTarget, Option<String>)>>,
    /// Async save/export: receives the status message to display, or None if cancelled.
    pub(crate) save_rx: Option<mpsc::Receiver<Option<String>>>,
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
            file_rx: None,
            save_rx: None,
        }
    }
}

impl HireLensApp {
    pub(crate) fn is_loading(&self) -> bool {
        self.audit_rx.is_some() || self.adapt_rx.is_some()
    }

    pub(crate) fn is_saving(&self) -> bool {
        self.save_rx.is_some()
    }

    #[allow(dead_code)] // used in PR 2.5 (Typst PDF export)
    pub(crate) fn adapted_markdown(&self) -> Option<&str> {
        if let AdaptState::Done { markdown, .. } = &self.adapt_state {
            Some(markdown.as_str())
        } else {
            None
        }
    }

    pub(crate) fn poll_results(&mut self) {
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
        if let Some(rx) = &self.file_rx {
            if let Ok((target, content)) = rx.try_recv() {
                if let Some(text) = content {
                    match target {
                        FileTarget::Cv => self.cv_text = text,
                        FileTarget::Job => self.job_text = text,
                    }
                }
                self.file_rx = None;
            }
        }
        if let Some(rx) = &self.save_rx {
            if let Ok(status) = rx.try_recv() {
                self.save_status = status;
                self.save_rx = None;
            }
        }
    }

    pub(crate) fn start_open_file(&mut self, target: FileTarget, ctx: &eframe::egui::Context) {
        let ctx = ctx.clone();
        let (tx, rx) = mpsc::channel();
        self.file_rx = Some(rx);
        std::thread::spawn(move || {
            let content = rfd::FileDialog::new()
                .add_filter("Documents", &["md", "txt"])
                .pick_file()
                .and_then(|p| std::fs::read_to_string(p).ok());
            let _ = tx.send((target, content));
            ctx.request_repaint();
        });
    }

    pub(crate) fn start_save_md(&mut self, markdown: String, ctx: &eframe::egui::Context) {
        let ctx = ctx.clone();
        let (tx, rx) = mpsc::channel();
        self.save_rx = Some(rx);
        std::thread::spawn(move || {
            let status = rfd::FileDialog::new()
                .set_file_name("cv-optimise.md")
                .add_filter("Markdown", &["md"])
                .save_file()
                .map(|path| match std::fs::write(&path, &markdown) {
                    Ok(()) => Some(format!("✅ Enregistré : {}", path.display())),
                    Err(e) => Some(format!("❌ Erreur : {e}")),
                })
                .unwrap_or(None);
            let _ = tx.send(status);
            ctx.request_repaint();
        });
    }

    pub(crate) fn start_export_html(&mut self, markdown: String, ctx: &eframe::egui::Context) {
        let ctx = ctx.clone();
        let (tx, rx) = mpsc::channel();
        self.save_rx = Some(rx);
        std::thread::spawn(move || {
            let html = crate::gui::html_export::to_html(&markdown);
            let status = rfd::FileDialog::new()
                .set_file_name("cv-optimise.html")
                .add_filter("HTML", &["html"])
                .save_file()
                .map(|path| match std::fs::write(&path, &html) {
                    Ok(()) => {
                        let _ = open::that(&path);
                        Some(format!("✅ HTML exporté : {}", path.display()))
                    }
                    Err(e) => Some(format!("❌ Erreur : {e}")),
                })
                .unwrap_or(None);
            let _ = tx.send(status);
            ctx.request_repaint();
        });
    }

    pub(crate) fn start_audit(&mut self, ctx: &eframe::egui::Context) {
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
                            .audit_text(&cv, &job, PipelineOptions { offline, use_cache: true })
                            .await
                            .map_err(|e| friendly_error(e.to_string()))
                    })
                });
            let _ = tx.send(result);
            ctx.request_repaint();
        });
    }

    pub(crate) fn start_adapt(&mut self, ctx: &eframe::egui::Context) {
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
                            .adapt_text(&cv, &job, PipelineOptions { offline, use_cache: true })
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

impl eframe::App for HireLensApp {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        use crate::gui::views::main_view;
        use eframe::egui::ScrollArea;

        self.poll_results();
        if self.is_loading() {
            ctx.request_repaint_after(std::time::Duration::from_millis(80));
        }

        main_view::render_header(ctx);

        eframe::egui::CentralPanel::default().show(ctx, |ui| {
            ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
                ui.add_space(12.0);
                main_view::render_inputs(self, ui, ctx);
                ui.add_space(12.0);
                main_view::render_controls(self, ui, ctx);
                ui.add_space(8.0);
                main_view::render_results(self, ui, ctx);
            });
        });
    }
}

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
