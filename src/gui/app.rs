use std::sync::mpsc;

use crate::core::{AuditReport, Pipeline, PipelineOptions};
use crate::llm::{GuiRouterOptions, LlmProviderKind, LlmRouter};

use super::settings::GuiSettings;
use super::state::{AdaptState, AuditState};

// ──────────────────────────────────────────────────────────────
// Provider enum
// ──────────────────────────────────────────────────────────────

#[derive(Default, PartialEq, Clone, Copy)]
pub(crate) enum Provider {
    #[default]
    Offline,
    Ollama,
    LmStudio,
    OpenAi,
    Gemini,
}

impl Provider {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Offline => "🔌 Offline (sans IA)",
            Self::Ollama => "🦙 Ollama (local)",
            Self::LmStudio => "🏠 LM Studio (local)",
            Self::OpenAi => "✨ OpenAI",
            Self::Gemini => "🌟 Google Gemini",
        }
    }

    fn to_kind(self) -> (LlmProviderKind, bool) {
        match self {
            Self::Offline => (LlmProviderKind::Ollama, true),
            Self::Ollama => (LlmProviderKind::Ollama, false),
            Self::LmStudio => (LlmProviderKind::LmStudio, false),
            Self::OpenAi => (LlmProviderKind::OpenAi, false),
            Self::Gemini => (LlmProviderKind::Gemini, false),
        }
    }
}

// ──────────────────────────────────────────────────────────────
// Enums
// ──────────────────────────────────────────────────────────────

/// Identifies which text field an async file-open targets.
pub(crate) enum FileTarget {
    Cv,
    Job,
}

// ──────────────────────────────────────────────────────────────
// App state
// ──────────────────────────────────────────────────────────────

pub struct HireLensApp {
    // ── Inputs ──
    pub(crate) cv_text: String,
    pub(crate) job_text: String,
    pub(crate) provider: Provider,

    // ── LLM results ──
    pub(crate) audit_state: AuditState,
    pub(crate) audit_rx: Option<mpsc::Receiver<Result<AuditReport, String>>>,
    pub(crate) adapt_state: AdaptState,
    pub(crate) adapt_rx: Option<mpsc::Receiver<Result<(String, AuditReport), String>>>,

    // ── File I/O ──
    pub(crate) save_status: Option<String>,
    pub(crate) file_rx: Option<mpsc::Receiver<(FileTarget, Option<String>)>>,
    pub(crate) save_rx: Option<mpsc::Receiver<Option<String>>>,

    // ── Settings panel ──
    pub(crate) show_settings: bool,
    pub(crate) settings: GuiSettings,
    pub(crate) settings_status: Option<String>,
    pub(crate) openai_key_input: String,
    pub(crate) openai_key_visible: bool,

    // ── Google OAuth2 ──
    pub(crate) google_auth_rx: Option<mpsc::Receiver<Result<(), String>>>,

    // ── Provider ping ──
    pub(crate) ping_rx: Option<mpsc::Receiver<(bool, bool)>>,
    pub(crate) ping_status: Option<(bool, bool)>,
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
            show_settings: false,
            settings: GuiSettings::load(),
            settings_status: None,
            openai_key_input: String::new(),
            openai_key_visible: false,
            google_auth_rx: None,
            ping_rx: None,
            ping_status: None,
        }
    }
}

