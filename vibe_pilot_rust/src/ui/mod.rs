pub mod components;
pub mod global_config;
pub mod prompt_editor;
pub mod setup;
pub mod quick_start;

use crate::app::{VibePilotApp, Tab};
use crate::event_bus::EventType;
use eframe::egui;

use components::{render_console_and_timeline_split, render_status_controls};
use global_config::render_config_tab;
use prompt_editor::render_prompt_editor_tab;
use setup::render_setup_tab;
use quick_start::render_help_tab;

pub fn render_main_window(ctx: &egui::Context, app: &mut VibePilotApp) {
    // Apply dark theme if enabled
    if app.current_config.theme_sombre {
        ctx.style_mut(|style| {
            style.visuals = egui::Visuals::dark();
        });
    } else {
        ctx.style_mut(|style| {
            style.visuals = egui::Visuals::light();
        });
    }

    // === FOOTER PANEL (Always visible at the bottom) ===
    egui::TopBottomPanel::bottom("status_footer_panel")
        .frame(egui::Frame::NONE.outer_margin(egui::Margin::symmetric(10, 6)))
        .show(ctx, |ui| {
            render_status_controls(ui, app);
        });

    // === CENTRAL PANEL (Unified scrollable main section) ===
    egui::CentralPanel::default().show(ctx, |ui| {
        // --- Header bar ---
        ui.horizontal(|ui| {
            ui.heading("VibePilot");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(crate::version::get_version_info());
            });
        });
        ui.separator();

        // --- Tab Selection Headers (Setup is last) ---
        ui.horizontal(|ui| {
            let tabs = [
                (Tab::GlobalConfig, app.t("tab_config")),
                (Tab::PromptEditor, app.t("tab_prompts")),
                (Tab::Help, app.t("tab_quickstart")),
                (Tab::Setup, app.t("tab_setup")),
                (Tab::Console, app.t("tab_console")),
            ];
            for (tab_type, tab_label) in tabs {
                let is_active = app.active_tab == tab_type;
                if ui.selectable_label(is_active, &tab_label).clicked() {
                    app.active_tab = tab_type;
                }
            }
        });
        ui.separator();

        if app.active_tab == Tab::Console {
            render_console_and_timeline_split(ui, app, 400.0);
        } else {
            // Unified ScrollArea covering all the middle contents (active tab, timeline, console)
            egui::ScrollArea::vertical()
                .max_width(ui.available_width())
                .show(ui, |ui| {
                    match app.active_tab {
                        Tab::GlobalConfig => render_config_tab(ui, app),
                        Tab::PromptEditor => render_prompt_editor_tab(ui, app),
                        Tab::Help => render_help_tab(ui, app),
                        Tab::Setup => render_setup_tab(ui, app),
                        _ => {}
                    }
                });
        }
    });

    // Draw confirmation modals if active
    render_confirmations(ctx, app);
}

