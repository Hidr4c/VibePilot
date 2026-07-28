use crate::app::VibePilotApp;
use crate::event_bus::NotificationEvent;
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
                            app.bus.emit_notification(NotificationEvent::Log("Storage path migrated successfully.".to_string()));
                        }
                        Err(e) => {
                            app.storage_status_message = format!("Error migrating storage: {}", e);
                            app.bus.emit_notification(NotificationEvent::Log(format!("Error: failed to migrate storage: {}", e)));
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
        let ssd_savings = app.current_config.economie_ecriture_ssd;
        let keep_screen_awake = app.current_config.garder_ecran_actif;
        let capture_folder = app.current_config.dossier_sauvegarde_captures.clone().unwrap_or_default();
        let chiffrement_dpapi = app.current_config.chiffrement_dpapi;

        let mut c_son = son;
        let mut c_tooltips = tooltips;
        let mut c_auto_val = auto_val;
        let mut c_auto_val_dang = auto_val_dang;
        let mut c_theme = theme;
        let mut c_langue = langue.clone();
        let mut c_verify_cursor = verify_cursor;
        let mut c_ssd_savings = ssd_savings;
        let mut c_keep_screen_active = keep_screen_awake;
        let mut c_capture_folder = capture_folder;
        let mut c_chiffrement_dpapi = chiffrement_dpapi;

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
            ui.checkbox(&mut c_ssd_savings, app.t("chk_ssd_savings"));
            ui.add_space(4.0);

            if !c_ssd_savings {
                ui.indent("ssd_savings_indent", |ui| {
                    ui.horizontal(|ui| {
                        ui.label(app.t("lbl_capture_save_folder")).on_hover_text(app.t("tip_capture_save_folder"));
                        ui.add(
                            egui::TextEdit::singleline(&mut c_capture_folder)
                                .desired_width(180.0)
                        ).on_hover_text(app.t("tip_capture_save_folder"));
                        
                        if ui.button(app.t("btn_choose_folder")).clicked() {
                            if let Some(path) = rfd::FileDialog::new().pick_folder() {
                                c_capture_folder = path.to_string_lossy().into_owned();
                            }
                        }
                    });
                });
                ui.add_space(4.0);
            }
            ui.checkbox(&mut c_keep_screen_active, app.t("chk_keep_screen_awake"));
            ui.add_space(4.0);
            
            #[cfg(target_os = "windows")]
            {
                ui.checkbox(&mut c_chiffrement_dpapi, "🔒 Windows hardware encryption (DPAPI)")
                    .on_hover_text("Encrypts credentials using your Windows User Account (DPAPI).");
            }
            #[cfg(not(target_os = "windows"))]
            {
                let mut fake = false;
                ui.add_enabled_ui(false, |ui| {
                    ui.checkbox(&mut fake, "🔒 Windows hardware encryption (DPAPI) (Windows only)")
                        .on_hover_text("DPAPI is only supported on Windows targets.");
                });
            }
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
                app.current_config.economie_ecriture_ssd = c_ssd_savings;
                app.current_config.dossier_sauvegarde_captures = if c_capture_folder.trim().is_empty() {
                    None
                } else {
                    Some(c_capture_folder.clone())
                };
                app.current_config.garder_ecran_actif = c_keep_screen_active;
                app.current_config.theme_sombre = c_theme;
                app.current_config.langue = c_langue.clone();
                app.current_config.chiffrement_dpapi = c_chiffrement_dpapi;

                app.config_repo.save_config(&app.current_config);
                app.last_saved_config = app.current_config.clone();
                let log_msg = if app.current_config.langue == "Français" { "💾 Configuration globale sauvegardée !" } else { "💾 Global configuration saved!" };
                app.bus.emit_notification(crate::event_bus::NotificationEvent::Log(log_msg.to_string()));
                app.show_save_success_popup = Some("global_config".to_string());
            }
        });

        app.current_config.activer_son = c_son;
        app.current_config.activer_tooltips = c_tooltips;
        app.current_config.auto_validate = c_auto_val;
        app.current_config.auto_validate_dangerous = c_auto_val_dang;
        app.current_config.verifier_placement_souris = c_verify_cursor;
        app.current_config.economie_ecriture_ssd = c_ssd_savings;
        app.current_config.garder_ecran_actif = c_keep_screen_active;
        app.current_config.theme_sombre = c_theme;
        app.current_config.langue = c_langue;
        app.current_config.chiffrement_dpapi = c_chiffrement_dpapi;
    });

    ui.add_space(12.0);

    // --- Import / Export ---
    crate::ui::setup_import_export::render_import_export_group(ui, app);

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
                app.bus.emit_notification(NotificationEvent::Log("Resumption prompt cleared.".to_string()));
            }

            ui.add_space(16.0);

            if ui.add(egui::Button::new(egui::RichText::new("⚠️ Reset All Settings to Default").color(egui::Color32::WHITE).strong()).fill(egui::Color32::from_rgb(175, 45, 45))).clicked() {
                app.show_reset_confirm = true;
            }
        });
    });
}


