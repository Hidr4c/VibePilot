use crate::app::VibePilotApp;
use crate::event_bus::NotificationEvent;
use eframe::egui;

// ============================================================
// TAB 1: Global Configuration
// ============================================================
pub fn render_config_tab(ui: &mut egui::Ui, app: &mut VibePilotApp) {
    let available_width = ui.available_width();

    // --- Section 1: AI Engine & Model ---
    crate::ui::engine_config::render_engine_config_section(ui, app);

    ui.add_space(12.0);


    // --- Section 1.5: Advanced Intelligence & Guidance ---
    ui.group(|ui| {
        ui.horizontal(|ui| {
            ui.label(if app.current_config.langue == "Français" { "🧠 Intelligence & Guidage Avancé" } else { "🧠 Advanced Intelligence & Guidance" });
        });
        ui.separator();
        
        ui.horizontal(|ui| {
            ui.checkbox(
                &mut app.current_config.decomposer_taches,
                if app.current_config.langue == "Français" {
                    "Décomposer l'objectif en graphe de sous-tâches (TaskGraph)"
                } else {
                    "Decompose objective into a task graph (TaskGraph)"
                }
            );
            
            ui.add_space(16.0);
            
            ui.checkbox(
                &mut app.current_config.activer_reflexion,
                if app.current_config.langue == "Français" {
                    "Activer la réflexion visuelle post-action (Reflection)"
                } else {
                    "Enable post-action visual reflection (Reflection)"
                }
            );
        });
        
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.checkbox(
                &mut app.current_config.pipeline_vision_avance,
                if app.current_config.langue == "Français" {
                    "Activer le zoom dynamique sur zone d'intérêt (Zoom VLM)"
                } else {
                    "Enable dynamic zoom on region of interest (Zoom VLM)"
                }
            );
        });
        
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.checkbox(
                &mut app.current_config.activer_systeme_fast_slow,
                if app.current_config.langue == "Français" {
                    "Activer le routage d'actions rapides (Fast/Slow)"
                } else {
                    "Enable fast/slow action routing (Fast/Slow)"
                }
            );
            
            ui.add_space(16.0);
            
            ui.checkbox(
                &mut app.current_config.activer_compression_historique,
                if app.current_config.langue == "Français" {
                    "Activer la compression d'historique de contexte"
                } else {
                    "Enable context history compression"
                }
            );
        });
        
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.checkbox(
                &mut app.current_config.economie_ecriture_ssd,
                if app.current_config.langue == "Français" {
                    "Économie d'écriture SSD (Vacation de 5 min)"
                } else {
                    "SSD Write Protection (5-min vacation cache)"
                }
            );

            ui.add_space(16.0);

            ui.checkbox(
                &mut app.current_config.activer_recadrage_workspace,
                if app.current_config.langue == "Français" {
                    "Activer le recadrage adaptatif de la zone de travail"
                } else {
                    "Enable adaptive workspace cropping"
                }
            );
        });
        
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.checkbox(
                &mut app.current_config.activer_roi,
                if app.current_config.langue == "Français" {
                    "Activer la zone d'intérêt (ROI) fixe"
                } else {
                    "Enable fixed Region of Interest (ROI)"
                }
            );
        });

        if app.current_config.activer_roi {
            ui.indent("roi_settings_indent", |ui| {
                ui.horizontal(|ui| {
                    ui.label("X:");
                    ui.add(egui::DragValue::new(&mut app.current_config.roi_x).range(0..=9999));
                    ui.add_space(8.0);
                    ui.label("Y:");
                    ui.add(egui::DragValue::new(&mut app.current_config.roi_y).range(0..=9999));
                    ui.add_space(8.0);
                    ui.label("W:");
                    ui.add(egui::DragValue::new(&mut app.current_config.roi_width).range(1..=9999));
                    ui.add_space(8.0);
                    ui.label("H:");
                    ui.add(egui::DragValue::new(&mut app.current_config.roi_height).range(1..=9999));
                });
            });
        }
        
        if app.current_config.decomposer_taches {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(if app.current_config.langue == "Français" { "Tentatives max par tâche :" } else { "Max attempts per sub-task:" });
                ui.add(egui::Slider::new(&mut app.current_config.max_tentatives_par_tache, 1..=10));
            });
        }
    });

    ui.add_space(12.0);

    // --- Section 2: Target Applications ---
    ui.group(|ui| {
        ui.horizontal(|ui| {
            ui.label(app.t("frame_windows")).on_hover_text(app.t("tip_frame_windows"));
        });
        ui.separator();

        ui.horizontal(|ui| {
            let mut all_windows = app.orchestrator.get_available_windows();
            all_windows.sort_by(|a, b| a.to_lowercase().cmp(&b.to_lowercase()));
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

            let refresh_label = if app.current_config.langue == "Français" { "🔄 Rafraîchir" } else { "🔄 Refresh List" };
            if ui.button(refresh_label).on_hover_text(app.t("tip_refresh")).clicked() {
                app.orchestrator.refresh_windows();
                app.bus.emit_notification(NotificationEvent::Log("Refreshing window list...".to_string()));
            }

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
                windows.sort_by(|a, b| a.to_lowercase().cmp(&b.to_lowercase()));
                app.current_config.fenetres_surveillees = windows;
                app.bus.emit_notification(NotificationEvent::Log(format!("Window added: {}", win)));
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
            windows_clone.sort_by(|a, b| a.to_lowercase().cmp(&b.to_lowercase()));
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
            let mut profiles = app.config_repo.list_profiles();
            profiles.sort_by(|a, b| a.to_lowercase().cmp(&b.to_lowercase()));
            
            let mut current_profile_name = app.selected_profile.clone();
            let mut selected_val = current_profile_name.clone();
            
            // Text box for typing/creating a new profile name or displaying the loaded one
            let response = ui.add(egui::TextEdit::singleline(&mut current_profile_name).desired_width(180.0));
            
            // Dropdown combobox next to it to select from existing profiles
            egui::ComboBox::from_id_salt("profile_combo_suggestions")
                .width(16.0)
                .selected_text("")
                .show_ui(ui, |ui| {
                    for p in &profiles {
                        ui.selectable_value(&mut selected_val, p.clone(), p);
                    }
                });
            
            // Save Button (saves current configuration to the selected profile)
            let btn_save_label = if app.current_config.langue == "Français" { "💾 Sauver" } else { "💾 Save" };
            if ui.button(btn_save_label).on_hover_text(app.t("tip_save_profile")).clicked()
                && !app.selected_profile.is_empty()
            {
                app.current_config.dernier_profil = app.selected_profile.clone();
                if app.config_repo.save_profile(&app.selected_profile, &app.current_config) {
                    app.last_saved_config = app.current_config.clone();
                    app.bus.emit_notification(NotificationEvent::Log(format!("Profile '{}' saved", app.selected_profile)));
                    app.show_save_success_popup = Some(app.selected_profile.clone());
                }
            }
            
            if selected_val != app.selected_profile && !selected_val.is_empty() {
                // User selected an existing profile from the dropdown
                app.load_profile(&selected_val);
            } else if response.changed() {
                // User typed something
                let trimmed = current_profile_name.trim().to_string();
                if profiles.contains(&trimmed) {
                    app.load_profile(&trimmed);
                } else {
                    app.selected_profile = trimmed.clone();
                    app.quick_start_profile_name = trimmed;
                }
            }
        });
    });

    ui.add_space(12.0);

    // --- Section 4: AI Prompt Generator ---
    ui.group(|ui| {
        ui.horizontal(|ui| {
            ui.label(app.t("frame_generateur")).on_hover_text(app.t("tip_generate_all"));
        });
        ui.separator();

        let hint_gen_desc = if app.current_config.langue == "Français" {
            "Saisissez une simple demande et l'IA va générer la configuration complète :"
        } else {
            "Enter a simple request and AI will generate the full configuration:"
        };
        ui.label(hint_gen_desc);
        ui.add_space(6.0);
        let hint_gen_ex = if app.current_config.langue == "Français" {
            "Exemple: 'Automatiser les tests d'extension VS Code pour atteindre 80% de couverture'"
        } else {
            "Example: 'Automate testing VS Code extension to reach 80% coverage'"
        };
        
        let text_height = 80.0;
        ui.add(egui::TextEdit::multiline(&mut app.current_config.demande_generique)
            .hint_text(hint_gen_ex)
            .desired_width(available_width)
            .min_size(egui::vec2(available_width, text_height)));

        ui.add_space(8.0);
        ui.horizontal(|ui| {
            let btn = egui::Button::new(egui::RichText::new(app.t("btn_generer_tout")).color(egui::Color32::WHITE).strong())
                .fill(egui::Color32::from_rgb(106, 27, 154));

            ui.add_enabled_ui(!app.is_generating_prompts, |ui| {
                if ui.add(btn).on_hover_text(app.t("tip_generate_all")).clicked() {
                    app.generate_prompts_from_request();
                }
            });

            if app.is_generating_prompts {
                ui.spinner();
                let generating_lbl = if app.current_config.langue == "Français" {
                    "Génération en cours..."
                } else {
                    "Generating..."
                };
                ui.label(generating_lbl);
            }
        });
    });
}
