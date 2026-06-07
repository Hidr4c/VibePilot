use crate::app::VibePilotApp;
use crate::event_bus::EventType;
use eframe::egui;

// ============================================================
// TAB 1: Global Configuration
// ============================================================
pub fn render_config_tab(ui: &mut egui::Ui, app: &mut VibePilotApp) {
    let available_width = ui.available_width();

    // --- Section 1: AI Engine & Model ---
    ui.group(|ui| {
        ui.label(app.t("frame_ia")).on_hover_text(app.t("tip_frame_ia"));
        ui.separator();

        ui.horizontal_wrapped(|ui| {
            ui.label(app.t("lbl_moteur")).on_hover_text(app.t("tip_frame_ia"));
            let engines = app.engine_presets.all_keys();
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
                    app.bus.emit(EventType::Log(format!("Engine preset '{}' deleted", current_engine)));
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
                        for m in &preset.modeles {
                            ui.selectable_value(&mut model_sel, m.clone(), m);
                        }
                    }
                });
            if model_sel != model && model_sel != app.current_config.nom_modele {
                model = model_sel;
            }
            app.current_config.nom_modele = model;
        });

        if app.show_add_engine {
            ui.add_space(6.0);
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.label("Name:");
                    ui.text_edit_singleline(&mut app.new_engine_name);
                    ui.label("URL:");
                    ui.text_edit_singleline(&mut app.new_engine_url);
                    if ui.button("Save Preset").clicked() {
                        if !app.new_engine_name.is_empty() && !app.new_engine_url.is_empty() {
                            let new_profile = crate::config::EngineProfile {
                                url: app.new_engine_url.clone(),
                                modeles: vec!["gpt-4o".to_string(), "qwen2.5-vl-7b-instruct".to_string(), "meta-llama-3-8b-instruct".to_string()],
                            };
                            app.engine_presets.insert(app.new_engine_name.clone(), new_profile);
                            app.config_repo.save_engines(&app.engine_presets);
                            app.current_config.moteur = app.new_engine_name.clone();
                            app.current_config.url_api = app.new_engine_url.clone();
                            app.show_add_engine = false;
                            app.new_engine_name = String::new();
                            app.new_engine_url = String::new();
                        }
                    }
                    if ui.button("Cancel").clicked() {
                        app.show_add_engine = false;
                    }
                });
            });
        }
    });

    ui.add_space(12.0);

    // --- Section 2: Target Applications ---
    ui.group(|ui| {
        ui.horizontal(|ui| {
            ui.label(app.t("frame_windows")).on_hover_text(app.t("tip_frame_windows"));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let refresh_label = if app.current_config.langue == "Français" { "🔄 Rafraîchir" } else { "🔄 Refresh List" };
                if ui.button(refresh_label).on_hover_text(app.t("tip_refresh")).clicked() {
                    app.orchestrator.refresh_windows();
                    app.bus.emit(EventType::Log("Refreshing window list...".to_string()));
                }
            });
        });
        ui.separator();

        ui.horizontal(|ui| {
            let all_windows = app.orchestrator.get_available_windows();
            let current = &app.current_config.fenetres_surveillees;
            let mut selected_win: Option<String> = None;
            
            let select_label = if app.current_config.langue == "Français" { "Sélectionner application :" } else { "Select Window:" };
            ui.label(select_label).on_hover_text(app.t("tip_frame_windows"));
            
            let combobox_hint = if app.current_config.langue == "Français" { "Choisir une fenêtre à surveiller..." } else { "Choose a window to add..." };
            egui::ComboBox::from_id_salt("window_combo")
                .width(available_width * 0.6)
                .selected_text(combobox_hint)
                .show_ui(ui, |ui| {
                    for w in &all_windows {
                        if !current.contains(w) {
                            ui.selectable_value(&mut selected_win, Some(w.clone()), w);
                        }
                    }
                });

            if let Some(win) = selected_win {
                let mut windows = app.current_config.fenetres_surveillees.clone();
                if win.to_lowercase().contains("all screens") || win.to_lowercase().contains("tous les") {
                    windows.clear();
                } else {
                    windows.retain(|f| {
                        !f.to_lowercase().contains("all screens") && !f.to_lowercase().contains("tous les")
                    });
                }
                windows.push(win.clone());
                app.current_config.fenetres_surveillees = windows;
                app.bus.emit(EventType::Log(format!("Window added: {}", win)));
            }
        });

        ui.add_space(8.0);
        ui.label(app.t("lbl_selection_fenetres")).on_hover_text(app.t("tip_frame_windows"));
        
        let windows = &app.current_config.fenetres_surveillees;
        if windows.is_empty() {
            ui.horizontal(|ui| {
                ui.colored_label(egui::Color32::from_rgb(255, 215, 0), app.t("lbl_aucune_fenetre"));
            });
        } else {
            let mut windows_clone = windows.clone();
            let mut to_remove: Option<usize> = None;
            for (i, title) in windows_clone.iter().enumerate() {
                ui.horizontal(|ui| {
                    let color = if title.to_lowercase().contains("all screens") {
                        egui::Color32::from_rgb(50, 205, 50) // Light green
                    } else {
                        egui::Color32::from_rgb(100, 255, 255) // Custom Cyan
                    };
                    ui.colored_label(color, if title.to_lowercase().contains("all screens") { "🖥️" } else { "📱" });
                    ui.label(title);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("❌").clicked() {
                            to_remove = Some(i);
                        }
                    });
                });
            }
            if let Some(idx) = to_remove {
                windows_clone.remove(idx);
                app.current_config.fenetres_surveillees = windows_clone;
            }
        }
    });

    ui.add_space(12.0);

    // --- Section 3: Profile Management ---
    ui.group(|ui| {
        ui.label(app.t("frame_profils")).on_hover_text(app.t("tip_frame_profils"));
        ui.separator();
        
        ui.horizontal(|ui| {
            ui.label(app.t("lbl_profil")).on_hover_text(app.t("tip_frame_profils"));
            let profiles = app.config_repo.list_profiles();
            
            let mut selected = app.selected_profile.clone();
            let combobox_hint = if app.current_config.langue == "Français" { "Sélectionner un profil..." } else { "Select a profile..." };
            egui::ComboBox::from_id_salt("profile_mgr_config_select_only")
                .selected_text(if selected.is_empty() { combobox_hint } else { &selected })
                .show_ui(ui, |ui| {
                    for p in &profiles {
                        ui.selectable_value(&mut selected, p.clone(), p);
                    }
                });
            if selected != app.selected_profile {
                app.selected_profile = selected.clone();
                if !selected.is_empty() {
                    if let Some(cfg) = app.config_repo.load_profile(&selected) {
                        app.current_config = cfg.clone();
                        app.last_saved_config = cfg;
                        app.bus.emit(EventType::Log(format!("Profile '{}' loaded", selected)));
                    }
                }
            }
        });
    });
}
