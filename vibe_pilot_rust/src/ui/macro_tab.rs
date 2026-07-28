//! Standalone Macro Action Recorder UI Tab for VibePilot.
//!
//! Refactored with a chronological 4-step workflow layout (Session & Hotkeys -> Capture -> Graph / Text Script Protocol -> Playback)
//! fully integrated with VibePilot's multi-language localization system (app.t(...)), providing per-run options,
//! per-run execution statistics (launches, iterations executed, completed runs, interruptions), remaining iterations counter badge,
//! TEXT SCRIPT PROTOCOL EDITOR & PARSER (bidirectional DSL for AI / LLM pattern generation, sleep/wait commands, option overrides,
//! file export `.txt` / `.vibemacro` & file import via native file dialogs), auto-closing/deselecting node editor on focus loss/Esc,
//! Undo/Redo history stack (Ctrl+Z / Ctrl+Y), interactive video-game style keybinding capture for individual action nodes, action type
//! dropdown picker, STRICT MANUAL SAVING with explicit pre-save confirmation, music-score / sequencer style real-time visual step highlighting,
//! and an intuitive user experience.

use eframe::egui;
use crate::app::VibePilotApp;
use crate::event_bus::NotificationEvent;
use crate::macro_recorder::BindingTarget;
use crate::macro_recorder::script_parser::MacroScriptParser;

