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
                "Légende: T: Réflexion  W: IA Occupée  CD: Cooldown  S: Défiler  C: Saisie  ★: Succès  X: Échec  SL: Veille  ST: Écran Statique"
            } else {
                "Legend: T: Thinking  W: Busy  CD: Stabilizing  S: Scroll  C: Type  ★: Success  X: Fail  SL: Sleep  ST: Static"
            };
            ui.label(legend);
        });
    });
}

fn action_chars(action: &str) -> String {
    match action {
        "THINK" => "T".to_string(), "WAIT" => "W".to_string(), "COOL" => "CD".to_string(),
        "SCROLL" => "S".to_string(), "CLICK_AND_TYPE" => "C".to_string(), "SUCCESS" => "*".to_string(),
        "FAIL" => "X".to_string(), "SLEEP" => "SL".to_string(), "STATIC" => "ST".to_string(),
        _ => "?".to_string(),
    }
}

// ============================================================
// CONSOLE LOGS
// ============================================================
pub fn render_activity_console(ui: &mut egui::Ui, app: &mut VibePilotApp, height: f32) {
    ui.group(|ui| {
        ui.horizontal(|ui| {
            ui.label(app.t("frame_logs"));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let clear_console_label = if app.current_config.langue == "Français" { "🗑️ Effacer la Console" } else { "🗑️ Clear Console" };
                if ui.button(clear_console_label).clicked() {
                    app.logs.clear();
                }
            });
        });
        ui.separator();

        egui::ScrollArea::vertical()
            .max_width(ui.available_width())
            .max_height(height)
            .show(ui, |ui| {
                ui.set_min_height(height - 15.0);
                for log in &app.logs {
                    ui.colored_label(egui::Color32::from_rgb(0, 255, 0), log);
                }
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

        let status_lbl = if app.current_config.langue == "Français" { "Statut :" } else { "Status:" };
        ui.label(egui::RichText::new(status_lbl).strong());
        ui.colored_label(status_color, egui::RichText::new(&app.status_text).strong());

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
