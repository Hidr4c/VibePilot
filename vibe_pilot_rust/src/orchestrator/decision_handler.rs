use tokio::time::{sleep, Duration};
use crate::event_bus::{NotificationEvent};
use crate::llm_client::LlmResponse;
use crate::peripheral_controller::ActionPayload;
use crate::orchestrator::VibePilotOrchestrator;
use crate::orchestrator::helpers::{parse_key_name};
use crate::commands::Command;

impl VibePilotOrchestrator {
    /// Handles an LLM decision and executes the corresponding action.
    pub(crate) async fn handle_decision(&self, res: LlmResponse) -> Result<(), String> {
        self.bus.emit_notification(NotificationEvent::UpdateStatus {
            text: res.status_display.clone(),
            color: "green".to_string(),
        });

        // Intercept action for manual verification if auto-validate is disabled
        let config = self.config_repo.load_config();
        
        let offsets = if let Ok(lock) = self.active_workspace_offsets.lock() {
            if !lock.is_empty() {
                lock.clone()
            } else {
                self.get_target_offsets(&config)
            }
        } else {
            self.get_target_offsets(&config)
        };

        // Delegate manual confirmation logic to helper
        if !self.check_action_confirmation(&res, &config, &offsets).await? {
            return Ok(());
        }

        match res.action.as_str() {
            "SUCCESS" => {
                self.bus.emit_notification(NotificationEvent::Log("Task completed successfully!".to_string()));
                return Err("STOP_SUCCESS".to_string());
            },
            "FAIL" => {
                self.bus.emit_notification(NotificationEvent::Log("LLM reported failure.".to_string()));
                sleep(Duration::from_secs(res.wait_seconds as u64)).await;
            },
            "CLICK_AND_TYPE" => {
                self.bus.emit_notification(NotificationEvent::AppendAction {
                    action: "Click & Type".to_string(),
                    display: res.text_to_type.clone(),
                });
                let payload = ActionPayload {
                    action: "CLICK_AND_TYPE".to_string(),
                    relative_click_position: res.relative_click_position.clone(),
                    text_to_type: res.text_to_type.clone(),
                    scroll_value: res.scroll_value,
                    wait_seconds: res.wait_seconds,
                };
                let x_rel = res.relative_click_position.first().copied().unwrap_or(0.5);
                let y_rel = res.relative_click_position.get(1).copied().unwrap_or(0.5);
                let (abs_x, abs_y) = self.controller.compute_absolute_coordinates(x_rel, y_rel, &offsets);
                if !offsets.is_empty() {
                    let target = &offsets[0];
                    self.bus.emit_notification(NotificationEvent::Log(format!(
                        "🤖 Target window: '{}' [left={}, top={}, width={}, height={}]. Relative target: [x={:.3}, y={:.3}] -> absolute screen: [x={}, y={}]",
                        target.titre, target.left, target.top, target.width, target.height, x_rel, y_rel, abs_x, abs_y
                    )));
                } else {
                    self.bus.emit_notification(NotificationEvent::Log(format!(
                        "🖥️ Target: Full screen. Relative target: [x={:.3}, y={:.3}] -> absolute screen: [x={}, y={}]",
                        x_rel, y_rel, abs_x, abs_y
                    )));
                }

                // Wait if user is active
                self.wait_if_user_active().await;
 
                let verify_str = if config.verifier_placement_souris {
                    if config.langue == "Français" {
                        "Vérification activée (délai de 1.5s avant clic)"
                    } else {
                        "Verification enabled (1.5s delay before click)"
                    }
                } else {
                    if config.langue == "Français" {
                        "Vérification désactivée (délai standard de 100ms)"
                    } else {
                        "Verification disabled (100ms standard delay)"
                    }
                };

                let text_str = if res.text_to_type.is_empty() {
                    if config.langue == "Français" { "Pas de texte à saisir" } else { "No text to type" }.to_string()
                } else {
                    format!("Text: '{}'", res.text_to_type)
                };

                self.bus.emit_notification(NotificationEvent::Log(format!(
                    "🖱️ Action execution: CLICK_AND_TYPE | Mouse verification: {} | {}",
                    verify_str, text_str
                )));

                if let Ok(mut t) = self.last_simulated_input_time.lock() {
                    *t = std::time::Instant::now();
                }
                self.controller.execute_action(&payload, &offsets, config.verifier_placement_souris);
                sleep(Duration::from_secs(res.wait_seconds as u64)).await;
            },
            "SCROLL" => {
                self.bus.emit_notification(NotificationEvent::AppendAction {
                    action: "Scroll".to_string(),
                    display: format!("Value: {} Mode: {}", res.scroll_value, res.scroll_mode),
                });
                self.wait_if_user_active().await;

                if let Ok(mut t) = self.last_simulated_input_time.lock() {
                    *t = std::time::Instant::now();
                }

                if res.scroll_mode == "quick" {
                    let dir = if res.scroll_value > 0 { "up" } else { "down" };
                    let intensity = if res.scroll_value.abs() > 20 { "high" } else { "medium" };
                    let _ = self.controller.quick_scroll_vertical(dir, intensity);
                } else if res.scroll_mode == "smooth" {
                    let _ = self.controller.smooth_scroll_vertical(res.scroll_value, 10, 300);
                } else {
                    let _ = self.controller.scroll_vertical(res.scroll_value);
                }
                sleep(Duration::from_secs(res.wait_seconds as u64)).await;
            },
            "RIGHT_CLICK" => {
                self.bus.emit_notification(NotificationEvent::AppendAction {
                    action: "Right Click".to_string(),
                    display: format!("Pos: {:?}", res.relative_click_position),
                });
                self.wait_if_user_active().await;
                if let Ok(mut t) = self.last_simulated_input_time.lock() {
                    *t = std::time::Instant::now();
                }
                let x_rel = res.relative_click_position.first().copied().unwrap_or(0.5);
                let y_rel = res.relative_click_position.get(1).copied().unwrap_or(0.5);
                let (abs_x, abs_y) = self.controller.compute_absolute_coordinates(x_rel, y_rel, &offsets);
                #[cfg(target_os = "windows")]
                {
                    if !cfg!(test) {
                        crate::peripheral_controller::win32_move_mouse_absolute(abs_x, abs_y);
                    }
                }
                #[cfg(not(target_os = "windows"))]
                {
                    let _ = (abs_x, abs_y);
                }
                std::thread::sleep(std::time::Duration::from_millis(100));
                let _ = self.controller.right_click();
                sleep(Duration::from_secs(res.wait_seconds as u64)).await;
            },
            "DOUBLE_CLICK" => {
                self.bus.emit_notification(NotificationEvent::AppendAction {
                    action: "Double Click".to_string(),
                    display: format!("Pos: {:?}", res.relative_click_position),
                });
                self.wait_if_user_active().await;
                if let Ok(mut t) = self.last_simulated_input_time.lock() {
                    *t = std::time::Instant::now();
                }
                let x_rel = res.relative_click_position.first().copied().unwrap_or(0.5);
                let y_rel = res.relative_click_position.get(1).copied().unwrap_or(0.5);
                let (abs_x, abs_y) = self.controller.compute_absolute_coordinates(x_rel, y_rel, &offsets);
                #[cfg(target_os = "windows")]
                {
                    if !cfg!(test) {
                        crate::peripheral_controller::win32_move_mouse_absolute(abs_x, abs_y);
                    }
                }
                #[cfg(not(target_os = "windows"))]
                {
                    let _ = (abs_x, abs_y);
                }
                std::thread::sleep(std::time::Duration::from_millis(100));
                let _ = self.controller.double_click();
                sleep(Duration::from_secs(res.wait_seconds as u64)).await;
            },
            "MIDDLE_CLICK" => {
                self.bus.emit_notification(NotificationEvent::AppendAction {
                    action: "Middle Click".to_string(),
                    display: format!("Pos: {:?}", res.relative_click_position),
                });
                self.wait_if_user_active().await;
                if let Ok(mut t) = self.last_simulated_input_time.lock() {
                    *t = std::time::Instant::now();
                }
                let x_rel = res.relative_click_position.first().copied().unwrap_or(0.5);
                let y_rel = res.relative_click_position.get(1).copied().unwrap_or(0.5);
                let (abs_x, abs_y) = self.controller.compute_absolute_coordinates(x_rel, y_rel, &offsets);
                #[cfg(target_os = "windows")]
                {
                    if !cfg!(test) {
                        crate::peripheral_controller::win32_move_mouse_absolute(abs_x, abs_y);
                    }
                }
                #[cfg(not(target_os = "windows"))]
                {
                    let _ = (abs_x, abs_y);
                }
                std::thread::sleep(std::time::Duration::from_millis(100));
                let _ = self.controller.middle_click();
                sleep(Duration::from_secs(res.wait_seconds as u64)).await;
            },
            "MOUSE_MOVE_RELATIVE" => {
                self.bus.emit_notification(NotificationEvent::AppendAction {
                    action: "Mouse Move Rel".to_string(),
                    display: format!("Deltas: {:?}", res.relative_move),
                });
                self.wait_if_user_active().await;
                if let Ok(mut t) = self.last_simulated_input_time.lock() {
                    *t = std::time::Instant::now();
                }
                let dx_rel = res.relative_move.first().copied().unwrap_or(0.0);
                let dy_rel = res.relative_move.get(1).copied().unwrap_or(0.0);
                let screen_w = offsets.first().map(|o| o.width).unwrap_or(1920) as f64;
                let screen_h = offsets.first().map(|o| o.height).unwrap_or(1080) as f64;
                let dx = (dx_rel * screen_w) as i32;
                let dy = (dy_rel * screen_h) as i32;
                let _ = self.controller.mouse_move_relative(dx, dy);
                sleep(Duration::from_secs(res.wait_seconds as u64)).await;
            },
            "KEY_COMBO" => {
                self.bus.emit_notification(NotificationEvent::AppendAction {
                    action: "Key Combo".to_string(),
                    display: res.keys_to_press.join("+"),
                });
                self.wait_if_user_active().await;
                if let Ok(mut t) = self.last_simulated_input_time.lock() {
                    *t = std::time::Instant::now();
                }
                let mut rdev_keys = Vec::new();
                for key_str in &res.keys_to_press {
                    if let Some(key) = parse_key_name(key_str) {
                        rdev_keys.push(key);
                    }
                }
                if !rdev_keys.is_empty() {
                    let _ = self.controller.key_combo(&rdev_keys);
                }
                sleep(Duration::from_secs(res.wait_seconds as u64)).await;
            },
            "CLIPBOARD" => {
                self.bus.emit_notification(NotificationEvent::AppendAction {
                    action: "Clipboard".to_string(),
                    display: res.clipboard_op.clone(),
                });
                self.wait_if_user_active().await;
                if let Ok(mut t) = self.last_simulated_input_time.lock() {
                    *t = std::time::Instant::now();
                }
                match res.clipboard_op.as_str() {
                    "copy" => { let _ = self.controller.copy_selected(); },
                    "paste" => { let _ = self.controller.paste(); },
                    "cut" => { let _ = self.controller.cut_selected(); },
                    "select_all" => { let _ = self.controller.select_all(); },
                    "get_text" => {
                        match self.controller.get_clipboard_text() {
                            Ok(text) => {
                                self.bus.emit_notification(NotificationEvent::Log(format!("📋 [Clipboard] Read text: \"{}\"", if text.len() > 60 { format!("{}...", &text[..57]) } else { text.clone() })));
                            }
                            Err(e) => {
                                self.bus.emit_notification(NotificationEvent::Log(format!("⚠️ Clipboard read error: {}", e)));
                            }
                        }
                    }
                    _ => {}
                }
                sleep(Duration::from_secs(res.wait_seconds as u64)).await;
            },
            "DRAG_DROP" => {
                self.bus.emit_notification(NotificationEvent::AppendAction {
                    action: "Drag & Drop".to_string(),
                    display: format!("From {:?} to {:?}", res.drag_from, res.drag_to),
                });
                self.wait_if_user_active().await;
                if let Ok(mut t) = self.last_simulated_input_time.lock() {
                    *t = std::time::Instant::now();
                }
                let fx_rel = res.drag_from.first().copied().unwrap_or(0.5);
                let fy_rel = res.drag_from.get(1).copied().unwrap_or(0.5);
                let tx_rel = res.drag_to.first().copied().unwrap_or(0.5);
                let ty_rel = res.drag_to.get(1).copied().unwrap_or(0.5);
                let (from_x, from_y) = self.controller.compute_absolute_coordinates(fx_rel, fy_rel, &offsets);
                let (to_x, to_y) = self.controller.compute_absolute_coordinates(tx_rel, ty_rel, &offsets);
                let _ = self.controller.drag_and_drop((from_x, from_y), (to_x, to_y), rdev::Button::Left);
                sleep(Duration::from_secs(res.wait_seconds as u64)).await;
            },
            "WAIT" => {
                let duration = res.duration_secs.unwrap_or(res.wait_seconds as u32);
                let cond_str = res.condition.as_deref();
                let condition = crate::llm_client::WaitCondition::from_str(cond_str, duration);
                
                self.bus.emit_notification(NotificationEvent::AppendAction {
                    action: "Wait".to_string(),
                    display: format!("Duree: {}s | Cond: {:?}", duration, condition),
                });
                
                self.bus.emit_notification(NotificationEvent::Log(format!(
                    "⏳ Smart wait started: {}s, condition: {:?}", duration, condition
                )));

                // Reset the skip wait trigger before starting
                self.skip_wait.store(false, std::sync::atomic::Ordering::Relaxed);

                // Run the wait
                let success = self.wait_manager.wait_for(
                    condition.clone(),
                    duration,
                    self.capturer.clone(),
                    self.bus.clone(),
                    self.skip_wait.clone(),
                ).await;

                if success {
                    self.bus.emit_notification(NotificationEvent::Log(
                        "⏳ Smart wait finished successfully.".to_string()
                    ));
                } else {
                    self.bus.emit_notification(NotificationEvent::Log(
                        "⚠️ Smart wait timed out or failed to meet condition.".to_string()
                    ));
                }
            },
            "SHORTCUT" => {
                self.bus.emit_notification(NotificationEvent::AppendAction {
                    action: "Shortcut".to_string(),
                    display: res.shortcut_name.clone(),
                });
                self.wait_if_user_active().await;
                if let Ok(mut t) = self.last_simulated_input_time.lock() {
                    *t = std::time::Instant::now();
                }
                if let Some(combo) = crate::peripheral::shortcuts::resolve_shortcut(&res.shortcut_name) {
                    let mut rdev_keys = Vec::new();
                    if combo.ctrl { rdev_keys.push(rdev::Key::ControlLeft); }
                    if combo.shift { rdev_keys.push(rdev::Key::ShiftLeft); }
                    if combo.alt { rdev_keys.push(rdev::Key::Alt); }
                    if combo.win { rdev_keys.push(rdev::Key::MetaLeft); }
                    if let Some(k) = combo.key { rdev_keys.push(k); }
                    if !rdev_keys.is_empty() {
                        let _ = self.controller.key_combo(&rdev_keys);
                    }
                } else {
                    self.bus.emit_notification(NotificationEvent::Log(format!("⚠️ Unknown shortcut name: {}", res.shortcut_name)));
                }
                sleep(Duration::from_secs(res.wait_seconds as u64)).await;
            },
            "TYPE_WITH_DELAY" => {
                self.bus.emit_notification(NotificationEvent::AppendAction {
                    action: "Type w/ Delay".to_string(),
                    display: format!("'{}' ({}..{}ms)", res.text_to_type, res.min_delay_ms, res.max_delay_ms),
                });
                self.wait_if_user_active().await;
                if let Ok(mut t) = self.last_simulated_input_time.lock() {
                    *t = std::time::Instant::now();
                }
                let min_delay = if res.min_delay_ms == 0 { 10 } else { res.min_delay_ms };
                let max_delay = if res.max_delay_ms == 0 { 50 } else { res.max_delay_ms.max(min_delay) };
                
                let cmd = crate::commands::TypeWithDelayCommand {
                    text: res.text_to_type.clone(),
                    min_delay_ms: min_delay,
                    max_delay_ms: max_delay,
                };
                let mut ctx = crate::commands::ExecutionContext::new();
                let _ = cmd.execute(&mut ctx).await;
                sleep(Duration::from_secs(res.wait_seconds as u64)).await;
            },
            "SMOOTH_SCROLL" => {
                self.bus.emit_notification(NotificationEvent::AppendAction {
                    action: "Smooth Scroll".to_string(),
                    display: format!("delta: {} duration: {}ms", res.scroll_value, res.duration_ms),
                });
                self.wait_if_user_active().await;
                if let Ok(mut t) = self.last_simulated_input_time.lock() {
                    *t = std::time::Instant::now();
                }
                let duration = if res.duration_ms == 0 { 300 } else { res.duration_ms };
                
                let cmd = crate::commands::MouseSmoothScrollCommand {
                    delta_total: res.scroll_value,
                    duration_ms: duration,
                };
                let mut ctx = crate::commands::ExecutionContext::new();
                let _ = cmd.execute(&mut ctx).await;
                sleep(Duration::from_secs(res.wait_seconds as u64)).await;
            },
            "KEY_HOLD" => {
                let key_str = res.keys_to_press.first().cloned().unwrap_or_else(|| res.text_to_type.clone());
                self.bus.emit_notification(NotificationEvent::AppendAction {
                    action: "Key Hold".to_string(),
                    display: key_str.clone(),
                });
                self.wait_if_user_active().await;
                if let Ok(mut t) = self.last_simulated_input_time.lock() {
                    *t = std::time::Instant::now();
                }
                if let Some(key) = parse_key_name(&key_str) {
                    let cmd = crate::commands::KeyHoldCommand { key };
                    let mut ctx = crate::commands::ExecutionContext::new();
                    let _ = cmd.execute(&mut ctx).await;
                } else {
                    self.bus.emit_notification(NotificationEvent::Log(format!("⚠️ Cannot parse key for KeyHold: {}", key_str)));
                }
                sleep(Duration::from_secs(res.wait_seconds as u64)).await;
            },
            "KEY_RELEASE" => {
                let key_str = res.keys_to_press.first().cloned().unwrap_or_else(|| res.text_to_type.clone());
                self.bus.emit_notification(NotificationEvent::AppendAction {
                    action: "Key Release".to_string(),
                    display: key_str.clone(),
                });
                self.wait_if_user_active().await;
                if let Ok(mut t) = self.last_simulated_input_time.lock() {
                    *t = std::time::Instant::now();
                }
                if let Some(key) = parse_key_name(&key_str) {
                    let cmd = crate::commands::KeyReleaseCommand { key };
                    let mut ctx = crate::commands::ExecutionContext::new();
                    let _ = cmd.execute(&mut ctx).await;
                } else {
                    self.bus.emit_notification(NotificationEvent::Log(format!("⚠️ Cannot parse key for KeyRelease: {}", key_str)));
                }
                sleep(Duration::from_secs(res.wait_seconds as u64)).await;
            },
            _ => {
                self.bus.emit_notification(NotificationEvent::Log(format!("Unknown action: {}", res.action)));
            }
        }

        Ok(())
    }
}
