use std::sync::mpsc;

use crate::core::AuditReport;
use crate::llm::{GuiRouterOptions, LlmProviderKind};

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

    pub(super) fn to_kind(self) -> (LlmProviderKind, bool) {
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
    pub(crate) pdf_rx: Option<mpsc::Receiver<Result<Vec<u8>, String>>>,

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
            pdf_rx: None,
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
        self.save_rx.is_some() || self.pdf_rx.is_some()
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
        if let Some(rx) = &self.pdf_rx {
            if let Ok(result) = rx.try_recv() {
                self.save_status = Some(match result {
                    Ok(_) => "✅ PDF enregistré.".to_owned(),
                    Err(e) => format!("❌ PDF : {e}"),
                });
                self.pdf_rx = None;
            }
        }
    }

    // ── Internal helpers ──

    pub(super) fn build_router_options(&self) -> GuiRouterOptions {
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