// ──────────────────────────────────────────────────────────────
// Methods
// ──────────────────────────────────────────────────────────────

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
        if let Some(rx) = &self.google_auth_rx {
            if let Ok(result) = rx.try_recv() {
                self.settings_status = match result {
                    Ok(()) => Some("✅ Connecté à Google Gemini !".to_owned()),
                    Err(msg) => Some(format!("❌ {msg}")),
                };
                self.google_auth_rx = None;
            }
        }
        if let Some(rx) = &self.ping_rx {
            if let Ok(status) = rx.try_recv() {
                self.ping_status = Some(status);
                self.ping_rx = None;
            }
        }
    }

    // ── File dialogs ──

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

    // ── LLM operations ──

    pub(crate) fn start_audit(&mut self, ctx: &eframe::egui::Context) {
        let cv = self.cv_text.clone();
        let job = self.job_text.clone();
        let (kind, offline) = self.provider.to_kind();
        let opts = self.build_router_options();
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
                        let router = LlmRouter::from_gui(kind, &opts)
                            .await
                            .map_err(|e| friendly_error(e.to_string()))?;
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
        let opts = self.build_router_options();
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
                        let router = LlmRouter::from_gui(kind, &opts)
                            .await
                            .map_err(|e| friendly_error(e.to_string()))?;
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

    // ── OAuth2 ──

    pub(crate) fn start_google_auth(&mut self, ctx: &eframe::egui::Context) {
        let client_id = self.settings.gemini.client_id.clone();
        let client_secret = self.settings.gemini.client_secret.clone();
        let ctx = ctx.clone();
        let (tx, rx) = mpsc::channel();
        self.google_auth_rx = Some(rx);
        std::thread::spawn(move || {
            let result = crate::auth::start_google_oauth_sync(&client_id, &client_secret)
                .map_err(|e| e.to_string());
            let _ = tx.send(result);
            ctx.request_repaint();
        });
    }

    // ── Provider ping ──

    pub(crate) fn start_ping(&mut self) {
        let ollama_url = self.settings.ollama_url.clone();
        let lmstudio_url = self.settings.lmstudio_url.clone();
        let (tx, rx) = mpsc::channel();
        self.ping_rx = Some(rx);
        std::thread::spawn(move || {
            let result = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map(|rt| {
                    rt.block_on(async {
                        let client = reqwest::Client::builder()
                            .timeout(std::time::Duration::from_secs(2))
                            .build()
                            .unwrap_or_else(|_| reqwest::Client::new());
                        let ollama_ok = client
                            .get(format!("{}/api/tags", ollama_url.trim_end_matches('/')))
                            .send()
                            .await
                            .map(|r| r.status().is_success())
                            .unwrap_or(false);
                        let lmstudio_ok = client
                            .get(format!("{}/models", lmstudio_url.trim_end_matches('/')))
                            .send()
                            .await
                            .map(|r| r.status().is_success())
                            .unwrap_or(false);
                        (ollama_ok, lmstudio_ok)
                    })
                })
                .unwrap_or((false, false));
            let _ = tx.send(result);
        });
    }

    // ── Internal helpers ──

    fn build_router_options(&self) -> GuiRouterOptions {
        GuiRouterOptions {
            ollama_url: self.settings.ollama_url.clone(),
            ollama_model: self.settings.ollama_model.clone(),
            lmstudio_url: self.settings.lmstudio_url.clone(),
            lmstudio_model: self.settings.lmstudio_model.clone(),
            gemini_model: self.settings.gemini.model.clone(),
            gemini_client_id: self.settings.gemini.client_id.clone(),
            gemini_client_secret: self.settings.gemini.client_secret.clone(),
        }
    }
}

// ──────────────────────────────────────────────────────────────
// eframe::App
// ──────────────────────────────────────────────────────────────

impl eframe::App for HireLensApp {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        use crate::gui::views::{main_view, settings_view};
        use eframe::egui::ScrollArea;

        self.poll_results();
        if self.is_loading() {
            ctx.request_repaint_after(std::time::Duration::from_millis(80));
        }

        main_view::render_header(self, ctx);

        eframe::egui::CentralPanel::default().show(ctx, |ui| {
            ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
                ui.add_space(12.0);
                if self.show_settings {
                    settings_view::render_settings(self, ui, ctx);
                } else {
                    main_view::render_inputs(self, ui, ctx);
                    ui.add_space(12.0);
                    main_view::render_controls(self, ui, ctx);
                    ui.add_space(8.0);
                    main_view::render_results(self, ui, ctx);
                }
            });
        });
    }
}

// ──────────────────────────────────────────────────────────────
// Error formatting
// ──────────────────────────────────────────────────────────────

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
        return "Clé API OpenAI manquante ou invalide. Configurez-la dans ⚙️ Paramètres.".to_owned();
    }
    msg
}