pub fn render_macro_recorder_tab(ui: &mut egui::Ui, app: &mut VibePilotApp) {
    use crate::macro_recorder::{ActionKind, RecordedAction};
    use std::sync::atomic::Ordering;

    let mut undo_requested = false;
    let mut redo_requested = false;

    // --- PRE-SAVE CONFIRMATION MODAL DIALOG ---
    if app.show_macro_save_confirm {
        egui::Window::new(app.t("msg_macro_save_title"))
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .show(ui.ctx(), |ui| {
                ui.vertical(|ui| {
                    ui.label(egui::RichText::new(app.t("msg_macro_save_text")).strong());
                    ui.add_space(12.0);
                    ui.horizontal(|ui| {
                        if ui.button(egui::RichText::new(app.t("btn_oui")).strong().color(egui::Color32::WHITE).background_color(egui::Color32::from_rgb(0, 140, 0))).clicked() {
                            app.save_macro_sessions_now();
                            app.show_macro_save_confirm = false;
                            app.bus.emit_notification(NotificationEvent::Log(app.t("msg_macro_save_success")));
                        }
                        ui.add_space(8.0);
                        if ui.button(app.t("btn_non")).clicked() {
                            app.show_macro_save_confirm = false;
                        }
                    });
                });
            });
    }

    // --- INTERACTIVE KEY CAPTURE FOR AN ACTION NODE IN THE GRAPH ---
    if let Some(target_node_id) = app.node_binding_capture {
        let mut captured_keys_opt: Option<Vec<String>> = None;
        ui.ctx().input(|i| {
            for event in &i.events {
                if let egui::Event::Key { key, pressed: true, modifiers, .. } = event {
                    if *key == egui::Key::Escape {
                        captured_keys_opt = None;
                        break;
                    }

                    let mut keys = Vec::new();
                    if modifiers.ctrl { keys.push("Ctrl".to_string()); }
                    if modifiers.alt { keys.push("Alt".to_string()); }
                    if modifiers.shift { keys.push("Shift".to_string()); }
                    if modifiers.mac_cmd { keys.push("Win".to_string()); }

                    let key_name = match key {
                        egui::Key::Space => "Space".to_string(),
                        egui::Key::Num0 => "0".to_string(),
                        egui::Key::Num1 => "1".to_string(),
                        egui::Key::Num2 => "2".to_string(),
                        egui::Key::Num3 => "3".to_string(),
                        egui::Key::Num4 => "4".to_string(),
                        egui::Key::Num5 => "5".to_string(),
                        egui::Key::Num6 => "6".to_string(),
                        egui::Key::Num7 => "7".to_string(),
                        egui::Key::Num8 => "8".to_string(),
                        egui::Key::Num9 => "9".to_string(),
                        egui::Key::A => "A".to_string(),
                        egui::Key::B => "B".to_string(),
                        egui::Key::C => "C".to_string(),
                        egui::Key::D => "D".to_string(),
                        egui::Key::E => "E".to_string(),
                        egui::Key::F => "F".to_string(),
                        egui::Key::G => "G".to_string(),
                        egui::Key::H => "H".to_string(),
                        egui::Key::I => "I".to_string(),
                        egui::Key::J => "J".to_string(),
                        egui::Key::K => "K".to_string(),
                        egui::Key::L => "L".to_string(),
                        egui::Key::M => "M".to_string(),
                        egui::Key::N => "N".to_string(),
                        egui::Key::O => "O".to_string(),
                        egui::Key::P => "P".to_string(),
                        egui::Key::Q => "Q".to_string(),
                        egui::Key::R => "R".to_string(),
                        egui::Key::S => "S".to_string(),
                        egui::Key::T => "T".to_string(),
                        egui::Key::U => "U".to_string(),
                        egui::Key::V => "V".to_string(),
                        egui::Key::W => "W".to_string(),
                        egui::Key::X => "X".to_string(),
                        egui::Key::Y => "Y".to_string(),
                        egui::Key::Z => "Z".to_string(),
                        other => format!("{:?}", other),
                    };

                    if key_name != "Alt" && key_name != "Control" && key_name != "Shift" {
                        if !keys.contains(&key_name) {
                            keys.push(key_name);
                        }
                        captured_keys_opt = Some(keys);
                        break;
                    }
                }
            }
        });

        if let Some(keys) = captured_keys_opt {
            app.push_macro_undo();
            if let Some(node) = app.macro_sequence.actions.iter_mut().find(|a| a.id == target_node_id) {
                if keys.len() > 1 {
                    node.action = ActionKind::KeyCombo { keys };
                } else if let Some(k) = keys.first() {
                    node.action = ActionKind::KeyPress { key: k.clone() };
                }
                node.label = node.action.description();
            }
            app.node_binding_capture = None;
        }
    }

    // --- KEYBOARD SHORTCUTS FOR UNDO / REDO / ESCAPE DESELECT ---
    let active_binding = app.macro_recorder.active_binding_capture();
    if active_binding.is_none() && app.node_binding_capture.is_none() {
        ui.ctx().input(|i| {
            if i.key_pressed(egui::Key::Escape) {
                app.selected_macro_node = None;
            } else if i.modifiers.command || i.modifiers.ctrl {
                if i.key_pressed(egui::Key::Z) {
                    if i.modifiers.shift {
                        redo_requested = true;
                    } else {
                        undo_requested = true;
                    }
                } else if i.key_pressed(egui::Key::Y) {
                    redo_requested = true;
                }
            }
        });
    }

    if undo_requested {
        if app.undo_macro_action() {
            app.bus.emit_notification(NotificationEvent::Log("↩️ Action annulée (Undo)".to_string()));
        }
    } else if redo_requested {
        if app.redo_macro_action() {
            app.bus.emit_notification(NotificationEvent::Log("↪️ Action rétablie (Redo)".to_string()));
        }
    }

    ui.vertical(|ui| {
        ui.heading(app.t("macro_heading"));
        ui.label(app.t("macro_subheading"));
        ui.add_space(8.0);

        let is_rec = app.macro_recorder.is_recording();
        let is_play = app.is_playing_macro;
        let live_captured_keys = app.macro_recorder.get_captured_binding_keys();
        let current_playback_step = app.macro_active_playback_step.lock().ok().and_then(|g| *g);

        // Track recording state transitions to automatically preserve undo state on recording start
        if is_rec && !app.was_recording_macro {
            app.push_macro_undo();
        }
        app.was_recording_macro = is_rec;

        // --- INTERACTIVE KEYBINDING CAPTURE VIA EGUI INPUT EVENTS ---
        if let Some(target) = active_binding {
            let mut captured_combo: Option<Vec<String>> = None;
            let mut cancel_requested = false;

            ui.ctx().input(|i| {
                for event in &i.events {
                    if let egui::Event::Key { key, pressed: true, modifiers, .. } = event {
                        if *key == egui::Key::Escape {
                            cancel_requested = true;
                            break;
                        }

                        let mut keys = Vec::new();
                        if modifiers.ctrl { keys.push("Ctrl".to_string()); }
                        if modifiers.alt { keys.push("Alt".to_string()); }
                        if modifiers.shift { keys.push("Shift".to_string()); }
                        if modifiers.mac_cmd { keys.push("Win".to_string()); }

                        let key_name = match key {
                            egui::Key::Space => "Space".to_string(),
                            egui::Key::Num0 => "0".to_string(),
                            egui::Key::Num1 => "1".to_string(),
                            egui::Key::Num2 => "2".to_string(),
                            egui::Key::Num3 => "3".to_string(),
                            egui::Key::Num4 => "4".to_string(),
                            egui::Key::Num5 => "5".to_string(),
                            egui::Key::Num6 => "6".to_string(),
                            egui::Key::Num7 => "7".to_string(),
                            egui::Key::Num8 => "8".to_string(),
                            egui::Key::Num9 => "9".to_string(),
                            egui::Key::A => "A".to_string(),
                            egui::Key::B => "B".to_string(),
                            egui::Key::C => "C".to_string(),
                            egui::Key::D => "D".to_string(),
                            egui::Key::E => "E".to_string(),
                            egui::Key::F => "F".to_string(),
                            egui::Key::G => "G".to_string(),
                            egui::Key::H => "H".to_string(),
                            egui::Key::I => "I".to_string(),
                            egui::Key::J => "J".to_string(),
                            egui::Key::K => "K".to_string(),
                            egui::Key::L => "L".to_string(),
                            egui::Key::M => "M".to_string(),
                            egui::Key::N => "N".to_string(),
                            egui::Key::O => "O".to_string(),
                            egui::Key::P => "P".to_string(),
                            egui::Key::Q => "Q".to_string(),
                            egui::Key::R => "R".to_string(),
                            egui::Key::S => "S".to_string(),
                            egui::Key::T => "T".to_string(),
                            egui::Key::U => "U".to_string(),
                            egui::Key::V => "V".to_string(),
                            egui::Key::W => "W".to_string(),
                            egui::Key::X => "X".to_string(),
                            egui::Key::Y => "Y".to_string(),
                            egui::Key::Z => "Z".to_string(),
                            other => format!("{:?}", other),
                        };

                        if key_name != "Alt" && key_name != "Control" && key_name != "Shift" {
                            if !keys.contains(&key_name) {
                                keys.push(key_name);
                            }
                            captured_combo = Some(keys);
                            break;
                        }
                    }
                }
            });

            if cancel_requested {
                app.macro_recorder.cancel_binding_capture();
            } else if let Some(combo) = captured_combo {
                match target {
                    BindingTarget::StartRecording => app.macro_hotkey_config.start_recording_keys = combo,
                    BindingTarget::StopRecording => app.macro_hotkey_config.stop_recording_keys = combo,
                }
                app.macro_recorder.set_hotkeys(app.macro_hotkey_config.clone());
                app.macro_recorder.cancel_binding_capture();
            }
        }
        
        // Sync latest hotkey config from recorder
        app.macro_hotkey_config = app.macro_recorder.get_hotkeys();

        // Sync recorded items from background listener thread into active sequence while recording
        if is_rec {
            app.macro_sequence = app.macro_recorder.get_sequence();
            if let Some(run) = app.macro_session_manager.active_run_mut() {
                run.sequence = app.macro_sequence.clone();
            }
        }

        // Helper closure to load selected run state into app fields
        let sync_active_run_to_app = |app_ref: &mut VibePilotApp| {
            if let Some(r) = app_ref.macro_session_manager.active_run() {
                app_ref.macro_sequence = r.sequence.clone();
                app_ref.macro_recorder.set_sequence(app_ref.macro_sequence.clone());
                app_ref.macro_iterations = r.iterations;
                app_ref.macro_start_delay_sec = r.start_delay_sec;
                app_ref.macro_record_start_delay_sec = r.record_start_delay_sec;
            }
        };

        // Pre-fetch translations to avoid borrow checker conflicts
        let t_run_label = app.t("macro_run_label");
        let t_lbl_delay_ms = app.t("macro_lbl_delay_ms");
        let t_edit_node_title = app.t("macro_edit_node_title");
        let t_lbl_action_type = app.t("macro_lbl_action_type");
        let t_type_keypress = app.t("macro_type_keypress");
        let t_type_keyhold = app.t("macro_type_keyhold");
        let t_type_keycombo = app.t("macro_type_keycombo");
        let t_type_click = app.t("macro_type_click");
        let t_type_mousedown = app.t("macro_type_mousedown");
        let t_type_mouseup = app.t("macro_type_mouseup");
        let t_type_drag = app.t("macro_type_drag");
        let t_type_move = app.t("macro_type_move");
        let t_type_scroll = app.t("macro_type_scroll");
        let t_type_wait = app.t("macro_type_wait");
        let t_btn_node_bind = app.t("macro_btn_node_bind");
        let t_btn_double_click = app.t("macro_btn_double_click");
        let t_drag_from = app.t("macro_drag_from");
        let t_drag_to = app.t("macro_drag_to");
        let t_key_label = app.t("macro_key_label");
        let t_hold_duration_ms = app.t("macro_hold_duration_ms");
        let t_key_combo_title = app.t("macro_key_combo_title");
        let t_wait_duration_ms = app.t("macro_wait_duration_ms");
        let t_btn_close_editor = app.t("macro_btn_close_editor");
        let t_lbl_remaining = app.t("macro_lbl_remaining_iters");

        // =========================================================================
        // ÉTAPE 1: CONFIGURATION (SESSION, RACCOURCIS GLOBAUX & STATISTIQUES)
        // =========================================================================
        ui.group(|ui| {
            ui.label(egui::RichText::new(app.t("macro_step1_title")).heading().size(15.0));
            ui.separator();

            ui.columns(2, |columns| {
                // --- Colonne Gauche: Gestion des Runs ---
                columns[0].vertical(|ui| {
                    ui.label(egui::RichText::new(app.t("macro_session_active")).strong());
                    ui.add_space(4.0);

                    ui.horizontal(|ui| {
                        if ui.button("«").on_hover_text("Premier Run").clicked() {
                            app.macro_session_manager.select_first();
                            sync_active_run_to_app(app);
                        }
                        if ui.button("‹").on_hover_text("Run Précédent").clicked() {
                            app.macro_session_manager.select_previous();
                            sync_active_run_to_app(app);
                        }

                        let total_runs = app.macro_session_manager.runs.len();
                        let active_idx = app.macro_session_manager.active_index + 1;
                        ui.label(egui::RichText::new(format!("Run {}/{}", active_idx, total_runs)).strong());

                        if ui.button("›").on_hover_text("Run Suivant").clicked() {
                            app.macro_session_manager.select_next();
                            sync_active_run_to_app(app);
                        }
                        if ui.button("»").on_hover_text("Dernier Run").clicked() {
                            app.macro_session_manager.select_last();
                            sync_active_run_to_app(app);
                        }
                    });

                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        if let Some(run) = app.macro_session_manager.active_run_mut() {
                            ui.label(&t_run_label);
                            if ui.text_edit_singleline(&mut run.name).changed() {
                                run.sequence.name = run.name.clone();
                                app.macro_sequence.name = run.name.clone();
                            }
                        }
                    });

                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        if ui.button(app.t("macro_btn_new")).clicked() {
                            let new_idx = app.macro_session_manager.add_new_run();
                            if let Some(r) = app.macro_session_manager.runs.get(new_idx) {
                                app.macro_sequence = r.sequence.clone();
                                app.macro_recorder.set_sequence(app.macro_sequence.clone());
                                app.macro_iterations = r.iterations;
                                app.macro_start_delay_sec = r.start_delay_sec;
                                app.macro_record_start_delay_sec = r.record_start_delay_sec;
                                app.selected_macro_node = None;
                                app.bus.emit_notification(NotificationEvent::Log(format!("Création du {}", r.name)));
                            }
                        }
                        if ui.button(app.t("macro_btn_delete")).clicked() {
                            if app.macro_session_manager.delete_active_run() {
                                sync_active_run_to_app(app);
                                app.selected_macro_node = None;
                                app.bus.emit_notification(NotificationEvent::Log("Run supprimé".to_string()));
                            }
                        }
                        // MANUAL SAVE BUTTON TRIGGERING CONFIRMATION DIALOG
                        if ui.button(egui::RichText::new(app.t("macro_btn_save")).color(egui::Color32::WHITE).background_color(egui::Color32::from_rgb(0, 120, 200))).on_hover_text(app.t("macro_tip_save")).clicked() {
                            app.show_macro_save_confirm = true;
                        }
                    });

                    // --- MACRO EXECUTION STATISTICS COLLAPSIBLE BOX ---
                    ui.add_space(8.0);
                    let stats_box_label = app.t("macro_stats_box");
                    if let Some(active_run) = app.macro_session_manager.active_run() {
                        let stats = &active_run.stats;
                        ui.collapsing(egui::RichText::new(&stats_box_label).strong().size(13.0), |ui| {
                            ui.label(format!("{} {}", app.t("macro_stats_launches"), stats.total_launches));
                            ui.label(format!("{} {}", app.t("macro_stats_executed_iters"), stats.total_iterations_executed));
                            ui.label(format!("{} {}", app.t("macro_stats_completed_runs"), stats.completed_runs_count));
                            ui.label(format!("{} {}", app.t("macro_stats_interruptions"), stats.interruption_count));
                        });
                    }
                });

                // --- Colonne Droite: Raccourcis Globaux ---
                columns[1].vertical(|ui| {
                    ui.label(egui::RichText::new(app.t("macro_hotkeys_title")).strong());
                    ui.add_space(4.0);

                    // Start Hotkey Row
                    ui.horizontal(|ui| {
                        ui.label(app.t("macro_hotkey_start"));
                        if active_binding == Some(BindingTarget::StartRecording) {
                            let display = if live_captured_keys.is_empty() {
                                app.t("macro_capture_press_key")
                            } else {
                                format!("{} {}", app.t("macro_capture_detected"), live_captured_keys.join(" + "))
                            };
                            ui.label(egui::RichText::new(display).color(egui::Color32::from_rgb(255, 165, 0)).strong());
                            if ui.button(app.t("macro_btn_validate")).clicked() {
                                app.macro_recorder.commit_binding_capture();
                            }
                            if ui.button(app.t("macro_btn_cancel")).on_hover_text("Annuler").clicked() {
                                app.macro_recorder.cancel_binding_capture();
                            }
                        } else {
                            let mut start_str = app.macro_hotkey_config.start_recording_keys.join(" + ");
                            if ui.button(app.t("macro_btn_capture")).on_hover_text("Cliquez puis appuyez sur votre combinaison au clavier (ex: Alt + S)").clicked() {
                                app.macro_recorder.start_binding_capture(BindingTarget::StartRecording);
                            }
                            if ui.add(egui::TextEdit::singleline(&mut start_str).desired_width(100.0)).changed() {
                                let keys: Vec<String> = start_str.split('+').map(|s| s.trim().to_string()).collect();
                                app.macro_hotkey_config.start_recording_keys = keys;
                                app.macro_recorder.set_hotkeys(app.macro_hotkey_config.clone());
                            }
                        }
                    });

                    ui.add_space(4.0);

                    // Stop Hotkey Row
                    ui.horizontal(|ui| {
                        ui.label(app.t("macro_hotkey_stop"));
                        if active_binding == Some(BindingTarget::StopRecording) {
                            let display = if live_captured_keys.is_empty() {
                                app.t("macro_capture_press_key")
                            } else {
                                format!("{} {}", app.t("macro_capture_detected"), live_captured_keys.join(" + "))
                            };
                            ui.label(egui::RichText::new(display).color(egui::Color32::from_rgb(255, 165, 0)).strong());
                            if ui.button(app.t("macro_btn_validate")).clicked() {
                                app.macro_recorder.commit_binding_capture();
                            }
                            if ui.button(app.t("macro_btn_cancel")).on_hover_text("Annuler").clicked() {
                                app.macro_recorder.cancel_binding_capture();
                            }
                        } else {
                            let mut stop_str = app.macro_hotkey_config.stop_recording_keys.join(" + ");
                            if ui.button(app.t("macro_btn_capture")).on_hover_text("Cliquez puis appuyez sur votre combinaison au clavier (ex: Alt + Space)").clicked() {
                                app.macro_recorder.start_binding_capture(BindingTarget::StopRecording);
                            }
                            if ui.add(egui::TextEdit::singleline(&mut stop_str).desired_width(100.0)).changed() {
                                let keys: Vec<String> = stop_str.split('+').map(|s| s.trim().to_string()).collect();
                                app.macro_hotkey_config.stop_recording_keys = keys;
                                app.macro_recorder.set_hotkeys(app.macro_hotkey_config.clone());
                            }
                        }
                    });

                    ui.add_space(4.0);
                    ui.label(egui::RichText::new(app.t("macro_hotkey_tip")).italics().size(12.0));
                });
            });
        });

        ui.add_space(8.0);

        // =========================================================================
        // ÉTAPE 2: ENREGISTREMENT (CAPTURE)
        // =========================================================================
        ui.group(|ui| {
            ui.label(egui::RichText::new(app.t("macro_step2_title")).heading().size(15.0));
            ui.separator();

            ui.horizontal(|ui| {
                if is_rec {
                    if ui.button(egui::RichText::new(app.t("macro_btn_stop_record")).size(15.0).color(egui::Color32::WHITE).background_color(egui::Color32::RED)).clicked() {
                        app.macro_recorder.stop_recording();
                        app.macro_sequence = app.macro_recorder.get_sequence();
                        if let Ok(mut c) = app.macro_recording_countdown_remaining.lock() {
                            *c = None;
                        }
                        app.bus.emit_notification(NotificationEvent::Log("Enregistrement macro arrêté ! N'oubliez pas de cliquer sur 💾 Sauvegarder pour pérenniser.".to_string()));
                    }
                } else {
                    if ui.button(egui::RichText::new(app.t("macro_btn_start_record")).size(15.0).color(egui::Color32::WHITE).background_color(egui::Color32::from_rgb(180, 0, 0))).clicked() {
                        app.push_macro_undo();
                        if !app.macro_sequence.actions.is_empty() {
                            let new_idx = app.macro_session_manager.add_new_run();
                            if let Some(r) = app.macro_session_manager.runs.get(new_idx) {
                                app.macro_sequence = r.sequence.clone();
                            }
                        }
                        app.macro_recorder.clear();
                        app.macro_sequence.clear();

                        let record_delay = app.macro_record_start_delay_sec;
                        let recorder = app.macro_recorder.clone();
                        let bus = app.bus.clone();
                        let countdown_ref = app.macro_recording_countdown_remaining.clone();

                        if record_delay > 0 {
                            bus.emit_notification(NotificationEvent::Log(format!("Démarrage enregistrement dans {record_delay} sec...")));
                            app.rt.spawn(async move {
                                for remaining in (1..=record_delay).rev() {
                                    if let Ok(mut c) = countdown_ref.lock() {
                                        *c = Some(remaining);
                                    }
                                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                                }
                                if let Ok(mut c) = countdown_ref.lock() {
                                    *c = None;
                                }
                                recorder.start_recording();
                                bus.emit_notification(NotificationEvent::Log("🔴 Enregistrement macro démarré ! (Captures en cours)".to_string()));
                            });
                        } else {
                            recorder.start_recording();
                            bus.emit_notification(NotificationEvent::Log("🔴 Enregistrement macro démarré !".to_string()));
                        }
                    }
                }

                ui.add_space(16.0);
                ui.label(app.t("macro_lbl_record_delay"));
                ui.add(egui::DragValue::new(&mut app.macro_record_start_delay_sec).range(0..=60));
                ui.label(app.t("macro_unit_seconds"));
            });

            // Live Countdown Chrono Badge Display for Recording
            let rec_countdown_val = app.macro_recording_countdown_remaining.lock().ok().and_then(|g| *g);
            if let Some(secs) = rec_countdown_val {
                ui.add_space(6.0);
                let chrono_text = app.t("macro_chrono_rec_start").replace("{}", &secs.to_string());
                ui.colored_label(
                    egui::Color32::from_rgb(255, 60, 60),
                    egui::RichText::new(chrono_text).strong().size(15.0),
                );
            }
        });

        ui.add_space(8.0);

        // =========================================================================
        // ÉTAPE 3: GRAPHE D'ACTIONS / ÉDITEUR TEXTE PROTOCOLE IA (IMPORT / EXPORT)
        // =========================================================================
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(app.t("macro_step3_title")).heading().size(15.0));

                ui.add_space(12.0);
                // VIEW MODE SWITCHER (Graphique vs Texte Script Protocol)
                if ui.selectable_label(!app.macro_view_mode_text, app.t("macro_tab_graph_view")).clicked() {
                    app.macro_view_mode_text = false;
                }
                if ui.selectable_label(app.macro_view_mode_text, app.t("macro_tab_text_view")).clicked() {
                    app.macro_view_mode_text = true;
                    // Auto-generate text script when switching to text view
                    app.macro_script_text = MacroScriptParser::to_script(
                        &app.macro_sequence,
                        app.macro_iterations,
                        app.macro_start_delay_sec,
                        app.macro_record_start_delay_sec,
                    );
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // MANUAL SAVE BUTTON TRIGGERING CONFIRMATION DIALOG
                    if ui.button(egui::RichText::new(app.t("macro_btn_save")).color(egui::Color32::WHITE).background_color(egui::Color32::from_rgb(0, 120, 200))).on_hover_text(app.t("macro_tip_save")).clicked() {
                        app.show_macro_save_confirm = true;
                    }

                    // --- UNDO / REDO BUTTONS ---
                    let redo_enabled = !app.macro_redo_stack.is_empty();
                    if ui.add_enabled(redo_enabled, egui::Button::new(app.t("macro_btn_redo"))).on_hover_text(app.t("macro_tip_redo")).clicked() {
                        if app.redo_macro_action() {
                            app.bus.emit_notification(NotificationEvent::Log("↪️ Action rétablie (Redo)".to_string()));
                        }
                    }

                    let undo_enabled = !app.macro_undo_stack.is_empty();
                    if ui.add_enabled(undo_enabled, egui::Button::new(app.t("macro_btn_undo"))).on_hover_text(app.t("macro_tip_undo")).clicked() {
                        if app.undo_macro_action() {
                            app.bus.emit_notification(NotificationEvent::Log("↩️ Action annulée (Undo)".to_string()));
                        }
                    }

                    // SCRIPT EXPORT / IMPORT FILE BUTTONS (Accessible in both modes)
                    if ui.button(app.t("macro_btn_export_file")).on_hover_text("Exporter la séquence sous forme de fichier texte protocole (.txt)").clicked() {
                        let run_name = app.macro_session_manager.active_run().map(|r| r.name.clone()).unwrap_or_else(|| "macro_script".to_string());
                        let safe_filename = format!("{}.txt", run_name.replace(' ', "_"));

                        if let Some(path) = rfd::FileDialog::new()
                            .set_file_name(&safe_filename)
                            .add_filter("Text Script Protocol", &["txt", "vibemacro"])
                            .save_file()
                        {
                            let script_content = MacroScriptParser::to_script(
                                &app.macro_sequence,
                                app.macro_iterations,
                                app.macro_start_delay_sec,
                                app.macro_record_start_delay_sec,
                            );
                            if let Err(e) = std::fs::write(&path, script_content) {
                                app.bus.emit_notification(NotificationEvent::Log(format!("⚠️ Erreur lors de l'export du script : {e}")));
                            } else {
                                app.bus.emit_notification(NotificationEvent::Log(format!("📤 Script protocole exporté vers {}", path.display())));
                            }
                        }
                    }

                    if ui.button(app.t("macro_btn_import_file")).on_hover_text("Importer un fichier script texte protocole (.txt)").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("Text Script Protocol", &["txt", "vibemacro", "log", "json"])
                            .pick_file()
                        {
                            match std::fs::read_to_string(&path) {
                                Ok(content) => {
                                    app.macro_script_text = content;
                                    match MacroScriptParser::from_script(&app.macro_script_text) {
                                        Ok((mut parsed_seq, parsed_iters, parsed_start, parsed_rec, rel_entries)) => {
                                            // Resolve relative coordinates using the focused screen
                                            if !rel_entries.is_empty() {
                                                let focused = MacroScriptParser::get_focused_screen_offset();
                                                MacroScriptParser::resolve_relative_coords(&mut parsed_seq, &rel_entries, &[focused]);
                                            }
                                            app.push_macro_undo();
                                            app.macro_sequence = parsed_seq.clone();
                                            app.macro_recorder.set_sequence(parsed_seq.clone());

                                            if let Some(r) = app.macro_session_manager.active_run_mut() {
                                                r.sequence = parsed_seq;
                                                if let Some(it) = parsed_iters { r.iterations = it; }
                                                if let Some(st) = parsed_start { r.start_delay_sec = st; }
                                                if let Some(rc) = parsed_rec { r.record_start_delay_sec = rc; }
                                            }

                                            if let Some(it) = parsed_iters { app.macro_iterations = it; }
                                            if let Some(st) = parsed_start { app.macro_start_delay_sec = st; }
                                            if let Some(rc) = parsed_rec { app.macro_record_start_delay_sec = rc; }

                                            app.bus.emit_notification(NotificationEvent::Log(format!("📥 Script protocole importé depuis {}", path.display())));
                                        }
                                        Err(err_msg) => {
                                            app.bus.emit_notification(NotificationEvent::Log(format!("⚠️ Erreur de syntaxe du script : {err_msg}")));
                                        }
                                    }
                                }
                                Err(e) => {
                                    app.bus.emit_notification(NotificationEvent::Log(format!("⚠️ Impossible de lire le fichier : {e}")));
                                }
                            }
                        }
                    }

                    if !app.macro_view_mode_text {
                        if ui.button(app.t("macro_btn_add_action")).clicked() {
                            app.push_macro_undo();
                            let new_id = app.macro_sequence.add_action(200, ActionKind::KeyPress { key: "A".to_string() });
                            app.macro_recorder.set_sequence(app.macro_sequence.clone());
                            app.selected_macro_node = Some(new_id);
                            app.node_binding_capture = Some(new_id);
                        }
                        if ui.button(app.t("macro_btn_clear_run")).clicked() {
                            app.push_macro_undo();
                            app.macro_recorder.clear();
                            app.macro_sequence.clear();
                            app.selected_macro_node = None;
                            app.node_binding_capture = None;
                            app.bus.emit_notification(NotificationEvent::Log("Run vidé".to_string()));
                        }
                    }
                });
            });
            ui.separator();

            if app.macro_view_mode_text {
                // =========================================================================
                // VIEW MODE: TEXT SCRIPT PROTOCOL EDITOR (FOR AI & SCRIPTING)
                // =========================================================================
                ui.label(egui::RichText::new(app.t("macro_ai_protocol_tip")).italics().size(12.5).color(egui::Color32::from_rgb(100, 180, 255)));
                ui.add_space(6.0);

                ui.horizontal(|ui| {
                    if ui.button(egui::RichText::new(app.t("macro_btn_script_to_graph")).strong().color(egui::Color32::WHITE).background_color(egui::Color32::from_rgb(0, 150, 80))).clicked() {
                        match MacroScriptParser::from_script(&app.macro_script_text) {
                            Ok((mut parsed_seq, parsed_iters, parsed_start, parsed_rec, rel_entries)) => {
                                // Resolve relative coordinates using the focused screen
                                if !rel_entries.is_empty() {
                                    let focused = MacroScriptParser::get_focused_screen_offset();
                                    MacroScriptParser::resolve_relative_coords(&mut parsed_seq, &rel_entries, &[focused]);
                                }
                                app.push_macro_undo();
                                app.macro_sequence = parsed_seq.clone();
                                app.macro_recorder.set_sequence(parsed_seq.clone());

                                if let Some(r) = app.macro_session_manager.active_run_mut() {
                                    r.sequence = parsed_seq;
                                    if let Some(it) = parsed_iters { r.iterations = it; }
                                    if let Some(st) = parsed_start { r.start_delay_sec = st; }
                                    if let Some(rc) = parsed_rec { r.record_start_delay_sec = rc; }
                                }

                                if let Some(it) = parsed_iters { app.macro_iterations = it; }
                                if let Some(st) = parsed_start { app.macro_start_delay_sec = st; }
                                if let Some(rc) = parsed_rec { app.macro_record_start_delay_sec = rc; }

                                app.bus.emit_notification(NotificationEvent::Log("✅ Script protocole converti et chargé dans le graphe !".to_string()));
                            }
                            Err(err_msg) => {
                                app.bus.emit_notification(NotificationEvent::Log(format!("⚠️ Erreur de syntaxe protocole : {}", err_msg)));
                            }
                        }
                    }

                    if ui.button(app.t("macro_btn_graph_to_script")).clicked() {
                        app.macro_script_text = MacroScriptParser::to_script(
                            &app.macro_sequence,
                            app.macro_iterations,
                            app.macro_start_delay_sec,
                            app.macro_record_start_delay_sec,
                        );
                        app.bus.emit_notification(NotificationEvent::Log("🔄 Protocole texte synchronisé depuis le graphe !".to_string()));
                    }

                    if ui.button(app.t("macro_btn_copy_script")).clicked() {
                        ui.ctx().copy_text(app.macro_script_text.clone());
                        app.bus.emit_notification(NotificationEvent::Log("📋 Protocole texte copié dans le presse-papier !".to_string()));
                    }
                });

                ui.add_space(6.0);

                egui::ScrollArea::vertical().id_salt("macro_script_text_scroll").max_height(280.0).show(ui, |ui| {
                    ui.add(
                        egui::TextEdit::multiline(&mut app.macro_script_text)
                            .font(egui::TextStyle::Monospace)
                            .desired_width(f32::INFINITY)
                            .desired_rows(15)
                    );
                });

            } else {
                // =========================================================================
                // VIEW MODE: GRAPHIC NODE EDITOR
                // =========================================================================
                if app.macro_sequence.actions.is_empty() {
                    ui.add_space(6.0);
                    ui.label(egui::RichText::new(app.t("macro_no_actions")).italics());
                    ui.add_space(6.0);
                } else {
                    let action_count_str = app.t("macro_action_count").replace("{}", &app.macro_sequence.actions.len().to_string());
                    ui.label(egui::RichText::new(action_count_str).strong());
                    ui.separator();

                    let mut action_to_remove = None;
                    let mut newly_selected = None;

                    egui::ScrollArea::vertical().id_salt("standalone_macro_graph_scroll").max_height(280.0).show(ui, |ui| {
                        let selected_id = app.selected_macro_node;

                        for (idx, action) in app.macro_sequence.actions.iter_mut().enumerate() {
                            let is_active_step = match current_playback_step {
                                Some((_iter, step_idx)) => step_idx == idx,
                                None => false,
                            };

                            let active_iter = current_playback_step.map(|(iter, _)| iter).unwrap_or(1);
                            let total_iters = if app.macro_iterations == 0 { 1 } else { app.macro_iterations };
                            let rem_iters = total_iters.saturating_sub(active_iter);

                            ui.horizontal(|ui| {
                                ui.checkbox(&mut action.enabled, "");

                                let badge_text = format!("[#{}] {}", idx + 1, action.label);
                                let is_selected = selected_id == Some(action.id);
                                
                                // Visual Music Score Sequencer Highlighting (Glowing Green Badge for active step)
                                if is_active_step {
                                    let rem_str = t_lbl_remaining.replace("{}", &rem_iters.to_string());
                                    let progress_fmt = format!("▶️ EN COURS (Iter #{active_iter}/{total_iters} - {rem_str}) {badge_text}");
                                    ui.colored_label(
                                        egui::Color32::from_rgb(0, 220, 100),
                                        egui::RichText::new(progress_fmt).strong().background_color(egui::Color32::from_rgb(20, 60, 30)),
                                    );
                                } else {
                                    let label_widget = egui::SelectableLabel::new(is_selected, egui::RichText::new(badge_text).strong());
                                    if ui.add(label_widget).clicked() {
                                        newly_selected = Some(if is_selected { None } else { Some(action.id) });
                                    }
                                }

                                ui.label(&t_lbl_delay_ms);
                                ui.add(egui::DragValue::new(&mut action.delay_ms).range(0..=60000));

                                if ui.button("🗑️").clicked() {
                                    action_to_remove = Some(action.id);
                                }
                            });

                            // Extended editor panel if node is selected
                            if selected_id == Some(action.id) {
                                ui.indent(format!("indent_standalone_node_{}", action.id), |ui| {
                                    ui.group(|ui| {
                                        ui.horizontal(|ui| {
                                            ui.label(egui::RichText::new(&t_edit_node_title).strong());
                                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                if ui.button(&t_btn_close_editor).clicked() {
                                                    newly_selected = Some(None);
                                                }
                                            });
                                        });

                                        // Action Kind Dropdown Picker
                                        ui.horizontal(|ui| {
                                            ui.label(&t_lbl_action_type);
                                            let current_kind_tag = match &action.action {
                                                ActionKind::KeyPress { .. } => 0,
                                                ActionKind::KeyHold { .. } => 1,
                                                ActionKind::KeyCombo { .. } => 2,
                                                ActionKind::Click { .. } => 3,
                                                ActionKind::MouseDown { .. } => 4,
                                                ActionKind::MouseUp { .. } => 5,
                                                ActionKind::DragAndDrop { .. } => 6,
                                                ActionKind::Move { .. } => 7,
                                                ActionKind::Scroll { .. } => 8,
                                                ActionKind::Wait { .. } => 9,
                                            };
                                            let mut selected_tag = current_kind_tag;

                                            egui::ComboBox::from_id_salt(format!("action_kind_combo_{}", action.id))
                                                .selected_text(match selected_tag {
                                                    0 => &t_type_keypress,
                                                    1 => &t_type_keyhold,
                                                    2 => &t_type_keycombo,
                                                    3 => &t_type_click,
                                                    4 => &t_type_mousedown,
                                                    5 => &t_type_mouseup,
                                                    6 => &t_type_drag,
                                                    7 => &t_type_move,
                                                    8 => &t_type_scroll,
                                                    _ => &t_type_wait,
                                                })
                                                .show_ui(ui, |ui| {
                                                    ui.selectable_value(&mut selected_tag, 0, &t_type_keypress);
                                                    ui.selectable_value(&mut selected_tag, 1, &t_type_keyhold);
                                                    ui.selectable_value(&mut selected_tag, 2, &t_type_keycombo);
                                                    ui.selectable_value(&mut selected_tag, 3, &t_type_click);
                                                    ui.selectable_value(&mut selected_tag, 4, &t_type_mousedown);
                                                    ui.selectable_value(&mut selected_tag, 5, &t_type_mouseup);
                                                    ui.selectable_value(&mut selected_tag, 6, &t_type_drag);
                                                    ui.selectable_value(&mut selected_tag, 7, &t_type_move);
                                                    ui.selectable_value(&mut selected_tag, 8, &t_type_scroll);
                                                    ui.selectable_value(&mut selected_tag, 9, &t_type_wait);
                                                });

                                            if selected_tag != current_kind_tag {
                                                action.action = match selected_tag {
                                                    0 => ActionKind::KeyPress { key: "A".to_string() },
                                                    1 => ActionKind::KeyHold { key: "A".to_string(), hold_duration_ms: 500 },
                                                    2 => ActionKind::KeyCombo { keys: vec!["Ctrl".to_string(), "C".to_string()] },
                                                    3 => ActionKind::Click { x: 500, y: 500, button: "Left".to_string(), double_click: false },
                                                    4 => ActionKind::MouseDown { x: 500, y: 500, button: "Left".to_string() },
                                                    5 => ActionKind::MouseUp { x: 500, y: 500, button: "Left".to_string() },
                                                    6 => ActionKind::DragAndDrop { from_x: 100, from_y: 100, to_x: 300, to_y: 300, button: "Left".to_string() },
                                                    7 => ActionKind::Move { x: 500, y: 500 },
                                                    8 => ActionKind::Scroll { dx: 0, dy: -120 },
                                                    _ => ActionKind::Wait { ms: 500 },
                                                };
                                            }
                                        });

                                        ui.add_space(4.0);

                                        match &mut action.action {
                                            ActionKind::Click { x, y, button, double_click } => {
                                                ui.horizontal(|ui| {
                                                    ui.label("X:");
                                                    ui.add(egui::DragValue::new(x));
                                                    ui.label("Y:");
                                                    ui.add(egui::DragValue::new(y));
                                                    ui.label("Button:");
                                                    ui.text_edit_singleline(button);
                                                    ui.checkbox(double_click, &t_btn_double_click);
                                                });
                                            }
                                            ActionKind::MouseDown { x, y, button } | ActionKind::MouseUp { x, y, button } => {
                                                ui.horizontal(|ui| {
                                                    ui.label("X:");
                                                    ui.add(egui::DragValue::new(x));
                                                    ui.label("Y:");
                                                    ui.add(egui::DragValue::new(y));
                                                    ui.label("Button:");
                                                    ui.text_edit_singleline(button);
                                                });
                                            }
                                            ActionKind::DragAndDrop { from_x, from_y, to_x, to_y, button } => {
                                                ui.horizontal(|ui| {
                                                    ui.label(&t_drag_from);
                                                    ui.add(egui::DragValue::new(from_x));
                                                    ui.label("Y:");
                                                    ui.add(egui::DragValue::new(from_y));
                                                    ui.label(&t_drag_to);
                                                    ui.add(egui::DragValue::new(to_x));
                                                    ui.label("Y:");
                                                    ui.add(egui::DragValue::new(to_y));
                                                    ui.label("Button:");
                                                    ui.text_edit_singleline(button);
                                                });
                                            }
                                            ActionKind::Move { x, y } => {
                                                ui.horizontal(|ui| {
                                                    ui.label("X:");
                                                    ui.add(egui::DragValue::new(x));
                                                    ui.label("Y:");
                                                    ui.add(egui::DragValue::new(y));
                                                });
                                            }
                                            ActionKind::KeyPress { key } => {
                                                ui.horizontal(|ui| {
                                                    ui.label(&t_key_label);
                                                    ui.text_edit_singleline(key);
                                                    if app.node_binding_capture == Some(action.id) {
                                                        ui.label(egui::RichText::new("⌨️ Pressez votre touche au clavier...").color(egui::Color32::from_rgb(255, 165, 0)).strong());
                                                    } else if ui.button(&t_btn_node_bind).on_hover_text("Cliquez puis appuyez sur n'importe quelle touche au clavier").clicked() {
                                                        app.node_binding_capture = Some(action.id);
                                                    }
                                                });
                                            }
                                            ActionKind::KeyHold { key, hold_duration_ms } => {
                                                ui.horizontal(|ui| {
                                                    ui.label(&t_key_label);
                                                    ui.text_edit_singleline(key);
                                                    if app.node_binding_capture == Some(action.id) {
                                                        ui.label(egui::RichText::new("⌨️ Pressez votre touche au clavier...").color(egui::Color32::from_rgb(255, 165, 0)).strong());
                                                    } else if ui.button(&t_btn_node_bind).on_hover_text("Cliquez puis appuyez sur n'importe quelle touche au clavier").clicked() {
                                                        app.node_binding_capture = Some(action.id);
                                                    }
                                                    ui.label(&t_hold_duration_ms);
                                                    ui.add(egui::DragValue::new(hold_duration_ms).range(1..=60000));
                                                });
                                            }
                                            ActionKind::KeyCombo { keys } => {
                                                ui.vertical(|ui| {
                                                    ui.horizontal(|ui| {
                                                        ui.label(egui::RichText::new(&t_key_combo_title).strong());
                                                        if app.node_binding_capture == Some(action.id) {
                                                            ui.label(egui::RichText::new("⌨️ Pressez votre combinaison au clavier...").color(egui::Color32::from_rgb(255, 165, 0)).strong());
                                                        } else if ui.button(&t_btn_node_bind).on_hover_text("Cliquez puis appuyez sur votre combinaison au clavier (ex: Ctrl + C)").clicked() {
                                                            app.node_binding_capture = Some(action.id);
                                                        }
                                                    });
                                                    for k in keys.iter_mut() {
                                                        ui.horizontal(|ui| {
                                                            ui.label("•");
                                                            ui.text_edit_singleline(k);
                                                        });
                                                    }
                                                });
                                            }
                                            ActionKind::Scroll { dx, dy } => {
                                                ui.horizontal(|ui| {
                                                    ui.label("dx:");
                                                    ui.add(egui::DragValue::new(dx));
                                                    ui.label("dy:");
                                                    ui.add(egui::DragValue::new(dy));
                                                });
                                            }
                                            ActionKind::Wait { ms } => {
                                                ui.horizontal(|ui| {
                                                    ui.label(&t_wait_duration_ms);
                                                    ui.add(egui::DragValue::new(ms).range(0..=60000));
                                                });
                                            }
                                        }
                                        action.label = action.action.description();
                                    });
                                });
                            }
                        }
                    });

                    if let Some(ns) = newly_selected {
                        app.selected_macro_node = ns;
                    }

                    if let Some(id_to_del) = action_to_remove {
                        app.push_macro_undo();
                        app.macro_sequence.remove_action(id_to_del);
                        if app.selected_macro_node == Some(id_to_del) {
                            app.selected_macro_node = None;
                        }
                        if app.node_binding_capture == Some(id_to_del) {
                            app.node_binding_capture = None;
                        }
                    }
                }
            }
        });

        ui.add_space(8.0);

        // =========================================================================
        // ÉTAPE 4: LANCEMENT & REJEU (EXECUTION)
        // =========================================================================
        ui.group(|ui| {
            ui.label(egui::RichText::new(app.t("macro_step4_title")).heading().size(15.0));
            ui.separator();

            ui.horizontal(|ui| {
                if is_play {
                    if ui.button(egui::RichText::new(app.t("macro_btn_stop_play")).size(15.0).color(egui::Color32::WHITE).background_color(egui::Color32::RED)).clicked() {
                        app.macro_playback_cancel.store(true, Ordering::SeqCst);
                        app.is_playing_macro = false;
                        if let Ok(mut c) = app.macro_countdown_remaining.lock() {
                            *c = None;
                        }
                        if let Ok(mut step_ref) = app.macro_active_playback_step.lock() {
                            *step_ref = None;
                        }

                        // Track Interruption in stats
                        if let Some(r) = app.macro_session_manager.active_run_mut() {
                            r.stats.interruption_count += 1;
                        }

                        app.bus.emit_notification(NotificationEvent::Log("Rejeu macro interrompu".to_string()));
                    }
                } else {
                    let play_btn_enabled = !app.macro_sequence.actions.is_empty() && !is_rec;
                    if ui.add_enabled(play_btn_enabled, egui::Button::new(egui::RichText::new(app.t("macro_btn_start_play")).size(15.0).color(egui::Color32::WHITE).background_color(egui::Color32::from_rgb(0, 140, 0)))).clicked() {
                        app.is_playing_macro = true;
                        app.macro_playback_cancel.store(false, Ordering::SeqCst);

                        // Increment Launch Counter in stats
                        if let Some(r) = app.macro_session_manager.active_run_mut() {
                            r.stats.total_launches += 1;
                        }
                        
                        let seq = app.macro_sequence.clone();
                        let iters = app.macro_iterations;
                        let start_delay = app.macro_start_delay_sec;
                        let cancel = app.macro_playback_cancel.clone();
                        let controller = app.orchestrator.controller.clone();
                        let bus = app.bus.clone();
                        let countdown_ref_cb = app.macro_countdown_remaining.clone();
                        let countdown_ref_spawn = app.macro_countdown_remaining.clone();
                        let step_ref = app.macro_active_playback_step.clone();
                        let step_ref_spawn = app.macro_active_playback_step.clone();

                        bus.emit_notification(NotificationEvent::Log(format!("Démarrage rejeu macro ({iters} itération(s), délai départ: {start_delay}s)...")));

                        let cb_countdown = Box::new(move |secs: u32| {
                            if let Ok(mut c) = countdown_ref_cb.lock() {
                                *c = Some(secs);
                            }
                        });

                        let cb_step = Box::new(move |iter: u32, idx: usize, _action: &ActionKind| {
                            if let Ok(mut s) = step_ref.lock() {
                                *s = Some((iter, idx));
                            }
                        });

                        app.rt.spawn(async move {
                            let res = crate::macro_recorder::player::MacroPlayer::play(
                                seq,
                                iters,
                                start_delay,
                                controller,
                                cancel,
                                Some(cb_countdown),
                                Some(cb_step),
                            ).await;
                            if let Ok(mut c) = countdown_ref_spawn.lock() {
                                *c = None;
                            }
                            if let Ok(mut s) = step_ref_spawn.lock() {
                                *s = None;
                            }
                            match res {
                                Ok(_) => bus.emit_notification(NotificationEvent::Log("Rejeu macro terminé avec succès !".to_string())),
                                Err(e) => bus.emit_notification(NotificationEvent::Log(format!("⚠️ Erreur lors du rejeu macro: {e}"))),
                            }
                        });
                    }
                }

                ui.add_space(16.0);
                ui.label(app.t("macro_lbl_iterations"));
                ui.add(egui::DragValue::new(&mut app.macro_iterations).range(1..=1000));

                ui.add_space(12.0);
                ui.label(app.t("macro_lbl_play_delay"));
                ui.add(egui::DragValue::new(&mut app.macro_start_delay_sec).range(0..=60));
                ui.label(app.t("macro_unit_sec"));

                ui.add_space(12.0);
                let mut has_global_delay = app.macro_sequence.global_delay_override_ms.is_some();
                if ui.checkbox(&mut has_global_delay, app.t("macro_chk_global_pacing")).changed() {
                    if has_global_delay {
                        app.macro_sequence.global_delay_override_ms = Some(150);
                    } else {
                        app.macro_sequence.global_delay_override_ms = None;
                    }
                }

                if let Some(ref mut delay_val) = app.macro_sequence.global_delay_override_ms {
                    ui.add(egui::DragValue::new(delay_val).range(0..=60000));
                }
            });

            // Live Countdown & Remaining Iterations Counter Display
            if is_play {
                if let Some((curr_iter, _step_idx)) = current_playback_step {
                    let total_iters = if app.macro_iterations == 0 { 1 } else { app.macro_iterations };
                    let rem_iters = total_iters.saturating_sub(curr_iter);
                    ui.add_space(6.0);
                    ui.colored_label(
                        egui::Color32::from_rgb(0, 200, 100),
                        egui::RichText::new(format!("🔄 LECTURE EN COURS : Itération {} / {} ({} restant(es))", curr_iter, total_iters, rem_iters)).strong().size(15.0),
                    );
                }
            }

            let countdown_val = app.macro_countdown_remaining.lock().ok().and_then(|g| *g);
            if let Some(secs) = countdown_val {
                ui.add_space(6.0);
                let chrono_text = app.t("macro_chrono_play_start").replace("{}", &secs.to_string());
                ui.colored_label(
                    egui::Color32::from_rgb(255, 165, 0),
                    egui::RichText::new(chrono_text).strong().size(15.0),
                );
            }
        });
    });
}
