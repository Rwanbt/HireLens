use eframe::egui::{self, RichText, TextEdit, Ui};

use crate::auth::load_token;
use crate::gui::app::{HireLensApp, Provider};
use crate::gui::settings::GuiSettings;
use crate::gui::theme::{STATUS_ERROR, STATUS_SUCCESS, STATUS_WARNING, TEXT_MUTED, TEXT_SECONDARY};

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
    egui::CollapsingHeader::new(RichText::new("🌟  Google Gemini").strong())
        .default_open(active == Provider::Gemini)
        .show(ui, |ui| {
            let token = load_token();
            let has_api_key = GuiSettings::get_gemini_api_key().is_some();

            // ── Connection status ──
            if has_api_key {
                ui.label(
                    RichText::new("✅ Clé API configurée")
                        .color(STATUS_SUCCESS)
                        .size(12.0),
                );
            } else {
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
                        ui.label(
                            RichText::new("❌ Non connecté à Google")
                                .color(STATUS_ERROR)
                                .size(12.0),
                        );
                    }
                }
            }

            ui.add_space(8.0);

            // ── 1. Login with Google (primary path, uses the embedded OAuth client) ──
            let (embedded_id, _) = crate::auth::embedded_client();
            let has_oauth_client =
                !embedded_id.is_empty() || !app.settings.gemini.client_id.is_empty();
            let auth_in_progress = app.google_auth_rx.is_some();
            ui.horizontal(|ui| {
                if auth_in_progress {
                    ui.spinner();
                    ui.label(
                        RichText::new("  Connexion en cours…")
                            .italics()
                            .size(12.0)
                            .color(TEXT_SECONDARY),
                    );
                } else {
                    if ui
                        .add_enabled(
                            has_oauth_client,
                            egui::Button::new(
                                RichText::new("🔑  Se connecter avec Google").size(13.0),
                            ),
                        )
                        .on_disabled_hover_text(
                            "Cette version n'embarque pas d'identifiants Google — utilisez une clé API ci-dessous",
                        )
                        .clicked()
                    {
                        app.start_google_auth(ctx);
                    }

                    if token.is_some() {
                        ui.add_space(6.0);
                        if ui
                            .button(RichText::new("🔓  Déconnecter").size(12.0).color(STATUS_ERROR))
                            .clicked()
                        {
                            crate::auth::clear_token();
                            app.settings_status = Some("✅ Déconnecté de Google.".to_owned());
                            app.refresh_gemini_available();
                        }
                    }
                }
            });
            if !has_oauth_client {
                ui.label(
                    RichText::new(
                        "ℹ️ « Se connecter avec Google » requiert des identifiants OAuth intégrés à la build (ou un client perso dans Avancé).",
                    )
                    .size(11.0)
                    .color(TEXT_MUTED),
                );
            }

            ui.add_space(10.0);
            ui.separator();
            ui.label(
                RichText::new("Ou colle une clé API Gemini (instantané, sans Google Cloud) :")
                    .size(12.0)
                    .color(TEXT_SECONDARY),
            );
            ui.add_space(4.0);

            // ── 2. API key (alternative path) ──
            render_gemini_api_key(app, ui, has_api_key);

            ui.add_space(8.0);

            // ── 3. Model ──
            render_gemini_model(app, ui);

            ui.add_space(4.0);

            // ── 4. Advanced: bring-your-own OAuth client ──
            render_gemini_advanced(app, ui);
        });
}

fn render_gemini_api_key(app: &mut HireLensApp, ui: &mut Ui, has_api_key: bool) {
    ui.hyperlink_to(
        RichText::new("Obtenir une clé API gratuite (Google AI Studio)").size(12.0),
        "https://aistudio.google.com/apikey",
    );
    ui.add_space(4.0);

    let visible = app.gemini_key_visible;
    ui.add(
        TextEdit::singleline(&mut app.gemini_key_input)
            .desired_width(320.0)
            .password(!visible)
            .hint_text("AIza…"),
    );

    ui.horizontal(|ui| {
        let toggle = if visible {
            "🙈 Masquer"
        } else {
            "👁 Afficher"
        };
        if ui.small_button(toggle).clicked() {
            app.gemini_key_visible = !app.gemini_key_visible;
        }

        ui.add_space(8.0);

        let can_save = !app.gemini_key_input.is_empty();
        if ui
            .add_enabled(
                can_save,
                egui::Button::new(RichText::new("💾 Enregistrer").size(12.0)),
            )
            .clicked()
        {
            match GuiSettings::set_gemini_api_key(&app.gemini_key_input) {
                Ok(()) => {
                    app.settings_status = Some("✅ Clé API Gemini enregistrée.".to_owned());
                    app.gemini_key_input.clear();
                    app.refresh_gemini_available();
                }
                Err(e) => {
                    app.settings_status = Some(format!("❌ Keyring error : {e}"));
                }
            }
        }

        if has_api_key
            && ui
                .small_button(RichText::new("🗑 Effacer").size(12.0).color(STATUS_ERROR))
                .clicked()
        {
            GuiSettings::delete_gemini_api_key();
            app.settings_status = Some("✅ Clé API Gemini supprimée.".to_owned());
            app.refresh_gemini_available();
        }
    });
}

fn render_gemini_model(app: &mut HireLensApp, ui: &mut Ui) {
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
}

fn render_gemini_advanced(app: &mut HireLensApp, ui: &mut Ui) {
    egui::CollapsingHeader::new(
        RichText::new("▸ Client OAuth personnalisé (avancé)")
            .size(12.0)
            .color(TEXT_SECONDARY),
    )
    .default_open(false)
    .show(ui, |ui| {
        ui.label(
            RichText::new(
                "Optionnel — la plupart des utilisateurs n'en ont pas besoin. Pour utiliser votre propre projet Google Cloud (OAuth 2.0 → Application de bureau).",
            )
            .size(11.0)
            .color(TEXT_MUTED),
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
                app.refresh_gemini_available();
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