fn render_confirmations(ctx: &egui::Context, app: &mut VibePilotApp) {
    if let Some(profile_to_delete) = app.show_delete_confirm.clone() {
        egui::Window::new("Confirm Deletion")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .show(ctx, |ui| {
                ui.label(format!("Are you sure you want to delete profile '{}'?", profile_to_delete));
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button("Yes, Delete").clicked() {
                        app.config_repo.delete_profile(&profile_to_delete);
                        app.bus.emit(EventType::Log(format!("Profile '{}' deleted", profile_to_delete)));
                        if app.selected_profile == profile_to_delete {
                            app.selected_profile = app.get_first_available_profile_name();
                            app.current_config.dernier_profil = app.selected_profile.clone();
                        }
                        app.show_delete_confirm = None;
                    }
                    if ui.button("Cancel").clicked() {
                        app.show_delete_confirm = None;
                    }
                });
            });
    }

    if let Some(old_name) = app.show_rename_profile.clone() {
        let title = if app.current_config.langue == "Français" { "Renommer le profil" } else { "Rename Profile" };
        egui::Window::new(title)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .show(ctx, |ui| {
                let lbl_desc = if app.current_config.langue == "Français" {
                    format!("Entrez le nouveau nom pour le profil '{}' :", old_name)
                } else {
                    format!("Enter new name for profile '{}':", old_name)
                };
                ui.label(lbl_desc);
                ui.add_space(4.0);
                ui.text_edit_singleline(&mut app.rename_profile_new_name);
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    let btn_ok = if app.current_config.langue == "Français" { "Valider" } else { "OK" };
                    let btn_cancel = if app.current_config.langue == "Français" { "Annuler" } else { "Cancel" };
                    if ui.button(btn_ok).clicked() {
                        let new_name = app.rename_profile_new_name.trim().to_string();
                        if !new_name.is_empty() && new_name != old_name {
                            if let Some(mut cfg) = app.config_repo.load_profile(&old_name) {
                                cfg.dernier_profil = new_name.clone();
                                if app.config_repo.save_profile(&new_name, &cfg) {
                                    app.config_repo.delete_profile(&old_name);
                                    app.selected_profile = new_name.clone();
                                    app.current_config.dernier_profil = new_name.clone();
                                    app.bus.emit(EventType::Log(format!("Profile '{}' renamed to '{}'", old_name, new_name)));
                                }
                            }
                        }
                        app.show_rename_profile = None;
                        app.rename_profile_new_name = String::new();
                    }
                    if ui.button(btn_cancel).clicked() {
                        app.show_rename_profile = None;
                        app.rename_profile_new_name = String::new();
                    }
                });
            });
    }

    if app.show_reset_confirm {
        egui::Window::new("Confirm Reset")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .show(ctx, |ui| {
                ui.label("Are you sure you want to reset all configuration to default?");
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button("Yes, Reset").clicked() {
                        app.reset_all_settings();
                        ctx.set_pixels_per_point(1.0);
                        app.show_reset_confirm = false;
                    }
                    if ui.button("Cancel").clicked() {
                        app.show_reset_confirm = false;
                    }
                });
            });
    }

    if let Some(pending_action) = app.pending_action.clone() {
        let is_dangerous = !pending_action.text_to_type.is_empty() 
            && crate::config::contains_dangerous_command(&pending_action.text_to_type);

        let window_title = if is_dangerous {
            "🛑 DANGEROUS AI Action - CONFIRMATION REQUIRED"
        } else {
            "⚠️ Confirm AI Action"
        };

        egui::Window::new(window_title)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .show(ctx, |ui| {
                if is_dangerous {
                    ui.colored_label(egui::Color32::LIGHT_RED, "🚨 WARNING: This action contains potentially dangerous command keywords (e.g. rm, format, sudo). Check very carefully before executing!");
                    ui.add_space(6.0);
                } else {
                    ui.label(egui::RichText::new("The AI has proposed to execute the following peripheral action:").strong());
                }
                ui.add_space(8.0);
                
                ui.group(|ui| {
                    ui.label(format!("Action type: {}", pending_action.action));
                    if !pending_action.text_to_type.is_empty() {
                        if is_dangerous {
                            ui.horizontal(|ui| {
                                ui.label("Text to type: ");
                                ui.colored_label(egui::Color32::LIGHT_RED, &pending_action.text_to_type);
                            });
                        } else {
                            ui.label(format!("Text to type: {}", pending_action.text_to_type));
                        }
                    }
                    if pending_action.scroll_value != 0 {
                        ui.label(format!("Scroll value: {}", pending_action.scroll_value));
                    }
                });
                
                ui.add_space(10.0);
                ui.label("Do you want to authorize and execute this action?");
                ui.add_space(8.0);
                
                ui.horizontal(|ui| {
                    if ui.add(egui::Button::new(egui::RichText::new("✅ Execute").color(egui::Color32::WHITE).strong()).fill(egui::Color32::from_rgb(34, 139, 34))).clicked() {
                        if let Ok(mut status) = app.action_confirmation_status.lock() {
                            *status = "approved".to_string();
                        }
                    }
                    if ui.add(egui::Button::new(egui::RichText::new("❌ Cancel").color(egui::Color32::WHITE).strong()).fill(egui::Color32::from_rgb(178, 34, 34))).clicked() {
                        if let Ok(mut status) = app.action_confirmation_status.lock() {
                            *status = "cancelled".to_string();
                        }
                    }
                });
            });
    }

    if let Some(profile_name) = app.show_profile_ready_popup.clone() {
        let title = app.t("lbl_profile_ready");
        egui::Window::new(title)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .show(ctx, |ui| {
                let msg = app.t("msg_profile_ready").replace("{}", &profile_name);
                ui.label(msg);
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button("OK").clicked() {
                        app.show_profile_ready_popup = None;
                    }
                });
            });
    }
}
