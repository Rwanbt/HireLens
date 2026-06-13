use std::sync::mpsc;

use crate::core::{Pipeline, PipelineOptions};
use crate::llm::LlmRouter;

use super::app::{FileTarget, HireLensApp};

impl HireLensApp {
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

    pub(crate) fn start_export_pdf(&mut self, markdown: String, ctx: &eframe::egui::Context) {
        let ctx = ctx.clone();
        let (tx, rx) = mpsc::channel();
        self.pdf_rx = Some(rx);
        std::thread::spawn(move || {
            use crate::export::typst_render::TypstRenderer;
            use crate::export::PdfRenderer as _;
            let pdf_result = TypstRenderer.render(&markdown).map_err(|e| e.to_string());
            match pdf_result {
                Ok(bytes) => {
                    let status = rfd::FileDialog::new()
                        .set_file_name("cv-optimise.pdf")
                        .add_filter("PDF", &["pdf"])
                        .save_file()
                        .map(|path| match std::fs::write(&path, &bytes) {
                            Ok(()) => {
                                let _ = open::that(&path);
                                Ok(bytes)
                            }
                            Err(e) => Err(e.to_string()),
                        });
                    let result = match status {
                        Some(r) => r,
                        None => Err("Annulé".to_owned()),
                    };
                    let _ = tx.send(result);
                }
                Err(e) => {
                    let _ = tx.send(Err(e));
                }
            }
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
        let opts = self.build_router_options();
        let ctx = ctx.clone();

        let (tx, rx) = mpsc::channel();
        self.audit_rx = Some(rx);
        self.audit_state = crate::gui::state::AuditState::Loading;
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
                            .audit_text(
                                &cv,
                                &job,
                                PipelineOptions {
                                    offline,
                                    use_cache: true,
                                },
                            )
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
        self.adapt_state = crate::gui::state::AdaptState::Loading;
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
                            .adapt_text(
                                &cv,
                                &job,
                                PipelineOptions {
                                    offline,
                                    use_cache: true,
                                },
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
        return "Clé API OpenAI manquante ou invalide. Configurez-la dans ⚙️ Paramètres."
            .to_owned();
    }
    msg
}
