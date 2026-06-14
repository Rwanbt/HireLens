use eframe::egui::{self, RichText, TextEdit, Ui};

use crate::auth::load_token;
use crate::gui::app::{HireLensApp, Provider};
use crate::gui::settings::GuiSettings;
use crate::gui::theme::{STATUS_ERROR, STATUS_SUCCESS, STATUS_WARNING, TEXT_SECONDARY};

pub(crate) fn render_settings(app: &mut HireLensApp, ui: &mut Ui, ctx: &egui::Context) {
    // ── Header ──
    ui.horizontal(|ui| {
        if ui.button(RichText::new("← Retour").size(13.0)).clicked() {
            app.settings.save();
            app.show_settings = false;
        }
        ui.add_space(8.0);
        ui.label(RichText::new("⚙️  Paramètres").size(18.0).strong());
    });

    ui.separator();
    ui.add_space(8.0);

    // ── Status message ──
    if let Some(status) = &app.settings_status.clone() {
        let color = if status.starts_with('✅') {
            STATUS_SUCCESS
        } else {
            STATUS_ERROR
        };
        ui.label(RichText::new(status).size(12.0).color(color));
        ui.add_space(6.0);
    }

    // 6.7 — pass provider so each section knows whether to default-open
    let active = app.provider;
    egui::ScrollArea::vertical()
        .auto_shrink(false)
        .show(ui, |ui| {
            render_openai_section(app, ui, active);
            ui.add_space(10.0);
            render_gemini_section(app, ui, ctx, active);
            ui.add_space(10.0);
            render_local_section(app, ui, "🦙  Ollama", true, active);
            ui.add_space(10.0);
            render_local_section(app, ui, "🏠  LM Studio", false, active);
        });
}

// ──────────────────────────────────────────────────────────────
// OpenAI
// ──────────────────────────────────────────────────────────────

fn render_openai_section(app: &mut HireLensApp, ui: &mut Ui, active: Provider) {
    egui::CollapsingHeader::new(RichText::new("✨  OpenAI").strong())
        .default_open(active == Provider::OpenAi)
        .show(ui, |ui| {
            let has_key = GuiSettings::get_openai_key().is_some();

            ui.horizontal(|ui| {
                if has_key {
                    ui.label(
                        RichText::new("✅ Clé configurée")
                            .color(STATUS_SUCCESS)
                            .size(12.0),
                    );
                } else {
                    ui.label(
                        RichText::new("❌ Aucune clé")
                            .color(STATUS_ERROR)
                            .size(12.0),
                    );
                }
            });

            ui.add_space(4.0);

            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("Nouvelle clé API :")
                        .color(TEXT_SECONDARY)
                        .size(12.0),
                );
            });

            let visible = app.openai_key_visible;
            let input = TextEdit::singleline(&mut app.openai_key_input)
                .desired_width(320.0)
                .password(!visible)
                .hint_text("sk-…");
            ui.add(input);

            ui.horizontal(|ui| {
                let toggle_label = if visible {
                    "🙈 Masquer"
                } else {
                    "👁 Afficher"
                };
                if ui.small_button(toggle_label).clicked() {
                    app.openai_key_visible = !app.openai_key_visible;
                }

                ui.add_space(8.0);

                let can_save = !app.openai_key_input.is_empty();
                if ui
                    .add_enabled(
                        can_save,
                        egui::Button::new(RichText::new("💾 Enregistrer").size(12.0)),
                    )
                    .clicked()
                {
                    match GuiSettings::set_openai_key(&app.openai_key_input) {
                        Ok(()) => {
                            app.settings_status = Some("✅ Clé API OpenAI enregistrée.".to_owned());
                            app.openai_key_input.clear();
                        }
                        Err(e) => {
                            app.settings_status = Some(format!("❌ Keyring error : {e}"));
                        }
                    }
                }

                if has_key
                    && ui
                        .small_button(RichText::new("🗑 Effacer").size(12.0).color(STATUS_ERROR))
                        .clicked()
                {
                    GuiSettings::delete_openai_key();
                    app.settings_status = Some("✅ Clé OpenAI supprimée.".to_owned());
                }
            });
        });
}

