use crate::app::VibePilotApp;
use crate::event_bus::EventType;
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
            ui.painter().text(egui::pos2(x, y), egui::Align2::CENTER_CENTER, &action_chars(action), egui::FontId::monospace(9.0), egui::Color32::BLACK);
            
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
            let title = if app.current_config.langue == "Français" { "Timeline" } else { "Timeline" };
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
                &action_chars(action),
                egui::FontId::monospace(8.0),
                egui::Color32::BLACK,
            );

            // Draw abbreviation label next to the circle
            ui.painter().text(
                egui::pos2(x + 12.0, y),
                egui::Align2::LEFT_CENTER,
                &action_chars(action),
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
        "ERROR" => "ERR".to_string(),
        _ => "?".to_string(),
    }
}

// ============================================================
// CONSOLE LOGS
// ============================================================
pub fn render_activity_console(ui: &mut egui::Ui, app: &mut VibePilotApp, height: f32) {
    ui.group(|ui| {
        ui.horizontal(|ui| {
            let label_logs = if app.current_config.langue == "Français" { "🔴 Console & Logs" } else { "🔴 Console & Logs" };
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

pub fn render_console_and_timeline_split(ui: &mut egui::Ui, app: &mut VibePilotApp, _default_height: f32) {
    // Top Legend (just a clean single line at the top)
    ui.horizontal(|ui| {
        let legend = if app.current_config.langue == "Français" {
            "Légende: T: Réflexion  W: IA Occupée  CD: Cooldown  S: Défiler  C: Saisie  ★: Succès  X: Échec  SL: Veille  ST: Écran Statique  ERR: Erreur"
        } else {
            "Legend: T: Thinking  W: Busy  CD: Stabilizing  S: Scroll  C: Type  ★: Success  X: Fail  SL: Sleep  ST: Static  ERR: Error"
        };
        ui.label(egui::RichText::new(legend).small().color(egui::Color32::GRAY));
    });
    ui.separator();

    // Use all available remaining height of the window!
    let height = ui.available_height() - 10.0;

    // Horizontal split: Left is Vertical Timeline, Right is Console Logs/AI Report
    ui.horizontal(|ui| {
        // Timeline Column (width: 35px)
        ui.vertical(|ui| {
            ui.set_width(35.0);
            ui.set_height(height);
            
            // Draw vertical timeline nodes
            let size = egui::Vec2::new(35.0, height);
            let (rect, _response) = ui.allocate_exact_size(size, egui::Sense::hover());
            
            let x_center = rect.min.x + rect.width() * 0.5;
            
            // Draw vertical line down the middle
            ui.painter().line_segment(
                [egui::pos2(x_center, rect.min.y), egui::pos2(x_center, rect.max.y)],
                (1.5, egui::Color32::from_rgb(85, 85, 85)),
            );

            let actions = &app.action_history;
            let spacing = 32.0;
            let max_nodes = (rect.height() / spacing).floor() as usize;
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

            // Render nodes (oldest at the top, newest at the bottom)
            for (i, (action, tooltip)) in display.iter().enumerate() {
                let x = x_center;
                let y = rect.min.y + 16.0 + (i as f32 * spacing);
                let color = colors.get(action.as_str()).copied().unwrap_or(egui::Color32::from_rgb(169, 169, 169));
                
                // Draw circle node
                ui.painter().circle_filled(egui::pos2(x, y), 8.0, color);
                ui.painter().text(
                    egui::pos2(x, y),
                    egui::Align2::CENTER_CENTER,
                    &action_chars(action),
                    egui::FontId::monospace(8.0),
                    egui::Color32::BLACK,
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

        ui.separator();

        // Right side: Logs / AI Report Selection
        ui.vertical(|ui| {
            ui.set_height(height);
            
            ui.horizontal(|ui| {
                let label_logs = if app.current_config.langue == "Français" { "🔴 Console & Logs" } else { "🔴 Console & Logs" };
                let label_report = if app.current_config.langue == "Français" { "📄 Rapport IA" } else { "📄 AI Report" };

                ui.selectable_value(&mut app.bottom_tab, 0, label_logs);
                ui.selectable_value(&mut app.bottom_tab, 1, label_report);

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if app.bottom_tab == 0 {
                        let clear_console_label = if app.current_config.langue == "Français" { "🗑️ Effacer l'Historique" } else { "🗑️ Clear History" };
                        if ui.button(clear_console_label).clicked() {
                            app.action_history.clear();
                            app.logs.clear();
                        }

                        let auto_scroll_lbl = if app.current_config.langue == "Français" { "Défilement auto" } else { "Auto-scroll" };
                        ui.checkbox(&mut app.auto_scroll_logs, auto_scroll_lbl);
                    } else {
                        let clear_report_label = if app.current_config.langue == "Français" { "🗑️ Effacer le Rapport" } else { "🗑️ Clear Report" };
                        if ui.button(clear_report_label).clicked() {
                            app.report_content.clear();
                        }
                    }
                });
            });
            ui.separator();

            let feedback_panel_height = 35.0;
            let content_height = height - 40.0 - feedback_panel_height - 10.0;

            if app.bottom_tab == 0 {
                let mut all_logs = app.logs.join("\n");
                egui::ScrollArea::vertical()
                    .max_width(ui.available_width())
                    .max_height(content_height)
                    .stick_to_bottom(app.auto_scroll_logs)
                    .show(ui, |ui| {
                        ui.set_min_height(content_height - 15.0);
                        ui.add_sized(
                            egui::vec2(ui.available_width(), content_height - 15.0),
                            egui::TextEdit::multiline(&mut all_logs)
                                .font(egui::TextStyle::Monospace)
                                .text_color(egui::Color32::from_rgb(0, 255, 0))
                        );
                    });
            } else {
                egui::ScrollArea::vertical()
                    .max_width(ui.available_width())
                    .max_height(content_height)
                    .stick_to_bottom(app.auto_scroll_logs)
                    .show(ui, |ui| {
                        ui.set_min_height(content_height - 15.0);
                        if app.report_content.is_empty() {
                            let empty_msg = if app.current_config.langue == "Français" {
                                "Aucun rapport disponible pour le moment."
                            } else {
                                "No report available yet."
                            };
                            ui.colored_label(egui::Color32::LIGHT_GRAY, empty_msg);
                        } else {
                            ui.add_sized(
                                egui::vec2(ui.available_width(), content_height - 15.0),
                                egui::TextEdit::multiline(&mut app.report_content)
                            );
                        }
                    });
            }

            ui.separator();
            ui.horizontal(|ui| {
                let lbl = if app.current_config.langue == "Français" { "Message pour l'IA :" } else { "Message to AI:" };
                ui.label(egui::RichText::new(lbl).strong());

                let text_hint = if app.current_config.langue == "Français" {
                    "Ex: Tu as oublié de scroller sur le côté..."
                } else {
                    "e.g., You forgot to scroll to the side..."
                };

                let input_width = ui.available_width() - 120.0;
                let response = ui.add(egui::TextEdit::singleline(&mut app.user_feedback_input)
                    .hint_text(text_hint)
                    .desired_width(input_width));

                if ui.button("🪄").on_hover_text(app.t("tip_optimize_field")).clicked() {
                    app.optimize_prompt_field("user_feedback");
                }

                let btn_lbl = if app.current_config.langue == "Français" { "Envoyer" } else { "Send" };
                if ui.button(btn_lbl).clicked() || (response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))) {
                    let msg = app.user_feedback_input.trim().to_string();
                    if !msg.is_empty() {
                        let mut guard = app.accumulated_user_feedback.lock().unwrap();
                        if guard.is_empty() {
                            *guard = msg.clone();
                        } else {
                            *guard = format!("{}; {}", *guard, msg);
                        }

                        let log_msg = if app.current_config.langue == "Français" {
                            format!("✍️ Indication ajoutée : \"{}\"", msg)
                        } else {
                            format!("✍️ Hint added: \"{}\"", msg)
                        };
                        app.bus.emit(EventType::Log(log_msg));
                        app.user_feedback_input.clear();
                    }
                }
            });
        });
    });
}


// ============================================================
// CONTROLS & SETTINGS (FOOTER)
// ============================================================
pub fn render_status_controls(ui: &mut egui::Ui, app: &mut VibePilotApp) {
    ui.horizontal(|ui| {
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
            if app.is_running {
                let stop_lbl = if app.current_config.langue == "Français" { "⏹ Arrêter l'Orchestrateur" } else { "⏹ Stop Orchestrator" };
                let stop_btn = egui::Button::new(egui::RichText::new(stop_lbl).color(egui::Color32::WHITE).strong())
                    .fill(egui::Color32::from_rgb(175, 45, 45))
                    .min_size(egui::vec2(150.0, 30.0));
                if ui.add(stop_btn).clicked() {
                    app.stop_orchestrator();
                    app.bus.emit(EventType::Log("Orchestrator stopped".to_string()));
                    play_system_beep();
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
                    app.bus.emit(EventType::Log("Orchestrator started".to_string()));
                    play_system_beep();
                }
            }

            ui.add_space(12.0);

            let pause_label = if app.pause_mode {
                if app.current_config.langue == "Français" { "⏸ EN PAUSE" } else { "⏸ PAUSED" }
            } else {
                if app.current_config.langue == "Français" { "⏸ Pause" } else { "⏸ Pause" }
            };
            let mut pause_val = app.pause_mode;
            if ui.checkbox(&mut pause_val, pause_label).changed() {
                let pause_ref = app.get_pause_ref().clone();
                pause_ref.store(pause_val, std::sync::atomic::Ordering::Relaxed);
                app.pause_mode = pause_val;
                app.bus.emit(EventType::Log(format!("Pause mode: {}", pause_val)));
                play_system_beep();
            }
        });
    });
}

#[cfg(target_os = "windows")]
extern "system" {
    fn MessageBeep(utype: u32) -> i32;
}

pub fn play_system_beep() {
    #[cfg(target_os = "windows")]
    unsafe {
        let _ = MessageBeep(0);
    }
}

#[cfg(target_os = "windows")]
pub(crate) fn is_user_active_now() -> bool {
    use windows::Win32::UI::Input::KeyboardAndMouse::{GetLastInputInfo, LASTINPUTINFO};
    use windows::Win32::System::SystemInformation::GetTickCount;

    let mut lii = LASTINPUTINFO::default();
    lii.cbSize = std::mem::size_of::<LASTINPUTINFO>() as u32;
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
