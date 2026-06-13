use crate::app::{VibePilotApp, Tab};
use crate::event_bus::NotificationEvent;
use eframe::egui;

// ============================================================
// TIMELINE
// ============================================================
pub fn render_timeline(ui: &mut egui::Ui, app: &mut VibePilotApp) {
    ui.group(|ui| {
        ui.horizontal(|ui| {
            ui.label(app.t("frame_timeline"));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button(app.t("btn_clear_logs")).clicked() {
                    app.action_history.clear();
                    app.logs.clear();
                }
                let len = app.action_history.len();
                ui.label(format!("{} actions", len));
            });
        });
        ui.separator();

        let height = 45.0;
        let size = egui::Vec2::new(ui.available_width(), height);
        let (rect, _response) = ui.allocate_exact_size(size, egui::Sense::hover());
        
        ui.painter().line_segment(
            [rect.min + egui::vec2(10.0, height * 0.5), rect.max - egui::vec2(10.0, height * 0.5)],
            (1.5, egui::Color32::from_rgb(85, 85, 85)),
        );
        let actions = &app.action_history;
        let max_nodes = (rect.width() / 40.0).floor() as usize;
        let display = if actions.len() > max_nodes { &actions[actions.len() - max_nodes..] } else { actions };
        let colors: std::collections::HashMap<&str, egui::Color32> = [
            ("THINK", egui::Color32::from_rgb(255, 140, 0)),
            ("WAIT", egui::Color32::from_rgb(30, 144, 255)),
            ("COOL", egui::Color32::from_rgb(0, 206, 209)),
            ("SCROLL", egui::Color32::from_rgb(186, 85, 211)),
            ("CLICK_AND_TYPE", egui::Color32::from_rgb(50, 205, 50)),
            ("SUCCESS", egui::Color32::from_rgb(255, 215, 0)),
            ("FAIL", egui::Color32::from_rgb(255, 69, 0)),
            ("SLEEP", egui::Color32::from_rgb(64, 64, 64)),
            ("STATIC", egui::Color32::from_rgb(119, 136, 153)),
            ("ERROR", egui::Color32::from_rgb(220, 20, 60)),
        ].into_iter().collect();

        for (i, (action, tooltip)) in display.iter().enumerate() {
            let x = rect.min.x + 20.0 + (i as f32 * 40.0);
            let y = rect.min.y + height * 0.5;
            let color = colors.get(action.as_str()).copied().unwrap_or(egui::Color32::from_rgb(169, 169, 169));
            ui.painter().circle_filled(egui::pos2(x, y), 10.0, color);
            ui.painter().text(egui::pos2(x, y), egui::Align2::CENTER_CENTER, action_chars(action), egui::FontId::monospace(9.0), egui::Color32::BLACK);
            
            // Hover tooltip on timeline nodes
            let node_rect = egui::Rect::from_center_size(egui::pos2(x, y), egui::vec2(20.0, 20.0));
            if ui.rect_contains_pointer(node_rect) {
                egui::show_tooltip_at_pointer(ui.ctx(), ui.layer_id(), ui.make_persistent_id(format!("node_tooltip_{}", i)), |ui| {
                    ui.label(format!("Action: {}", action));
                    if !tooltip.is_empty() {
                        ui.label(format!("Details: {}", tooltip));
                    }
                });
            }
        }
        ui.horizontal(|ui| {
            let legend = if app.current_config.langue == "Français" {
                "Légende: T: Réflexion  W: IA Occupée  CD: Cooldown  S: Défiler  C: Saisie  ★: Succès  X: Échec  SL: Veille  ST: Écran Statique  ERR: Erreur"
            } else {
                "Legend: T: Thinking  W: Busy  CD: Stabilizing  S: Scroll  C: Type  ★: Success  X: Fail  SL: Sleep  ST: Static  ERR: Error"
            };
            ui.label(legend);
        });
    });
}

