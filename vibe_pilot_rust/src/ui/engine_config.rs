use crate::app::VibePilotApp;
use crate::event_bus::NotificationEvent;
use eframe::egui;

pub fn render_engine_config_section(ui: &mut egui::Ui, app: &mut VibePilotApp) {
    // --- Section 1: AI Engine & Model ---
    ui.group(|ui| {
        ui.label(app.t("frame_ia")).on_hover_text(app.t("tip_frame_ia"));
        ui.separator();

        ui.horizontal_wrapped(|ui| {
            ui.label(app.t("lbl_moteur")).on_hover_text(app.t("tip_frame_ia"));
            let mut engines = app.engine_presets.all_keys();
            engines.sort_by(|a, b| a.to_lowercase().cmp(&b.to_lowercase()));
            let current = app.current_config.moteur.clone();
            let mut engine_sel = current.clone();
            egui::ComboBox::from_id_salt("engine_combo")
                .width(120.0)
                .selected_text(&current)
                .show_ui(ui, |ui| {
                    for engine in &engines {
                        ui.selectable_value(&mut engine_sel, engine.clone(), engine);
                    }
                });
            if engine_sel != current {
                app.current_config.moteur = engine_sel.clone();
                if let Some(preset) = app.engine_presets.get(&engine_sel) {
                    app.current_config.url_api = preset.url.clone();
                }
            }

            if ui.button("+").on_hover_text(app.t("tip_add_engine")).clicked() {
                app.show_add_engine = true;
            }

            if ui.button("🗑").on_hover_text(app.t("tip_del_engine")).clicked() {
                let current_engine = app.current_config.moteur.clone();
                if current_engine != "LM Studio" && current_engine != "Ollama" && current_engine != "Perso / Autre" {
                    app.engine_presets.remove(&current_engine);
                    app.config_repo.save_engines(&app.engine_presets);
                    app.current_config.moteur = "LM Studio".to_string();
                    app.bus.emit_notification(NotificationEvent::Log(format!("Engine preset '{}' deleted", current_engine)));
                }
            }

            ui.add_space(8.0);
            ui.label(app.t("lbl_url")).on_hover_text(app.t("tip_frame_ia"));
            ui.add(egui::TextEdit::singleline(&mut app.current_config.url_api).desired_width(180.0));

            ui.add_space(8.0);
            ui.label(app.t("lbl_modele")).on_hover_text(app.t("tip_frame_ia"));
            let mut model = app.current_config.nom_modele.clone();
            let mut model_sel = model.clone();
            ui.add(egui::TextEdit::singleline(&mut model).desired_width(180.0));
            egui::ComboBox::from_id_salt("model_combo_suggestions")
                .width(16.0)
                .selected_text("")
                .show_ui(ui, |ui| {
                    if let Some(preset) = app.engine_presets.get(&app.current_config.moteur) {
                        let mut models = preset.modeles.clone();
                        models.sort_by(|a, b| a.to_lowercase().cmp(&b.to_lowercase()));
                        for m in &models {
                            ui.selectable_value(&mut model_sel, m.clone(), m);
                        }
                    }
                });
            if model_sel != model && model_sel != app.current_config.nom_modele {
                model = model_sel;
            }
            app.current_config.nom_modele = model;

            ui.add_space(4.0);
            let scan_lbl = "🔄 Scan";
            let scan_tip = if app.current_config.langue == "Français" { "Scanner les modèles disponibles sur ce moteur" } else { "Scan available models on this engine" };
            if ui.button(scan_lbl).on_hover_text(scan_tip).clicked() {
                app.scan_models();
            }
        });

        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.label(app.t("lbl_auth_mode"));
            let mut auth_sel = app.current_config.auth_mode.clone();
            egui::ComboBox::from_id_salt("auth_mode_combo")
                .width(100.0)
                .selected_text(match auth_sel.as_str() {
                    "api_key" => app.t("lbl_auth_api_key"),
                    "basic_auth" => app.t("lbl_auth_basic"),
                    _ => app.t("lbl_auth_none"),
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut auth_sel, "none".to_string(), app.t("lbl_auth_none"));
                    ui.selectable_value(&mut auth_sel, "api_key".to_string(), app.t("lbl_auth_api_key"));
                    ui.selectable_value(&mut auth_sel, "basic_auth".to_string(), app.t("lbl_auth_basic"));
                });
            if auth_sel != app.current_config.auth_mode {
                app.current_config.auth_mode = auth_sel;
            }

            if app.current_config.auth_mode == "api_key" {
                ui.add_space(8.0);
                ui.label(app.t("lbl_api_key"));
                ui.add(egui::TextEdit::singleline(&mut app.current_config.auth_api_key).password(true).desired_width(120.0));
            } else if app.current_config.auth_mode == "basic_auth" {
                ui.add_space(8.0);
                ui.label(app.t("lbl_auth_login"));
                ui.add(egui::TextEdit::singleline(&mut app.current_config.auth_login).desired_width(100.0));
                ui.add_space(4.0);
                ui.label(app.t("lbl_auth_password"));
                ui.add(egui::TextEdit::singleline(&mut app.current_config.auth_password).password(true).desired_width(100.0));
            }

            ui.add_space(16.0);
            ui.label(app.t("lbl_request_timeout"));
            ui.add(egui::Slider::new(&mut app.current_config.request_timeout_secs, 10..=3600).suffix("s"));
        });

        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.checkbox(
                &mut app.current_config.utiliser_moteur_vision_dedie,
                if app.current_config.langue == "Français" {
                    "Utiliser un modèle de vision dédié (conseillé pour la rapidité)"
                } else {
                    "Use a dedicated Vision Model (recommended for speed)"
                }
            );
        });

        if app.current_config.utiliser_moteur_vision_dedie {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(if app.current_config.langue == "Français" { "Moteur Vision :" } else { "Vision Engine:" });
                let mut engines = app.engine_presets.all_keys();
                engines.sort_by(|a, b| a.to_lowercase().cmp(&b.to_lowercase()));
                let current_v_engine = app.current_config.moteur_vision.clone();
                let mut v_engine_sel = current_v_engine.clone();
                egui::ComboBox::from_id_salt("vision_engine_combo")
                    .width(120.0)
                    .selected_text(&current_v_engine)
                    .show_ui(ui, |ui| {
                        for engine in &engines {
                            ui.selectable_value(&mut v_engine_sel, engine.clone(), engine);
                        }
                    });
                if v_engine_sel != current_v_engine {
                    app.current_config.moteur_vision = v_engine_sel.clone();
                    if let Some(preset) = app.engine_presets.get(&v_engine_sel) {
                        app.current_config.url_api_vision = preset.url.clone();
                    }
                }

                ui.add_space(8.0);
                ui.label(if app.current_config.langue == "Français" { "URL Vision :" } else { "Vision URL:" });
                ui.add(egui::TextEdit::singleline(&mut app.current_config.url_api_vision).desired_width(180.0));

                ui.add_space(8.0);
                ui.label(if app.current_config.langue == "Français" { "Modèle Vision :" } else { "Vision Model:" });
                let mut v_model = app.current_config.nom_modele_vision.clone();
                let mut v_model_sel = v_model.clone();
                ui.add(egui::TextEdit::singleline(&mut v_model).desired_width(180.0));
                egui::ComboBox::from_id_salt("vision_model_combo_suggestions")
                    .width(16.0)
                    .selected_text("")
                    .show_ui(ui, |ui| {
                        if let Some(preset) = app.engine_presets.get(&app.current_config.moteur_vision) {
                            let mut models = preset.modeles.clone();
                            models.sort_by(|a, b| a.to_lowercase().cmp(&b.to_lowercase()));
                            for m in &models {
                                ui.selectable_value(&mut v_model_sel, m.clone(), m);
                            }
                        }
                    });
                if v_model_sel != v_model && v_model_sel != app.current_config.nom_modele_vision {
                    v_model = v_model_sel;
                }
                app.current_config.nom_modele_vision = v_model;

                ui.add_space(4.0);
                let scan_lbl = "🔄 Scan";
                let scan_tip = if app.current_config.langue == "Français" { "Scanner les modèles disponibles sur ce moteur de vision" } else { "Scan available models on this vision engine" };
                if ui.button(scan_lbl).on_hover_text(scan_tip).clicked() {
                    app.scan_models_vision();
                }
            });

            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(if app.current_config.langue == "Français" { "Auth Vision :" } else { "Vision Auth:" });
                let mut auth_v_sel = app.current_config.auth_mode_vision.clone();
                egui::ComboBox::from_id_salt("vision_auth_mode_combo")
                    .width(100.0)
                    .selected_text(match auth_v_sel.as_str() {
                        "api_key" => app.t("lbl_auth_api_key"),
                        "basic_auth" => app.t("lbl_auth_basic"),
                        _ => app.t("lbl_auth_none"),
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut auth_v_sel, "none".to_string(), app.t("lbl_auth_none"));
                        ui.selectable_value(&mut auth_v_sel, "api_key".to_string(), app.t("lbl_auth_api_key"));
                        ui.selectable_value(&mut auth_v_sel, "basic_auth".to_string(), app.t("lbl_auth_basic"));
                    });
                if auth_v_sel != app.current_config.auth_mode_vision {
                    app.current_config.auth_mode_vision = auth_v_sel;
                }

                if app.current_config.auth_mode_vision == "api_key" {
                    ui.add_space(8.0);
                    ui.label(app.t("lbl_api_key"));
                    ui.add(egui::TextEdit::singleline(&mut app.current_config.auth_api_key_vision).password(true).desired_width(120.0));
                } else if app.current_config.auth_mode_vision == "basic_auth" {
                    ui.add_space(8.0);
                    ui.label(app.t("lbl_auth_login"));
                    ui.add(egui::TextEdit::singleline(&mut app.current_config.auth_login_vision).desired_width(100.0));
                    ui.add_space(4.0);
                    ui.label(app.t("lbl_auth_password"));
                    ui.add(egui::TextEdit::singleline(&mut app.current_config.auth_password_vision).password(true).desired_width(100.0));
                }

                ui.add_space(16.0);
                ui.label(if app.current_config.langue == "Français" { "Timeout Vision :" } else { "Vision Timeout:" });
                ui.add(egui::Slider::new(&mut app.current_config.request_timeout_secs_vision, 10..=3600).suffix("s"));
            });
        }

        if app.show_add_engine {
            ui.add_space(6.0);
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.label("Name:");
                    ui.text_edit_singleline(&mut app.new_engine_name);
                    ui.label("URL:");
                    ui.text_edit_singleline(&mut app.new_engine_url);
                    if ui.button("Save Preset").clicked()
                        && !app.new_engine_name.is_empty() && !app.new_engine_url.is_empty()
                    {
                        let new_profile = crate::config::EngineProfile {
                            url: app.new_engine_url.clone(),
                            modeles: vec!["gpt-4o".to_string(), "qwen2.5-vl-7b-instruct".to_string(), "meta-llama-3-8b-instruct".to_string()],
                        };
                        app.engine_presets.insert(app.new_engine_name.clone(), new_profile);
                        app.config_repo.save_engines(&app.engine_presets);
                        app.current_config.moteur = app.new_engine_name.clone();
                        app.current_config.url_api = app.new_engine_url.clone();
                        app.show_save_success_popup = Some(format!("engine:{}", app.new_engine_name));
                        app.show_add_engine = false;
                        app.new_engine_name = String::new();
                        app.new_engine_url = String::new();
                    }
                    if ui.button("Cancel").clicked() {
                        app.show_add_engine = false;
                    }
                });
            });
        }
    });
}