// ──────────────────────────────────────────────────────────────
// Google Gemini OAuth2
// ──────────────────────────────────────────────────────────────

fn render_gemini_section(
    app: &mut HireLensApp,
    ui: &mut Ui,
    ctx: &egui::Context,
    active: Provider,
) {
    egui::CollapsingHeader::new(RichText::new("🌟  Google Gemini (OAuth2)").strong())
        .default_open(active == Provider::Gemini)
        .show(ui, |ui| {
            // Token status
            let token = load_token();
            match &token {
                Some(t) if !t.is_expired() => {
                    let secs = t.seconds_until_expiry();
                    let label = if secs > 3600 {
                        format!("✅ Connecté — expire dans {}h", secs / 3600)
                    } else if secs > 0 {
                        format!("✅ Connecté — expire dans {}min", secs / 60)
                    } else {
                        "⚠️ Token sur le point d'expirer".to_owned()
                    };
                    ui.label(RichText::new(label).color(STATUS_SUCCESS).size(12.0));
                }
                Some(_) => {
                    ui.label(
                        RichText::new("⚠️ Token expiré — reconnectez-vous")
                            .color(STATUS_WARNING)
                            .size(12.0),
                    );
                }
                None => {
                    ui.label(RichText::new("❌ Non connecté").color(STATUS_ERROR).size(12.0));
                }
            }

            ui.add_space(4.0);

            // Model
            ui.horizontal(|ui| {
                ui.label(RichText::new("Modèle :").color(TEXT_SECONDARY).size(12.0));
                egui::ComboBox::from_id_salt("gemini_model_combo")
                    .selected_text(&app.settings.gemini.model)
                    .width(200.0)
                    .show_ui(ui, |ui| {
                        for m in [
                            "gemini-1.5-flash",
                            "gemini-1.5-pro",
                            "gemini-2.0-flash",
                            "gemini-2.0-pro",
                        ] {
                            if ui
                                .selectable_value(&mut app.settings.gemini.model, m.to_owned(), m)
                                .changed()
                            {
                                app.settings.save();
                            }
                        }
                    });
            });

            ui.add_space(4.0);

            // Credentials (collapsible, for developer setup)
            egui::CollapsingHeader::new(
                RichText::new("▸ Identifiants Google Cloud").size(12.0).color(TEXT_SECONDARY),
            )
            .default_open(app.settings.gemini.client_id.is_empty())
            .show(ui, |ui| {
                ui.label(
                    RichText::new(
                        "Créez un projet Google Cloud → Identifiants → OAuth 2.0 → Application de bureau",
                    )
                    .size(11.0)
                    .color(TEXT_SECONDARY),
                );
                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    ui.label(RichText::new("Client ID :").size(12.0).color(TEXT_SECONDARY));
                    if ui
                        .add(
                            TextEdit::singleline(&mut app.settings.gemini.client_id)
                                .desired_width(280.0)
                                .hint_text("xxxxx.apps.googleusercontent.com"),
                        )
                        .changed()
                    {
                        app.settings.save();
                    }
                });
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Client Secret :").size(12.0).color(TEXT_SECONDARY));
                    if ui
                        .add(
                            TextEdit::singleline(&mut app.settings.gemini.client_secret)
                                .desired_width(240.0)
                                .hint_text("GOCSPX-…"),
                        )
                        .changed()
                    {
                        app.settings.save();
                    }
                });
            });

            ui.add_space(6.0);

            // Auth buttons
            let auth_in_progress = app.google_auth_rx.is_some();
            ui.horizontal(|ui| {
                if auth_in_progress {
                    ui.spinner();
                    ui.label(
                        RichText::new("  Authentification en cours…").italics().size(12.0).color(TEXT_SECONDARY),
                    );
                } else {
                    let has_credentials = !app.settings.gemini.client_id.is_empty();
                    if ui
                        .add_enabled(
                            has_credentials,
                            egui::Button::new(RichText::new("🔑  Connexion Google").size(13.0)),
                        )
                        .on_disabled_hover_text("Configurez d'abord le Client ID ci-dessus")
                        .clicked()
                    {
                        app.start_google_auth(ctx);
                    }

                    if token.is_some() {
                        ui.add_space(4.0);
                        if ui
                            .button(RichText::new("🔓  Déconnecter").size(12.0).color(STATUS_ERROR))
                            .clicked()
                        {
                            crate::auth::clear_token();
                            app.settings_status = Some("✅ Déconnecté de Google.".to_owned());
                        }
                    }
                }
            });
        });
}

