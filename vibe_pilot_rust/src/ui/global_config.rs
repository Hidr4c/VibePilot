use crate::app::VibePilotApp;
use crate::event_bus::NotificationEvent;
use eframe::egui;

// ============================================================
// TAB 1: Global Configuration
// ============================================================
pub fn render_config_tab(ui: &mut egui::Ui, app: &mut VibePilotApp) {
    let available_width = ui.available_width();
    let is_fr = app.current_config.langue == "Français";

    // --- Configuration Inconsistency Alerts & Warnings ---
    let mut conflicts = Vec::new();

    let has_target_window = !app.current_config.fenetres_surveillees.is_empty()
        && !app.current_config.fenetres_surveillees[0].to_lowercase().contains("all screens")
        && !app.current_config.fenetres_surveillees[0].to_lowercase().contains("tous les")
        && !app.current_config.fenetres_surveillees[0].to_lowercase().contains("desktop")
        && !app.current_config.fenetres_surveillees[0].is_empty();

    if has_target_window && !app.current_config.activer_recadrage_workspace {
        conflicts.push(if is_fr {
            "⚠️ Une fenêtre cible est définie, mais « Recadrage Workspace » est désactivé. L'IA utilisera tout l'écran, ce qui perturbera la précision des clics."
        } else {
            "⚠️ Target window selected but 'Workspace Cropping' is disabled. AI will use the full desktop, causing click coordinate misalignment."
        });
    }

    if app.current_config.activer_roi && app.current_config.activer_recadrage_workspace {
        conflicts.push(if is_fr {
            "⚠️ Conflit : La 'ROI fixe' et le 'Recadrage Workspace' sont activés simultanément. La ROI fixe est prioritaire."
        } else {
            "⚠️ Conflict: Both 'Fixed ROI' and 'Workspace Cropping' are enabled. Fixed ROI takes precedence."
        });
    }

    if app.current_config.decomposer_taches && app.current_config.objectif.trim().is_empty() {
        conflicts.push(if is_fr {
            "⚠️ TaskGraph activé mais l'objectif principal est vide. Spécifiez une tâche dans l'Éditeur."
        } else {
            "⚠️ TaskGraph is enabled but the main objective is empty. Define your task in the Editor tab."
        });
    }

    if !conflicts.is_empty() {
        ui.group(|ui| {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.colored_label(
                        egui::Color32::from_rgb(255, 140, 0),
                        egui::RichText::new(if is_fr { "⚠️ Alertes de Configuration" } else { "⚠️ Configuration Warnings" }).strong()
                    );
                });
                ui.separator();
                for conflict in &conflicts {
                    ui.colored_label(egui::Color32::from_rgb(255, 165, 0), *conflict);
                }
            });
        });
        ui.add_space(8.0);
    }

    // --- Section 1: AI Engine & Model ---
    crate::ui::engine_config::render_engine_config_section(ui, app);

    ui.add_space(12.0);


    // --- Section 1.5: Advanced Intelligence & Guidance ---
    ui.group(|ui| {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(if is_fr { "🧠 Intelligence & Guidage Avancé" } else { "🧠 Advanced Intelligence & Guidance" }).strong());
        });
        ui.separator();
        
        // Category 1: Planification & Réflexion / Planning & Reasoning
        ui.label(egui::RichText::new(if is_fr { "🎯 Planification & Réflexion" } else { "🎯 Planning & Reasoning" }).strong().color(ui.visuals().hyperlink_color));
        ui.add_space(2.0);
        
        // 1.1 TaskGraph
        ui.checkbox(
            &mut app.current_config.decomposer_taches,
            if is_fr {
                "Décomposer l'objectif principal en graphe de tâches (TaskGraph)"
            } else {
                "Decompose main objective into a task graph (TaskGraph)"
            }
        );
        ui.indent("desc_decomposer_taches", |ui| {
            ui.weak(if is_fr {
                "Divise la demande principale en sous-tâches logiques exécutées séquentiellement et affichées sous forme de graphe."
            } else {
                "Splits the main request into logical sub-tasks executed sequentially and displayed in a task graph."
            });
            if app.current_config.decomposer_taches {
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label(if is_fr { "Tentatives max par tâche :" } else { "Max attempts per sub-task:" });
                    ui.add(egui::Slider::new(&mut app.current_config.max_tentatives_par_tache, 1..=10));
                });
            }
        });
        
        ui.add_space(6.0);
        
        // 1.2 Reflection
        ui.checkbox(
            &mut app.current_config.activer_reflexion,
            if is_fr {
                "Valider visuellement chaque action (Visual Reflection)"
            } else {
                "Visually validate each action (Visual Reflection)"
            }
        );
        ui.indent("desc_activer_reflexion", |ui| {
            ui.weak(if is_fr {
                "Prend une capture d'écran après chaque clic ou saisie pour vérifier si l'action a été correctement effectuée."
            } else {
                "Takes a screenshot after each click or input to verify if the action was executed successfully."
            });
        });

        ui.add_space(10.0);
        ui.separator();
        ui.add_space(4.0);

        // Category 2: Vision & Zone de Travail / Vision & Workspace
        ui.label(egui::RichText::new(if is_fr { "👁️ Vision & Zone de Travail" } else { "👁️ Vision & Workspace" }).strong().color(ui.visuals().hyperlink_color));
        ui.add_space(2.0);

        // 2.1 Zoom VLM
        ui.checkbox(
            &mut app.current_config.pipeline_vision_avance,
            if is_fr {
                "Zoom de précision sur la zone ciblée (Zoom VLM)"
            } else {
                "High-precision zoom on target area (Zoom VLM)"
            }
        );
        ui.indent("desc_pipeline_vision_avance", |ui| {
            ui.weak(if is_fr {
                "Agrandit temporairement la zone visée par le modèle pour l'aider à interagir avec les petits éléments."
            } else {
                "Temporarily enlarges the target area to help the model interact with small elements."
            });
        });

        ui.add_space(6.0);

        // 2.2 Workspace Crop
        ui.checkbox(
            &mut app.current_config.activer_recadrage_workspace,
            if is_fr {
                "Recadrer les captures sur l'application (Workspace Crop)"
            } else {
                "Crop captures to target application (Workspace Crop)"
            }
        );
        ui.indent("desc_activer_recadrage_workspace", |ui| {
            ui.weak(if is_fr {
                "Limite la capture visuelle aux dimensions de la fenêtre ciblée au lieu de capturer tout l'écran."
            } else {
                "Restricts visual capture to the active application window bounds instead of the entire screen."
            });
            if has_target_window && !app.current_config.activer_recadrage_workspace {
                ui.colored_label(
                    egui::Color32::from_rgb(255, 140, 0),
                    if is_fr {
                        "⚠️ Fortement Recommandé : Activez cette case car vous ciblez une fenêtre spécifique."
                    } else {
                        "⚠️ Strongly Recommended: Enable this option since you targeted a specific window."
                    }
                );
            }
        });

        ui.add_space(6.0);

        // 2.3 Visual Tracing
        ui.checkbox(
            &mut app.current_config.trace_actions_visuelles,
            if is_fr {
                "Enregistrer les micro-captures des actions (Visual Tracing)"
            } else {
                "Save micro-captures of actions (Visual Tracing)"
            }
        );
        ui.indent("desc_trace_actions_visuelles", |ui| {
            ui.weak(if is_fr {
                "Capture et affiche de petites vignettes centrées sur la zone du clic dans l'onglet des tâches."
            } else {
                "Captures and displays small cropped thumbnails centered on clicked zones in the tasks tab."
            });
        });

        ui.add_space(6.0);

        // 2.4 ROI
        ui.checkbox(
            &mut app.current_config.activer_roi,
            if is_fr {
                "Restreindre les actions à une zone fixe (Fixed ROI)"
            } else {
                "Restrict actions to a fixed zone (Fixed ROI)"
            }
        );
        ui.indent("desc_activer_roi", |ui| {
            ui.weak(if is_fr {
                "Limite l'analyse et les clics de l'IA à une portion rectangulaire spécifique de l'écran."
            } else {
                "Limits AI analysis and mouse clicks to a specific rectangular sub-region of the screen."
            });
            if app.current_config.activer_roi {
                ui.add_space(4.0);
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
            }
        });

        ui.add_space(10.0);
        ui.separator();
        ui.add_space(4.0);

        // Category 3: Performance & Ressources / Performance & Resources
        ui.label(egui::RichText::new(if is_fr { "⚙️ Performance & Ressources" } else { "⚙️ Performance & Resources" }).strong().color(ui.visuals().hyperlink_color));
        ui.add_space(2.0);

        // 3.1 Fast/Slow
        ui.checkbox(
            &mut app.current_config.activer_systeme_fast_slow,
            if is_fr {
                "Routage rapide pour actions simples (Fast/Slow)"
            } else {
                "Fast routing for simple actions (Fast/Slow)"
            }
        );
        ui.indent("desc_activer_systeme_fast_slow", |ui| {
            ui.weak(if is_fr {
                "Exécute les actions simples (comme l'attente) avec des règles locales sans appeler l'IA."
            } else {
                "Executes simple actions (like waiting) with local rules without calling the main AI."
            });
        });

        ui.add_space(6.0);

        // 3.2 Context History Compression
        ui.checkbox(
            &mut app.current_config.activer_compression_historique,
            if is_fr {
                "Compresser l'historique de contexte"
            } else {
                "Compress context history"
            }
        );
        ui.indent("desc_activer_compression_historique", |ui| {
            ui.weak(if is_fr {
                "Résume l'historique des actions précédentes dans le prompt pour économiser les jetons IA."
            } else {
                "Summarizes previous action history in the prompt to save AI tokens."
            });
        });

        ui.add_space(6.0);

        // 3.3 SSD Caching
        ui.checkbox(
            &mut app.current_config.economie_ecriture_ssd,
            if is_fr {
                "Économiser les écritures SSD (Cache RAM de 5 min)"
            } else {
                "Reduce SSD write wear (RAM Caching)"
            }
        );
        ui.indent("desc_economie_ecriture_ssd", |ui| {
            ui.weak(if is_fr {
                "Stocke les logs et le graphe en RAM avec écriture sur disque toutes les 5 minutes."
            } else {
                "Stores logs and graph in RAM with writing to disk every 5 minutes."
            });
        });

        ui.add_space(6.0);

        // 3.4 Auto-pause on user activity
        let detect_activity_lbl = app.t("chk_detect_activity");
        ui.checkbox(
            &mut app.current_config.detecter_activite_utilisateur,
            detect_activity_lbl
        );
        ui.indent("desc_detecter_activite_utilisateur", |ui| {
            ui.weak(if is_fr {
                "Met en pause l'IA si un mouvement de la souris ou une touche de clavier de l'utilisateur est détecté."
            } else {
                "Pauses the AI if mouse movement or keyboard input from the user is detected."
            });
        });
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

            // Modified Indicator
            if app.is_profile_modified() {
                ui.add_space(8.0);
                let modified_text = if app.current_config.langue == "Français" { "⚠️ Modifié (non sauvegardé)" } else { "⚠️ Modified (unsaved)" };
                ui.colored_label(egui::Color32::from_rgb(255, 140, 0), modified_text);
            }

            if selected_val != app.selected_profile && !selected_val.is_empty() {
                // User selected an existing profile from the dropdown
                app.load_profile(&selected_val);
            } else if response.changed() {
                // User typed something
                let trimmed = current_profile_name.trim().to_string();
                app.selected_profile = trimmed.clone();
                app.quick_start_profile_name = trimmed;
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
