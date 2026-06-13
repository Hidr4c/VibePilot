use crate::app::VibePilotApp;
use crate::event_bus::NotificationEvent;
use eframe::egui;

pub fn render_import_export_group(ui: &mut egui::Ui, app: &mut VibePilotApp) {
    ui.group(|ui| {
        ui.label(egui::RichText::new("Import / Export").strong());
        ui.separator();

        ui.horizontal(|ui| {
            ui.label(if app.current_config.langue == "Français" { "Profil à exporter :" } else { "Profile to export:" });
            let mut profiles = app.config_repo.list_profiles();
            profiles.sort_by(|a, b| a.to_lowercase().cmp(&b.to_lowercase()));
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
                                app.bus.emit_notification(NotificationEvent::Log(format!("Profile '{}' exported.", app.selected_profile_to_export)));
                            }
                            Err(e) => {
                                app.storage_status_message = format!("Export error: {}", e);
                                app.bus.emit_notification(NotificationEvent::Log(format!("Export error: {}", e)));
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
                            app.bus.emit_notification(NotificationEvent::Log(format!("Profile '{}' imported.", imported_name)));
                            app.selected_profile = imported_name;
                        }
                        Err(e) => {
                            app.storage_status_message = format!("Import error: {}", e);
                            app.bus.emit_notification(NotificationEvent::Log(format!("Import error: {}", e)));
                        }
                    }
                }
            }
        });
        ui.add_space(4.0);
        
        ui.horizontal(|ui| {
            ui.label(if app.current_config.langue == "Français" { "Moteur à exporter :" } else { "Engine to export:" });
            let mut engines = app.engine_presets.all_keys();
            engines.sort_by(|a, b| a.to_lowercase().cmp(&b.to_lowercase()));
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
                                app.bus.emit_notification(NotificationEvent::Log(format!("Engine '{}' exported.", app.selected_engine_to_export)));
                            }
                            Err(e) => {
                                app.storage_status_message = format!("Export error: {}", e);
                                app.bus.emit_notification(NotificationEvent::Log(format!("Export error: {}", e)));
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
                            app.bus.emit_notification(NotificationEvent::Log(format!("Engine '{}' imported.", imported_name)));
                            app.engine_presets = app.config_repo.load_engines();
                            app.selected_engine_to_export = imported_name;
                        }
                        Err(e) => {
                            app.storage_status_message = format!("Import error: {}", e);
                            app.bus.emit_notification(NotificationEvent::Log(format!("Import error: {}", e)));
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
                            app.bus.emit_notification(NotificationEvent::Log("Setup exported.".to_string()));
                        }
                        Err(e) => {
                            app.storage_status_message = format!("Export error: {}", e);
                            app.bus.emit_notification(NotificationEvent::Log(format!("Export error: {}", e)));
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
                            app.bus.emit_notification(NotificationEvent::Log("Setup imported.".to_string()));
                        }
                        Err(e) => {
                            app.storage_status_message = format!("Import error: {}", e);
                            app.bus.emit_notification(NotificationEvent::Log(format!("Import error: {}", e)));
                        }
                    }
                }
            }

            let btn_export_journal_label = if app.current_config.langue == "Français" { "🔓 Exporter le Journal Décrypté" } else { "🔓 Export Decrypted Journal" };
            if ui.button(btn_export_journal_label).clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .set_file_name("vibepilot-journal-decrypted.json")
                    .add_filter("JSON", &["json"])
                    .save_file()
                {
                    let encrypted_path = app.config_repo.get_store_path();
                    match crate::config::export_decrypted_journal(&encrypted_path, &path) {
                        Ok(()) => {
                            app.storage_status_message = if app.current_config.langue == "Français" { "Journal décrypté exporté avec succès !" } else { "Decrypted journal exported successfully!" }.to_string();
                            app.bus.emit_notification(NotificationEvent::Log("Decrypted journal exported.".to_string()));
                        }
                        Err(e) => {
                            app.storage_status_message = format!("Export error: {}", e);
                            app.bus.emit_notification(NotificationEvent::Log(format!("Export error: {}", e)));
                        }
                    }
                }
            }
        });
    });
}