pub fn render_timeline_vertical(ui: &mut egui::Ui, app: &mut VibePilotApp, height: f32) {
    ui.group(|ui| {
        ui.set_width(110.0);
        ui.set_height(height);
        
        ui.vertical_centered(|ui| {
            let title = "Timeline";
            ui.label(egui::RichText::new(title).strong());
        });
        ui.separator();

        let size = egui::Vec2::new(90.0, height - 50.0);
        let (rect, _response) = ui.allocate_exact_size(size, egui::Sense::hover());
        
        let x_center = rect.min.x + rect.width() * 0.35;
        
        // Draw vertical line
        ui.painter().line_segment(
            [egui::pos2(x_center, rect.min.y + 10.0), egui::pos2(x_center, rect.max.y - 10.0)],
            (1.5, egui::Color32::from_rgb(85, 85, 85)),
        );

        let actions = &app.action_history;
        let spacing = 32.0;
        let max_nodes = ((rect.height() - 20.0) / spacing).floor() as usize;
        let display = if actions.len() > max_nodes { &actions[actions.len() - max_nodes..] } else { actions };

        let colors: std::collections::HashMap<&str, egui::Color32> = [
            ("THINK", egui::Color32::from_rgb(255, 140, 0)),
            ("WAIT", egui::Color32::from_rgb(30, 144, 255)),
            ("COOL", egui::Color32::from_rgb(0, 206, 209)),
            ("SCROLL", egui::Color32::from_rgb(186, 85, 211)),
            ("CLICK_AND_TYPE", egui::Color32::from_rgb(50, 205, 50)),
            ("SUCCESS", egui::Color32::from_rgb(255, 215, 0)),
            ("FAIL", egui::Color32::from_rgb(255, 69, 0)),
            ("SLEEP", egui::Color32::from_rgb(64, 64, 64)),
            ("STATIC", egui::Color32::from_rgb(119, 136, 153)),
            ("ERROR", egui::Color32::from_rgb(220, 20, 60)),
        ].into_iter().collect();

        for (i, (action, tooltip)) in display.iter().enumerate() {
            let x = x_center;
            let y = rect.min.y + 20.0 + (i as f32 * spacing);
            let color = colors.get(action.as_str()).copied().unwrap_or(egui::Color32::from_rgb(169, 169, 169));
            
            // Draw circle node
            ui.painter().circle_filled(egui::pos2(x, y), 8.0, color);
            ui.painter().text(
                egui::pos2(x, y),
                egui::Align2::CENTER_CENTER,
                action_chars(action),
                egui::FontId::monospace(8.0),
                egui::Color32::BLACK,
            );

            // Draw abbreviation label next to the circle
            ui.painter().text(
                egui::pos2(x + 12.0, y),
                egui::Align2::LEFT_CENTER,
                action_chars(action),
                egui::FontId::proportional(10.0),
                egui::Color32::LIGHT_GRAY,
            );
            
            // Hover tooltip on timeline nodes
            let node_rect = egui::Rect::from_center_size(egui::pos2(x, y), egui::vec2(16.0, 16.0));
            if ui.rect_contains_pointer(node_rect) {
                egui::show_tooltip_at_pointer(ui.ctx(), ui.layer_id(), ui.make_persistent_id(format!("node_tooltip_v_{}", i)), |ui| {
                    ui.label(format!("Action: {}", action));
                    if !tooltip.is_empty() {
                        ui.label(format!("Details: {}", tooltip));
                    }
                });
            }
        }
    });
}

fn action_chars(action: &str) -> String {
    match action {
        "THINK" => "T".to_string(), "WAIT" => "W".to_string(), "COOL" => "CD".to_string(),
        "SCROLL" => "S".to_string(), "CLICK_AND_TYPE" => "C".to_string(), "SUCCESS" => "*".to_string(),
        "FAIL" => "X".to_string(), "SLEEP" => "SL".to_string(), "STATIC" => "ST".to_string(),
        "ERROR" => "ERR".to_string(), "STARTUP" => "🚀".to_string(),
        _ => "?".to_string(),
    }
}