// ──────────────────────────────────────────────────────────────
// Ollama / LM Studio (shared layout)
// ──────────────────────────────────────────────────────────────

fn render_local_section(
    app: &mut HireLensApp,
    ui: &mut Ui,
    title: &str,
    is_ollama: bool,
    active: Provider,
) {
    let open = if is_ollama {
        matches!(active, Provider::Ollama | Provider::Offline)
    } else {
        active == Provider::LmStudio
    };
    egui::CollapsingHeader::new(RichText::new(title).strong())
        .default_open(open)
        .show(ui, |ui| {
            let ping_status = if is_ollama {
                app.ping_status.map(|(o, _)| o)
            } else {
                app.ping_status.map(|(_, l)| l)
            };

            // Status badge
            match ping_status {
                Some(true) => {
                    ui.label(
                        RichText::new("✅ En ligne")
                            .color(STATUS_SUCCESS)
                            .size(12.0),
                    );
                }
                Some(false) => {
                    ui.label(
                        RichText::new("❌ Hors ligne")
                            .color(STATUS_ERROR)
                            .size(12.0),
                    );
                }
                None => {
                    ui.label(
                        RichText::new("● non testé")
                            .color(TEXT_SECONDARY)
                            .size(12.0),
                    );
                }
            }

            ui.add_space(4.0);

            // URL
            ui.horizontal(|ui| {
                ui.label(RichText::new("URL :").size(12.0).color(TEXT_SECONDARY));
                let changed = if is_ollama {
                    ui.add(
                        TextEdit::singleline(&mut app.settings.ollama_url)
                            .desired_width(260.0)
                            .hint_text("http://localhost:11434"),
                    )
                    .changed()
                } else {
                    ui.add(
                        TextEdit::singleline(&mut app.settings.lmstudio_url)
                            .desired_width(260.0)
                            .hint_text("http://localhost:1234/v1"),
                    )
                    .changed()
                };
                if changed {
                    app.settings.save();
                    app.ping_status = None; // reset status when URL changes
                }
            });

            // Model
            ui.horizontal(|ui| {
                ui.label(RichText::new("Modèle :").size(12.0).color(TEXT_SECONDARY));
                let changed = if is_ollama {
                    ui.add(
                        TextEdit::singleline(&mut app.settings.ollama_model)
                            .desired_width(200.0)
                            .hint_text("llama3.1"),
                    )
                    .changed()
                } else {
                    ui.add(
                        TextEdit::singleline(&mut app.settings.lmstudio_model)
                            .desired_width(200.0)
                            .hint_text("local-model"),
                    )
                    .changed()
                };
                if changed {
                    app.settings.save();
                }
            });

            ui.add_space(4.0);

            let pinging = app.ping_rx.is_some();
            if pinging {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(
                        RichText::new("  Test en cours…")
                            .italics()
                            .size(12.0)
                            .color(TEXT_SECONDARY),
                    );
                });
            } else if ui
                .button(RichText::new("🔄 Tester la connexion").size(12.0))
                .clicked()
            {
                app.start_ping();
            }
        });
}
