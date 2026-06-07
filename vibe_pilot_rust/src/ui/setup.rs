use crate::app::VibePilotApp;
use crate::event_bus::EventType;
use crate::ui::components::play_system_beep;
use eframe::egui;

// ============================================================
// TAB 4: Setup (Storage, preferences, and reset options)
// ============================================================
pub fn render_setup_tab(ui: &mut egui::Ui, app: &mut VibePilotApp) {
    ui.group(|ui| {
        ui.label(egui::RichText::new("Application Data Location").strong());
        ui.separator();

        ui.label("By default, configuration files are saved in the current folder where the binary is located.");
        ui.label("You can change this path to store configurations, presets, and profiles elsewhere.");

        ui.add_space(8.0);

        ui.horizontal(|ui| {
            ui.label("Data Directory:");
            ui.text_edit_singleline(&mut app.custom_storage_dir);
            if ui.button("📁 Browse...").clicked() {
                if let Some(path) = rfd::FileDialog::new().pick_folder() {
                    app.custom_storage_dir = path.to_string_lossy().to_string();
                }
            }
        });

        ui.add_space(8.0);

        ui.horizontal(|ui| {
            if ui.add(egui::Button::new(egui::RichText::new("💾 Apply & Migrate Storage Path").color(egui::Color32::WHITE).strong()).fill(egui::Color32::from_rgb(16, 110, 190))).clicked() {
                let path_str = app.custom_storage_dir.trim().to_string();
                if !path_str.is_empty() {
                    match app.migrate_storage(path_str) {
                        Ok(_) => {
                            app.storage_status_message = "Storage migrated successfully!".to_string();
                            app.bus.emit(EventType::Log("Storage path migrated successfully.".to_string()));
                        }
                        Err(e) => {
                            app.storage_status_message = format!("Error migrating storage: {}", e);
                            app.bus.emit(EventType::Log(format!("Error: failed to migrate storage: {}", e)));
                        }
                    }
                }
            }
        });

        if !app.storage_status_message.is_empty() {
            ui.add_space(6.0);
            let is_error = app.storage_status_message.starts_with("Error");
            let text_color = if is_error {
                egui::Color32::from_rgb(255, 69, 0)
            } else {
                egui::Color32::from_rgb(50, 205, 50)
            };
            ui.colored_label(text_color, &app.storage_status_message);
        }
    });

    ui.add_space(12.0);

    // --- Preferences & Theme ---
    ui.group(|ui| {
        ui.label(egui::RichText::new("Preferences & Theme").strong());
        ui.separator();

        let son = app.current_config.activer_son;
        let tooltips = app.current_config.activer_tooltips;
        let auto_val = app.current_config.auto_validate;
        let auto_val_dang = app.current_config.auto_validate_dangerous;
        let theme = app.current_config.theme_sombre;
        let langue = app.current_config.langue.clone();
        let verify_cursor = app.current_config.verifier_placement_souris;
        let detect_activity = app.current_config.detecter_activite_utilisateur;

        let mut c_son = son;
        let mut c_tooltips = tooltips;
        let mut c_auto_val = auto_val;
        let mut c_auto_val_dang = auto_val_dang;
        let mut c_theme = theme;
        let mut c_langue = langue.clone();
        let mut c_verify_cursor = verify_cursor;
        let mut c_detect_activity = detect_activity;

        ui.vertical(|ui| {
            if ui.checkbox(&mut c_son, "Sound Notification").changed() {
                play_system_beep();
            }
            ui.add_space(4.0);
            ui.checkbox(&mut c_tooltips, "Active Tooltips");
            ui.add_space(4.0);
            ui.checkbox(&mut c_auto_val, "Auto-validate Standard Actions");
            ui.add_space(4.0);
            ui.checkbox(&mut c_auto_val_dang, "Auto-validate Dangerous Commands");
            ui.add_space(4.0);
            ui.checkbox(&mut c_verify_cursor, app.t("chk_verify_cursor"));
            ui.add_space(4.0);
            ui.checkbox(&mut c_detect_activity, app.t("chk_detect_activity"));
            ui.add_space(4.0);
            ui.checkbox(&mut c_theme, "Dark Theme");

            ui.add_space(10.0);

            // Zoom Factor
            ui.horizontal(|ui| {
                let zoom_lbl = if app.current_config.langue == "Français" { "Facteur de Zoom :" } else { "Zoom Factor:" };
                ui.label(zoom_lbl).on_hover_text(app.t("tip_zoom_facteur"));
                let zooms = vec![
                    ("Auto", None),
                    ("100%", Some(1.0)),
                    ("125%", Some(1.25)),
                    ("150%", Some(1.5)),
                    ("175%", Some(1.75)),
                    ("200%", Some(2.0)),
                ];
                let current_zoom = app.current_config.zoom_facteur;
                let selected_label = match current_zoom {
                    None => "Auto",
                    Some(z) if (z - 1.0).abs() < 0.01 => "100%",
                    Some(z) if (z - 1.25).abs() < 0.01 => "125%",
                    Some(z) if (z - 1.5).abs() < 0.01 => "150%",
                    Some(z) if (z - 1.75).abs() < 0.01 => "175%",
                    Some(z) if (z - 2.0).abs() < 0.01 => "200%",
                    _ => "Custom",
                };
                let mut new_zoom = current_zoom;
                egui::ComboBox::from_id_salt("zoom_combo_setup")
                    .width(100.0)
                    .selected_text(selected_label)
                    .show_ui(ui, |ui| {
                        for (label, val) in &zooms {
                            ui.selectable_value(&mut new_zoom, *val, *label);
                        }
                    })
                    .response
                    .on_hover_text(app.t("tip_zoom_facteur"));
                if new_zoom != current_zoom {
                    app.current_config.zoom_facteur = new_zoom;
                    if let Some(zoom) = new_zoom {
                        ui.ctx().set_pixels_per_point(zoom);
                    } else {
                        ui.ctx().set_pixels_per_point(1.0);
                    }
                }
            });

            ui.add_space(8.0);

            // Language
            ui.horizontal(|ui| {
                ui.label("Language:");
                let langs = vec!["English", "Français"];
                let mut lang_sel = c_langue.clone();
                egui::ComboBox::from_id_salt("lang_combo_setup")
                    .width(120.0)
                    .selected_text(&c_langue)
                    .show_ui(ui, |ui| {
                        for l in &langs {
                            ui.selectable_value(&mut lang_sel, l.to_string(), l.to_string());
                        }
                    });
                if lang_sel != c_langue {
                    c_langue = lang_sel;
                }
            });

            ui.add_space(12.0);

            // Save Global Configuration Button
            let btn_label = if app.current_config.langue == "Français" { "💾 Sauvegarder la configuration globale" } else { "💾 Save Global Configuration" };
            if ui.button(btn_label).clicked() {
                // Immediate writeback
                app.current_config.activer_son = c_son;
                app.current_config.activer_tooltips = c_tooltips;
                app.current_config.auto_validate = c_auto_val;
                app.current_config.auto_validate_dangerous = c_auto_val_dang;
                app.current_config.verifier_placement_souris = c_verify_cursor;
                app.current_config.detecter_activite_utilisateur = c_detect_activity;
                app.current_config.theme_sombre = c_theme;
                app.current_config.langue = c_langue.clone();

                app.config_repo.save_config(&app.current_config);
                app.last_saved_config = app.current_config.clone();
                let log_msg = if app.current_config.langue == "Français" { "💾 Configuration globale sauvegardée !" } else { "💾 Global configuration saved!" };
                app.bus.emit(crate::event_bus::EventType::Log(log_msg.to_string()));
            }
        });

        app.current_config.activer_son = c_son;
        app.current_config.activer_tooltips = c_tooltips;
        app.current_config.auto_validate = c_auto_val;
        app.current_config.auto_validate_dangerous = c_auto_val_dang;
        app.current_config.verifier_placement_souris = c_verify_cursor;
        app.current_config.detecter_activite_utilisateur = c_detect_activity;
        app.current_config.theme_sombre = c_theme;
        app.current_config.langue = c_langue;
    });

    ui.add_space(12.0);

    // --- Import / Export ---
    ui.group(|ui| {
        ui.label(egui::RichText::new("Import / Export").strong());
        ui.separator();

        ui.horizontal(|ui| {
            ui.label(if app.current_config.langue == "Français" { "Profil à exporter :" } else { "Profile to export:" });
            let profiles = app.config_repo.list_profiles();
            if app.selected_profile_to_export.is_empty() && !profiles.is_empty() {
                app.selected_profile_to_export = profiles[0].clone();
            }
            egui::ComboBox::from_id_salt("profile_export_combo")
                .selected_text(&app.selected_profile_to_export)
                .show_ui(ui, |ui| {
                    for p in &profiles {
                        ui.selectable_value(&mut app.selected_profile_to_export, p.clone(), p);
                    }
                });

            let btn_export_label = if app.current_config.langue == "Français" { "📤 Exporter le profil" } else { "📤 Export Profile" };
            if ui.button(btn_export_label).clicked() {
                if !app.selected_profile_to_export.is_empty() {
                    let default_filename = format!("{}.json", app.selected_profile_to_export);
                    if let Some(path) = rfd::FileDialog::new()
                        .set_file_name(&default_filename)
                        .add_filter("JSON", &["json"])
                        .save_file()
                    {
                        match app.config_repo.export_single_profile(&app.selected_profile_to_export, &path) {
                            Ok(()) => {
                                app.storage_status_message = if app.current_config.langue == "Français" { "Profil exporté avec succès !" } else { "Profile exported successfully!" }.to_string();
                                app.bus.emit(EventType::Log(format!("Profile '{}' exported.", app.selected_profile_to_export)));
                            }
                            Err(e) => {
                                app.storage_status_message = format!("Export error: {}", e);
                                app.bus.emit(EventType::Log(format!("Export error: {}", e)));
                            }
                        }
                    }
                }
            }

            let btn_import_label = if app.current_config.langue == "Français" { "📥 Importer un profil" } else { "📥 Import Profile" };
            if ui.button(btn_import_label).clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("JSON", &["json"])
                    .pick_file()
                {
                    match app.config_repo.import_single_profile(&path) {
                        Ok(imported_name) => {
                            app.storage_status_message = if app.current_config.langue == "Français" {
                                format!("Profil '{}' importé !", imported_name)
                            } else {
                                format!("Profile '{}' imported!", imported_name)
                            };
                            app.bus.emit(EventType::Log(format!("Profile '{}' imported.", imported_name)));
                            app.selected_profile = imported_name;
                        }
                        Err(e) => {
                            app.storage_status_message = format!("Import error: {}", e);
                            app.bus.emit(EventType::Log(format!("Import error: {}", e)));
                        }
                    }
                }
            }
        });
        ui.add_space(4.0);
        
        ui.horizontal(|ui| {
            ui.label(if app.current_config.langue == "Français" { "Moteur à exporter :" } else { "Engine to export:" });
            let engines = app.engine_presets.all_keys();
            if app.selected_engine_to_export.is_empty() && !engines.is_empty() {
                app.selected_engine_to_export = engines[0].clone();
            }
            egui::ComboBox::from_id_salt("engine_export_combo")
                .selected_text(&app.selected_engine_to_export)
                .show_ui(ui, |ui| {
                    for e in &engines {
                        ui.selectable_value(&mut app.selected_engine_to_export, e.clone(), e);
                    }
                });

            let btn_export_label = if app.current_config.langue == "Français" { "📤 Exporter le moteur" } else { "📤 Export Engine" };
            if ui.button(btn_export_label).clicked() {
                if !app.selected_engine_to_export.is_empty() {
                    let default_filename = format!("{}.json", app.selected_engine_to_export);
                    if let Some(path) = rfd::FileDialog::new()
                        .set_file_name(&default_filename)
                        .add_filter("JSON", &["json"])
                        .save_file()
                    {
                        match app.config_repo.export_single_engine(&app.selected_engine_to_export, &path) {
                            Ok(()) => {
                                app.storage_status_message = if app.current_config.langue == "Français" { "Moteur exporté avec succès !" } else { "Engine exported successfully!" }.to_string();
                                app.bus.emit(EventType::Log(format!("Engine '{}' exported.", app.selected_engine_to_export)));
                            }
                            Err(e) => {
                                app.storage_status_message = format!("Export error: {}", e);
                                app.bus.emit(EventType::Log(format!("Export error: {}", e)));
                            }
                        }
                    }
                }
            }

            let btn_import_label = if app.current_config.langue == "Français" { "📥 Importer un moteur" } else { "📥 Import Engine" };
            if ui.button(btn_import_label).clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("JSON", &["json"])
                    .pick_file()
                {
                    match app.config_repo.import_single_engine(&path) {
                        Ok(imported_name) => {
                            app.storage_status_message = if app.current_config.langue == "Français" {
                                format!("Moteur '{}' importé !", imported_name)
                            } else {
                                format!("Engine '{}' imported!", imported_name)
                            };
                            app.bus.emit(EventType::Log(format!("Engine '{}' imported.", imported_name)));
                            app.engine_presets = app.config_repo.load_engines();
                            app.selected_engine_to_export = imported_name;
                        }
                        Err(e) => {
                            app.storage_status_message = format!("Import error: {}", e);
                            app.bus.emit(EventType::Log(format!("Import error: {}", e)));
                        }
                    }
                }
            }
        });

        ui.add_space(4.0);

        ui.horizontal(|ui| {
            let btn_export_cfg_label = if app.current_config.langue == "Français" { "📤 Exporter le Setup" } else { "📤 Export Setup" };
            if ui.button(btn_export_cfg_label).clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .set_file_name("vibepilot-setup.json")
                    .add_filter("JSON", &["json"])
                    .save_file()
                {
                    match app.config_repo.export_config(&path) {
                        Ok(()) => {
                            app.storage_status_message = if app.current_config.langue == "Français" { "Setup exporté avec succès !" } else { "Setup exported successfully!" }.to_string();
                            app.bus.emit(EventType::Log("Setup exported.".to_string()));
                        }
                        Err(e) => {
                            app.storage_status_message = format!("Export error: {}", e);
                            app.bus.emit(EventType::Log(format!("Export error: {}", e)));
                        }
                    }
                }
            }

            let btn_import_cfg_label = if app.current_config.langue == "Français" { "📥 Importer un Setup" } else { "📥 Import Setup" };
            if ui.button(btn_import_cfg_label).clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("JSON", &["json"])
                    .pick_file()
                {
                    match app.config_repo.import_config(&path) {
                        Ok(()) => {
                            app.storage_status_message = if app.current_config.langue == "Français" { "Setup importé avec succès ! (Un redémarrage peut être nécessaire)" } else { "Setup imported successfully! (Restart may be required)" }.to_string();
                            app.bus.emit(EventType::Log("Setup imported.".to_string()));
                        }
                        Err(e) => {
                            app.storage_status_message = format!("Import error: {}", e);
                            app.bus.emit(EventType::Log(format!("Import error: {}", e)));
                        }
                    }
                }
            }
        });
    });

    ui.add_space(12.0);

    // --- Resumption & Reset ---
    ui.group(|ui| {
        ui.label(egui::RichText::new("Resumption Checkpoint & System Reset").strong());
        ui.separator();

        ui.label("If the application was interrupted, a resumption prompt may be stored here to help resume execution.");

        let mut reprise_text = app.current_config.prompt_reprise.clone().unwrap_or_default();
        ui.horizontal(|ui| {
            ui.label("Resume Prompt:");
            let text_edit = egui::TextEdit::singleline(&mut reprise_text)
                .hint_text("No resumption prompt active...")
                .desired_width(ui.available_width() - 150.0);
            ui.add(text_edit);
        });

        if reprise_text != app.current_config.prompt_reprise.clone().unwrap_or_default() {
            app.current_config.prompt_reprise = if reprise_text.trim().is_empty() {
                None
            } else {
                Some(reprise_text)
            };
        }

        ui.add_space(8.0);

        ui.horizontal(|ui| {
            if ui.button("Clear Resume Prompt").clicked() {
                app.current_config.prompt_reprise = None;
                app.bus.emit(EventType::Log("Resumption prompt cleared.".to_string()));
            }

            ui.add_space(16.0);

            if ui.add(egui::Button::new(egui::RichText::new("⚠️ Reset All Settings to Default").color(egui::Color32::WHITE).strong()).fill(egui::Color32::from_rgb(175, 45, 45))).clicked() {
                app.show_reset_confirm = true;
            }
        });
    });
}