// ============================================================
// CONSOLE LOGS
// ============================================================
pub fn render_activity_console(ui: &mut egui::Ui, app: &mut VibePilotApp, height: f32) {
    ui.group(|ui| {
        ui.horizontal(|ui| {
                let label_logs = "🔴 Console & Logs";
                let label_report = if app.current_config.langue == "Français" { "📄 Rapport IA" } else { "📄 AI Report" };

            ui.selectable_value(&mut app.bottom_tab, 0, label_logs);
            ui.selectable_value(&mut app.bottom_tab, 1, label_report);

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if app.bottom_tab == 0 {
                    let clear_console_label = if app.current_config.langue == "Français" { "🗑️ Effacer la Console" } else { "🗑️ Clear Console" };
                    if ui.button(clear_console_label).clicked() {
                        app.logs.clear();
                    }
                } else {
                    let clear_report_label = if app.current_config.langue == "Français" { "🗑️ Effacer le Rapport" } else { "🗑️ Clear Report" };
                    if ui.button(clear_report_label).clicked() {
                        app.report_content.clear();
                    }
                }
            });
        });
        ui.separator();

        if app.bottom_tab == 0 {
            egui::ScrollArea::vertical()
                .max_width(ui.available_width())
                .max_height(height)
                .show(ui, |ui| {
                    ui.set_min_height(height - 15.0);
                    for log in &app.logs {
                        ui.colored_label(egui::Color32::from_rgb(0, 255, 0), log);
                    }
                });
        } else {
            egui::ScrollArea::vertical()
                .max_width(ui.available_width())
                .max_height(height)
                .show(ui, |ui| {
                    ui.set_min_height(height - 15.0);
                    if app.report_content.is_empty() {
                        let empty_msg = if app.current_config.langue == "Français" {
                            "Aucun rapport disponible pour le moment."
                        } else {
                            "No report available yet."
                        };
                        ui.colored_label(egui::Color32::LIGHT_GRAY, empty_msg);
                    } else {
                        ui.add(egui::TextEdit::multiline(&mut app.report_content)
                            .desired_width(ui.available_width())
                            .desired_rows(10));
                    }
                });
        }
    });
}


