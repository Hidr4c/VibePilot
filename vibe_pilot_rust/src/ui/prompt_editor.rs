use crate::app::VibePilotApp;
use crate::event_bus::EventType;
use eframe::egui;

fn is_profile_modified(app: &VibePilotApp) -> bool {
    app.current_config.contexte != app.last_saved_config.contexte
        || app.current_config.objectif != app.last_saved_config.objectif
        || app.current_config.task != app.last_saved_config.task
        || app.current_config.directives != app.last_saved_config.directives
}

fn render_editor_profile_manager(ui: &mut egui::Ui, app: &mut VibePilotApp) {
    ui.horizontal(|ui| {
        ui.label(app.t("lbl_profil")).on_hover_text(app.t("tip_frame_profils"));
        let profiles = app.config_repo.list_profiles();
        
        // Combobox for choosing existing profiles (auto-loads on selection)
        let mut selected = app.selected_profile.clone();
        let combobox_hint = if app.current_config.langue == "Français" { "Sélectionner un profil..." } else { "Select a profile..." };
        egui::ComboBox::from_id_salt("profile_mgr_editor_select")
            .selected_text(if selected.is_empty() { combobox_hint } else { &selected })
            .show_ui(ui, |ui| {
                for p in &profiles {
                    ui.selectable_value(&mut selected, p.clone(), p);
                }
            });
        if selected != app.selected_profile {
            if !selected.is_empty() {
                app.load_profile(&selected);
            }
        }

        // Textbox to edit/type the name directly
        let textedit_hint = if app.current_config.langue == "Français" { "Nouveau nom de profil..." } else { "New profile name..." };
        ui.add(egui::TextEdit::singleline(&mut app.selected_profile).hint_text(textedit_hint));

        // Save Button (saves current prompts to the selected profile)
        let btn_save_label = if app.current_config.langue == "Français" { "💾 Sauver" } else { "💾 Save" };
        if ui.button(btn_save_label).on_hover_text(app.t("tip_save_profile")).clicked() {
            if !app.selected_profile.is_empty() {
                app.current_config.dernier_profil = app.selected_profile.clone();
                if app.config_repo.save_profile(&app.selected_profile, &app.current_config) {
                    app.last_saved_config = app.current_config.clone();
                    app.bus.emit(EventType::Log(format!("Profile '{}' saved", app.selected_profile)));
                }
            }
        }

        // Delete Button
        if ui.button(app.t("btn_supprimer")).on_hover_text(app.t("tip_del_profile")).clicked() {
            if !app.selected_profile.is_empty() {
                app.show_delete_confirm = Some(app.selected_profile.clone());
            }
        }

        // Rename Button
        let btn_rename_label = if app.current_config.langue == "Français" { "✏️ Renommer" } else { "✏️ Rename" };
        let tip_rename_profile = if app.current_config.langue == "Français" { "Renommer le profil sélectionné" } else { "Rename the selected profile" };
        if ui.button(btn_rename_label).on_hover_text(tip_rename_profile).clicked() {
            if !app.selected_profile.is_empty() {
                app.show_rename_profile = Some(app.selected_profile.clone());
                app.rename_profile_new_name = app.selected_profile.clone();
            }
        }

        // Refresh/Reload Button
        let btn_refresh_label = if app.current_config.langue == "Français" { "🔄 Recharger" } else { "🔄 Reload" };
        let tip_refresh_profile = if app.current_config.langue == "Français" { "Annuler les modifications et recharger le profil depuis le disque" } else { "Discard changes and reload the profile from disk" };
        if ui.button(btn_refresh_label).on_hover_text(tip_refresh_profile).clicked() {
            if !app.selected_profile.is_empty() {
                let current_profile = app.selected_profile.clone();
                app.load_profile(&current_profile);
            }
        }

        // Modified Indicator
        if is_profile_modified(app) {
            ui.add_space(8.0);
            let modified_text = if app.current_config.langue == "Français" { "⚠️ Modifié (non sauvegardé)" } else { "⚠️ Modified (unsaved)" };
            ui.colored_label(egui::Color32::from_rgb(255, 140, 0), modified_text);
        }
    });
}

