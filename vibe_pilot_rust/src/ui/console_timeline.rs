use crate::app::{VibePilotApp, StructuredStep};
use crate::event_bus::NotificationEvent;
use eframe::egui;

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

    // Header selector and controls
    ui.horizontal(|ui| {
        let label_logs = "🔴 Console & Logs";
        let label_report = if app.current_config.langue == "Français" { "📄 Rapport IA" } else { "📄 AI Report" };

        ui.selectable_value(&mut app.bottom_tab, 0, label_logs);
        ui.selectable_value(&mut app.bottom_tab, 1, label_report);

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            // Auto-scroll checkbox is always visible
            let auto_scroll_lbl = if app.current_config.langue == "Français" { "Défilement auto" } else { "Auto-scroll" };
            ui.checkbox(&mut app.auto_scroll_logs, auto_scroll_lbl);

            if app.bottom_tab == 0 {
                let clear_console_label = if app.current_config.langue == "Français" { "🗑️ Effacer l'Historique" } else { "🗑️ Clear History" };
                if ui.button(clear_console_label).clicked() {
                    app.action_history.clear();
                    app.logs.clear();
                    app.structured_steps.clear();
                    app.action_textures.clear();
                }

                let copy_logs_label = app.t("btn_copy_logs");
                if ui.button(copy_logs_label).clicked() {
                    copy_logs_to_clipboard(ui.ctx(), &app.logs);
                }

                let export_gif_10_label = if app.current_config.langue == "Français" { "🎥 GIF (10 Derniers)" } else { "🎥 GIF (Last 10)" };
                if ui.button(export_gif_10_label).on_hover_text("Export the last 10 screens of execution as an animated GIF").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .set_file_name("replay_last_10.gif")
                        .add_filter("GIF Animation", &["gif"])
                        .save_file()
                    {
                        match app.orchestrator.replay_manager.export_gif(&path) {
                            Ok(()) => {
                                let success_msg = if app.current_config.langue == "Français" {
                                    "Replay exporté en GIF avec succès !"
                                } else {
                                    "Replay exported to GIF successfully!"
                                };
                                app.bus.emit_notification(NotificationEvent::Log(success_msg.to_string()));
                            }
                            Err(e) => {
                                let err_msg = format!("GIF export error: {}", e);
                                app.bus.emit_notification(NotificationEvent::Log(err_msg));
                            }
                        }
                    }
                }

                let export_gif_full_label = if app.current_config.langue == "Français" { "🎬 GIF (Session Complète)" } else { "🎬 GIF (Full Session)" };
                if ui.button(export_gif_full_label).on_hover_text("Export the entire execution session as an animated GIF").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .set_file_name("replay_full_session.gif")
                        .add_filter("GIF Animation", &["gif"])
                        .save_file()
                    {
                        match app.orchestrator.replay_manager.export_full_session_gif(&path) {
                            Ok(()) => {
                                let success_msg = if app.current_config.langue == "Français" {
                                    "Session complète exportée en GIF avec succès !"
                                } else {
                                    "Full session exported to GIF successfully!"
                                };
                                app.bus.emit_notification(NotificationEvent::Log(success_msg.to_string()));
                            }
                            Err(e) => {
                                let err_msg = format!("GIF export error: {}", e);
                                app.bus.emit_notification(NotificationEvent::Log(err_msg));
                            }
                        }
                    }
                }

                let save_all_label = if app.current_config.langue == "Français" { "📦 Tout Enregistrer" } else { "📦 Save All" };
                if ui.button(save_all_label).on_hover_text("Save all screenshots, logs, and AI report into a single folder").clicked() {
                    if let Some(path) = rfd::FileDialog::new().pick_folder() {
                        match app.orchestrator.replay_manager.export_full_session_archive(&path, &app.logs, &app.report_content) {
                            Ok(()) => {
                                let success_msg = if app.current_config.langue == "Français" {
                                    "Session complète enregistrée avec succès !"
                                } else {
                                    "Full session saved successfully!"
                                };
                                app.bus.emit_notification(NotificationEvent::Log(success_msg.to_string()));
                            }
                            Err(e) => {
                                let err_msg = format!("Save all error: {}", e);
                                app.bus.emit_notification(NotificationEvent::Log(err_msg));
                            }
                        }
                    }
                }
            } else {
                let clear_report_label = if app.current_config.langue == "Français" { "🗑️ Effacer le Rapport" } else { "🗑️ Clear Report" };
                if ui.button(clear_report_label).clicked() {
                    app.report_content.clear();
                    for step in &mut app.structured_steps {
                        step.report.clear();
                    }
                }

                let copy_report_label = app.t("btn_copy_report");
                if ui.button(copy_report_label).clicked() {
                    copy_report_to_clipboard(ui.ctx(), &app.report_content);
                }
            }
        });
    });
    ui.separator();

    let feedback_panel_height = 35.0;
    let content_height = (height - 40.0 - feedback_panel_height - 10.0).max(50.0);

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
        ("STARTUP", egui::Color32::from_rgb(128, 128, 128)),
    ].into_iter().collect();

    // Scroll container for timeline + logs/report split
    egui::ScrollArea::vertical()
        .max_width(ui.available_width())
        .max_height(content_height)
        .stick_to_bottom(app.auto_scroll_logs)
        .show(ui, |ui| {
            ui.set_min_height((content_height - 15.0).max(0.0));

            if app.structured_steps.is_empty() {
                let empty_msg = if app.current_config.langue == "Français" {
                    "Aucune activité disponible pour le moment."
                } else {
                    "No activity available yet."
                };
                ui.colored_label(egui::Color32::LIGHT_GRAY, empty_msg);
            } else {
                let steps_count = app.structured_steps.len();

                for (i, step) in app.structured_steps.iter().enumerate() {
                    let color = colors.get(step.action_type.as_str()).copied().unwrap_or(egui::Color32::from_rgb(169, 169, 169));

                    ui.horizontal(|ui| {
                        // 1. Spacer for the timeline node (width 40px)
                        let (rect_timeline, _response) = ui.allocate_exact_size(egui::vec2(40.0, 1.0), egui::Sense::hover());

                        // 2. Render logs/report block for this step
                        let logs_response = ui.vertical(|ui| {
                            let frame_color = color.linear_multiply(0.2); // Subtle matching background border
                            egui::Frame::group(ui.style())
                                .fill(ui.visuals().extreme_bg_color)
                                .stroke(egui::Stroke::new(1.0, frame_color))
                                .corner_radius(6.0)
                                .outer_margin(egui::Margin::symmetric(0, 4))
                                .inner_margin(egui::Margin::symmetric(10, 8))
                                .show(ui, |ui| {
                                    ui.set_width(ui.available_width() - 10.0);

                                    // Header of the block (Action type + details)
                                    ui.horizontal(|ui| {
                                        ui.colored_label(color, egui::RichText::new(&step.action_type).strong());
                                        if !step.tooltip.is_empty() {
                                            ui.colored_label(egui::Color32::LIGHT_GRAY, format!("- {}", step.tooltip));
                                        }
                                    });
                                    ui.separator();

                                    // Body of the block
                                    if app.bottom_tab == 0 {
                                        // Logs view
                                        if step.logs.is_empty() {
                                            ui.colored_label(egui::Color32::GRAY, "No logs for this step.");
                                        } else {
                                            let step_logs = step.logs.join("\n");
                                            ui.add(egui::Label::new(
                                                egui::RichText::new(step_logs)
                                                    .monospace()
                                                    .color(egui::Color32::from_rgb(0, 255, 0))
                                            ).wrap());
                                        }

                                        if let Some(ref bytes) = step.action_image {
                                            let texture = app.action_textures.entry(i).or_insert_with(|| {
                                                let img = image::load_from_memory(bytes).unwrap_or_else(|_| image::DynamicImage::new_rgba8(1, 1));
                                                let size = [img.width() as usize, img.height() as usize];
                                                let pixels = img.to_rgba8();
                                                let color_image = egui::ColorImage::from_rgba_unmultiplied(size, &pixels);
                                                ui.ctx().load_texture(
                                                    format!("step_crop_{}", i),
                                                    color_image,
                                                    Default::default()
                                                )
                                            });

                                            ui.add_space(6.0);
                                            ui.horizontal(|ui| {
                                                ui.image(&*texture);
                                            });
                                        }
                                    } else {
                                        // Report view
                                        if step.report.is_empty() {
                                            let no_report_msg = if app.current_config.langue == "Français" {
                                                "Aucun rapport disponible pour cette étape."
                                            } else {
                                                "No report available for this step."
                                            };
                                            ui.colored_label(egui::Color32::GRAY, no_report_msg);
                                        } else {
                                            let r = step.report.clone();
                                            ui.add(egui::Label::new(
                                                egui::RichText::new(r)
                                            ).wrap());
                                        }
                                    }
                                });
                        }).response;

                        // 3. Paint the timeline node, bracket, and connector using custom painting
                        let rect = logs_response.rect;
                        let x_timeline = rect_timeline.center().x;
                        let node_center = egui::pos2(x_timeline, rect.center().y);

                        // A. Draw vertical timeline line
                        let timeline_stroke = egui::Stroke::new(1.5, egui::Color32::from_rgb(85, 85, 85));
                        let line_start_y = if i == 0 { node_center.y } else { rect.top() - 8.0 };
                        let line_end_y = if i == steps_count - 1 { node_center.y } else { rect.bottom() + 8.0 };
                        ui.painter().line_segment(
                            [egui::pos2(x_timeline, line_start_y), egui::pos2(x_timeline, line_end_y)],
                            timeline_stroke,
                        );

                        // B. Draw circle node
                        ui.painter().circle_filled(node_center, 10.0, color);
                        ui.painter().circle_stroke(node_center, 10.0, egui::Stroke::new(1.5, egui::Color32::WHITE));
                        ui.painter().text(
                            node_center,
                            egui::Align2::CENTER_CENTER,
                            action_chars(&step.action_type),
                            egui::FontId::monospace(9.0),
                            egui::Color32::BLACK,
                        );

                        // C. Draw tooltip hover area for timeline node
                        let node_rect = egui::Rect::from_center_size(node_center, egui::vec2(20.0, 20.0));
                        if ui.rect_contains_pointer(node_rect) {
                            egui::show_tooltip_at_pointer(ui.ctx(), ui.layer_id(), ui.make_persistent_id(format!("node_tooltip_v_{}", i)), |ui| {
                                ui.label(format!("Action: {}", step.action_type));
                                if !step.tooltip.is_empty() {
                                    ui.label(format!("Details: {}", step.tooltip));
                                }
                            });
                        }

                        // D. Draw custom accolade/bracket connecting node to the logs box
                        let bracket_stroke = egui::Stroke::new(1.5, color);
                        let x_left = node_center.x + 10.0;
                        let x_right = rect.left() - 2.0;
                        let x_mid = (x_left + x_right) * 0.5;
                        let y_top = rect.top() + 4.0;
                        let y_bottom = rect.bottom() - 4.0;

                        let tip_offset = ((y_bottom - y_top) * 0.15).min(6.0).max(2.0);

                        // Top horizontal segment
                        ui.painter().line_segment([egui::pos2(x_right, y_top), egui::pos2(x_mid, y_top)], bracket_stroke);
                        // Top vertical segment
                        ui.painter().line_segment([egui::pos2(x_mid, y_top), egui::pos2(x_mid, node_center.y - tip_offset)], bracket_stroke);
                        // Upper diagonal of the tip pointing towards the node circle
                        ui.painter().line_segment([egui::pos2(x_mid, node_center.y - tip_offset), egui::pos2(x_left, node_center.y)], bracket_stroke);
                        // Lower diagonal of the tip pointing back to the spine
                        ui.painter().line_segment([egui::pos2(x_left, node_center.y), egui::pos2(x_mid, node_center.y + tip_offset)], bracket_stroke);
                        // Bottom vertical segment
                        ui.painter().line_segment([egui::pos2(x_mid, node_center.y + tip_offset), egui::pos2(x_mid, y_bottom)], bracket_stroke);
                        // Bottom horizontal segment
                        ui.painter().line_segment([egui::pos2(x_mid, y_bottom), egui::pos2(x_right, y_bottom)], bracket_stroke);
                    });
                    ui.add_space(8.0);
                }
            }
        });

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
                if let Ok(mut guard) = app.accumulated_user_feedback.lock() {
                    if guard.is_empty() {
                        *guard = msg.clone();
                    } else {
                        *guard = format!("{}; {}", *guard, msg);
                    }
                }

                let log_msg = if app.current_config.langue == "Français" {
                    format!("✍️ Indication ajoutée : \"{}\"", msg)
                } else {
                    format!("✍️ Hint added: \"{}\"", msg)
                };
                app.bus.emit_notification(NotificationEvent::Log(log_msg));
                app.user_feedback_input.clear();
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

pub fn copy_logs_to_clipboard(ctx: &egui::Context, logs: &[String]) {
    let all_logs = logs.join("\n");
    ctx.copy_text(all_logs);
}

pub fn copy_report_to_clipboard(ctx: &egui::Context, report: &str) {
    ctx.copy_text(report.to_string());
}