// ============================================================
// CONTROLS & SETTINGS (FOOTER)
// ============================================================
pub fn render_status_controls(ui: &mut egui::Ui, app: &mut VibePilotApp) {
    ui.horizontal(|ui| {
        // --- 1. Left side navigation (Previous button) ---
        let has_previous = app.active_tab == Tab::PromptEditor || app.active_tab == Tab::Console;
        if has_previous {
            let prev_label = if app.current_config.langue == "Français" { "◀ Précédent" } else { "◀ Previous" };
            let prev_btn = egui::Button::new(egui::RichText::new(prev_label).color(egui::Color32::WHITE).strong())
                .fill(egui::Color32::from_rgb(120, 120, 120))
                .min_size(egui::vec2(100.0, 30.0));
            if ui.add(prev_btn).clicked() {
                app.active_tab = match app.active_tab {
                    Tab::PromptEditor => Tab::GlobalConfig,
                    Tab::Console => Tab::PromptEditor,
                    _ => app.active_tab,
                };
            }
            ui.separator();
        }

        let status_color = match app.status_color.as_str() {
            "red" => egui::Color32::from_rgb(255, 69, 0),
            "green" => egui::Color32::from_rgb(50, 205, 50),
            "orange" => egui::Color32::from_rgb(255, 140, 0),
            "blue" => egui::Color32::from_rgb(30, 144, 255),
            "yellow" => egui::Color32::from_rgb(255, 215, 0),
            _ => egui::Color32::LIGHT_GRAY,
        };

        let status_lbl = if app.current_config.langue == "Français" { "Statut Moteur :" } else { "Engine Status:" };
        ui.label(egui::RichText::new(status_lbl).strong());
        ui.colored_label(status_color, egui::RichText::new(&app.status_text).strong());

        ui.separator();

        let user_status_lbl = if app.current_config.langue == "Français" { "Votre Statut :" } else { "My Status:" };
        ui.label(egui::RichText::new(user_status_lbl).strong());

        if app.current_config.detecter_activite_utilisateur {
            let active = is_user_active_now();
            let (user_text, user_color) = if active {
                if app.current_config.langue == "Français" {
                    ("Actif (clavier/souris)", egui::Color32::from_rgb(255, 140, 0))
                } else {
                    ("Active (keyboard/mouse)", egui::Color32::from_rgb(255, 140, 0))
                }
            } else {
                if app.current_config.langue == "Français" {
                    ("Inactif", egui::Color32::from_rgb(50, 205, 50))
                } else {
                    ("Inactive", egui::Color32::from_rgb(50, 205, 50))
                }
            };
            ui.colored_label(user_color, egui::RichText::new(user_text).strong());
        } else {
            let disabled_text = if app.current_config.langue == "Français" {
                "Désactivé"
            } else {
                "Disabled"
            };
            ui.colored_label(egui::Color32::GRAY, egui::RichText::new(disabled_text).strong());
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            // Far right button (first in right_to_left layout)
            if app.active_tab == Tab::GlobalConfig {
                let next_label = if app.current_config.langue == "Français" { "Suivant ▶" } else { "Next ▶" };
                let next_btn = egui::Button::new(egui::RichText::new(next_label).color(egui::Color32::WHITE).strong())
                    .fill(egui::Color32::from_rgb(16, 110, 190))
                    .min_size(egui::vec2(100.0, 30.0));
                if ui.add(next_btn).clicked() {
                    app.active_tab = Tab::PromptEditor;
                }
            } else if app.active_tab == Tab::PromptEditor {
                let next_label = if app.current_config.langue == "Français" { "Suivant ▶" } else { "Next ▶" };
                let next_btn = egui::Button::new(egui::RichText::new(next_label).color(egui::Color32::WHITE).strong())
                    .fill(egui::Color32::from_rgb(16, 110, 190))
                    .min_size(egui::vec2(100.0, 30.0));
                if ui.add(next_btn).clicked() {
                    app.active_tab = Tab::Console;
                }
            } else if app.active_tab == Tab::Console {
                if app.is_running {
                    let stop_lbl = if app.current_config.langue == "Français" { "⏹ Arrêter l'Orchestrateur" } else { "⏹ Stop Orchestrator" };
                    let stop_btn = egui::Button::new(egui::RichText::new(stop_lbl).color(egui::Color32::WHITE).strong())
                        .fill(egui::Color32::from_rgb(175, 45, 45))
                        .min_size(egui::vec2(150.0, 30.0));
                    if ui.add(stop_btn).clicked() {
                        app.stop_orchestrator();
                        app.bus.emit_notification(NotificationEvent::Log("Orchestrator stopped".to_string()));
                        play_system_beep();
                    }

                    if app.status_color == "orange" {
                        ui.add_space(12.0);
                        let skip_lbl = if app.current_config.langue == "Français" { "⏭ Sauter l'attente" } else { "⏭ Skip Wait" };
                        let skip_btn = egui::Button::new(egui::RichText::new(skip_lbl).color(egui::Color32::WHITE).strong())
                            .fill(egui::Color32::from_rgb(210, 105, 30))
                            .min_size(egui::vec2(120.0, 30.0));
                        if ui.add(skip_btn).clicked() {
                            app.orchestrator.skip_current_wait();
                            app.bus.emit_notification(NotificationEvent::Log("Skipping wait as requested by user.".to_string()));
                        }
                    }
                } else {
                    let start_btn_label = if app.action_history.is_empty() {
                        if app.current_config.langue == "Français" { "▶ Lancer l'Orchestrateur" } else { "▶ Start Orchestrator" }
                    } else {
                        if app.current_config.langue == "Français" { "⏯ Reprendre l'Orchestrateur" } else { "⏯ Resume Orchestrator" }
                    };
                    let start_btn_color = if app.action_history.is_empty() { egui::Color32::from_rgb(34, 139, 34) } else { egui::Color32::from_rgb(16, 110, 190) };
                    let start_btn = egui::Button::new(egui::RichText::new(start_btn_label).color(egui::Color32::WHITE).strong())
                        .fill(start_btn_color)
                        .min_size(egui::vec2(150.0, 30.0));
                    if ui.add(start_btn).clicked() {
                        app.start_orchestrator();
                        app.bus.emit_notification(NotificationEvent::Log("Orchestrator started".to_string()));
                        play_system_beep();
                    }
                }
            }

            ui.add_space(12.0);

            ui.add_space(12.0);

            // Step mode toggle
            let mut sm = app.current_config.step_mode_enabled;
            if ui.checkbox(&mut sm, "Step Mode").changed() {
                app.current_config.step_mode_enabled = sm;
                app.config_repo.save_config(&app.current_config);
                app.orchestrator.step_mode.store(sm, std::sync::atomic::Ordering::Relaxed);
            }

            // Step button (only visible when running + step mode enabled)
            if app.is_running && app.current_config.step_mode_enabled {
                ui.add_space(12.0);
                let step_btn = egui::Button::new(egui::RichText::new("⏭ Step").color(egui::Color32::WHITE).strong())
                    .fill(egui::Color32::from_rgb(30, 144, 255))
                    .min_size(egui::vec2(100.0, 30.0));
                if ui.add(step_btn).clicked() {
                    app.orchestrator.step_signal.store(true, std::sync::atomic::Ordering::Relaxed);
                }
            }

            ui.add_space(12.0);

            // Undo button
            if let Ok(history) = app.orchestrator.undo_history.lock() {
                if !history.is_empty() {
                    let count = history.len();
                    let undo_btn = egui::Button::new(egui::RichText::new(format!("↩ Undo ({})", count)).color(egui::Color32::WHITE).strong())
                        .fill(egui::Color32::from_rgb(255, 165, 0))
                        .min_size(egui::vec2(120.0, 30.0));
                    if ui.add(undo_btn).clicked() {
                        drop(history);
                        if let Ok(mut history_mut) = app.orchestrator.undo_history.lock() {
                            if let Some(snapshot) = history_mut.pop() {
                                // 1. Rollback task graph state in config repo if active_task_id is present
                                if let Some(task_id) = snapshot.active_task_id {
                                    if let Some(mut graph) = app.config_repo.load_task_graph() {
                                        let tid = crate::memory::TaskId(task_id);
                                        if let Some(node) = graph.tasks.get_mut(&tid) {
                                            node.status = crate::memory::TaskStatus::Pending;
                                            node.attempts = node.attempts.saturating_sub(1);
                                        }
                                        app.config_repo.save_task_graph(Some(graph));
                                    }
                                }

                                // 2. Trigger virtual Ctrl+Z keyboard shortcut to undo action in the target application
                                let ctrl_z_combo = vec![rdev::Key::ControlLeft, rdev::Key::KeyZ];
                                let _ = app.orchestrator.controller.key_combo(&ctrl_z_combo);

                                // 3. Pop the visual action history
                                app.action_history.pop();

                                app.bus.emit_notification(NotificationEvent::Log(if app.current_config.langue == "Français" {
                                    format!("↩ Action annulée : {} (Tâche #{:?}) et Ctrl+Z envoyé.", snapshot.action_type, snapshot.active_task_id)
                                } else {
                                    format!("↩ Undid action: {} (Task #{:?}) and sent Ctrl+Z.", snapshot.action_type, snapshot.active_task_id)
                                }));
                            }
                        }
                    }
                }
            }

            let pause_label = if app.pause_mode {
                if app.current_config.langue == "Français" { "⏸ EN PAUSE" } else { "⏸ PAUSED" }
            } else {
                "⏸ Pause"
            };
            let mut pause_val = app.pause_mode;
            if ui.checkbox(&mut pause_val, pause_label).changed() {
                let pause_ref = app.get_pause_ref().clone();
                pause_ref.store(pause_val, std::sync::atomic::Ordering::Relaxed);
                app.pause_mode = pause_val;
                app.bus.emit_notification(NotificationEvent::Log(format!("Pause mode: {}", pause_val)));
                play_system_beep();
            }
        });
    });
}

#[cfg(target_os = "windows")]
#[allow(dead_code)]
extern "system" {
    fn MessageBeep(utype: u32) -> i32;
}

pub fn play_system_beep() {
    // Disabled beep sound as requested by user to keep the application and tests silent
}

#[cfg(target_os = "windows")]
pub(crate) fn is_user_active_now() -> bool {
    use windows::Win32::UI::Input::KeyboardAndMouse::{GetLastInputInfo, LASTINPUTINFO};
    use windows::Win32::System::SystemInformation::GetTickCount;

    let mut lii = LASTINPUTINFO { cbSize: std::mem::size_of::<LASTINPUTINFO>() as u32, ..Default::default() };
    unsafe {
        if GetLastInputInfo(&mut lii).as_bool() {
            let tick_count = GetTickCount();
            let idle_millis = tick_count.wrapping_sub(lii.dwTime);
            if idle_millis < 5000 {
                return true;
            }
        }
    }
    false
}

#[cfg(not(target_os = "windows"))]
pub(crate) fn is_user_active_now() -> bool {
    false
}