// ============================================================
// TAB 2: Prompt Editor
// ============================================================
pub fn render_prompt_editor_tab(ui: &mut egui::Ui, app: &mut VibePilotApp) {
    // Profile Management Bar
    render_editor_profile_manager(ui, app);
    ui.separator();
    ui.add_space(8.0);

    let width = ui.available_width();

    // 0. General Situation Context
    ui.group(|ui| {
        ui.horizontal(|ui| {
            ui.label(app.t("lbl_context")).on_hover_text(app.t("tip_context"));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.add(egui::Button::new(egui::RichText::new(app.t("btn_opti_court")).color(egui::Color32::WHITE).strong()).fill(egui::Color32::from_rgb(16, 110, 190))).on_hover_text(app.t("tip_optimize_field")).clicked() {
                    app.optimize_prompt_field("contexte");
                }
            });
        });
        ui.separator();
        let hint_context = if app.current_config.langue == "Français" {
            "Décrivez la situation actuelle, l'environnement et les outils disponibles..."
        } else {
            "Describe the current situation, environment, and tools available..."
        };
        ui.add(egui::TextEdit::multiline(&mut app.current_config.contexte)
            .hint_text(hint_context)
            .desired_width(width)
            .desired_rows(3));
    });
    ui.add_space(8.0);

    // 1. Global Task
    ui.group(|ui| {
        ui.horizontal(|ui| {
            ui.label(app.t("lbl_tsk")).on_hover_text(app.t("tip_tsk"));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.add(egui::Button::new(egui::RichText::new(app.t("btn_opti_court")).color(egui::Color32::WHITE).strong()).fill(egui::Color32::from_rgb(16, 110, 190))).on_hover_text(app.t("tip_optimize_field")).clicked() {
                    app.optimize_prompt_field("task");
                }
            });
        });
        ui.separator();
        let hint_task = if app.current_config.langue == "Français" {
            "Description de la tâche principale pour l'opérateur IA..."
        } else {
            "Main task description for the AI operator..."
        };
        ui.add(egui::TextEdit::multiline(&mut app.current_config.task)
            .hint_text(hint_task)
            .desired_width(width)
            .desired_rows(3));
    });
    ui.add_space(8.0);

    // 2. Stop Condition
    ui.group(|ui| {
        ui.horizontal(|ui| {
            ui.label(app.t("lbl_obj")).on_hover_text(app.t("tip_obj"));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.add(egui::Button::new(egui::RichText::new(app.t("btn_opti_court")).color(egui::Color32::WHITE).strong()).fill(egui::Color32::from_rgb(16, 110, 190))).on_hover_text(app.t("tip_optimize_field")).clicked() {
                    app.optimize_prompt_field("objectif");
                }
            });
        });
        ui.separator();
        let hint_obj = if app.current_config.langue == "Français" {
            "Condition visuelle qui indique que la tâche est terminée..."
        } else {
            "Visual condition that indicates the task is complete..."
        };
        ui.add(egui::TextEdit::multiline(&mut app.current_config.objectif)
            .hint_text(hint_obj)
            .desired_width(width)
            .desired_rows(3));
    });
    ui.add_space(8.0);

    // 3. Specific System Directives
    ui.group(|ui| {
        ui.horizontal(|ui| {
            ui.label(app.t("lbl_dir")).on_hover_text(app.t("tip_dir"));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.add(egui::Button::new(egui::RichText::new(app.t("btn_opti_court")).color(egui::Color32::WHITE).strong()).fill(egui::Color32::from_rgb(16, 110, 190))).on_hover_text(app.t("tip_optimize_field")).clicked() {
                    app.optimize_prompt_field("directives");
                }
            });
        });
        ui.separator();
        let hint_dir = if app.current_config.langue == "Français" {
            "Règles de comportement et contraintes de sécurité pour l'IA..."
        } else {
            "Behavioral rules and safety constraints for the AI..."
        };
        ui.add(egui::TextEdit::multiline(&mut app.current_config.directives)
            .hint_text(hint_dir)
            .desired_width(width)
            .desired_rows(3));
    });
    ui.add_space(12.0);

    // Bottom buttons inside tab: Save Profile
    ui.horizontal(|ui| {
        if ui.add(egui::Button::new(egui::RichText::new(app.t("btn_sauver")).color(egui::Color32::WHITE).strong()).fill(egui::Color32::from_rgb(20, 120, 100))).on_hover_text(app.t("tip_save_profile")).clicked() {
            app.config_repo.save_config(&app.current_config);
            app.bus.emit(EventType::Log("Configuration saved".to_string()));
        }
    });
}
