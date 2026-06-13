use tokio::time::{sleep, Duration};
use crate::event_bus::{NotificationEvent, CommandEvent, QueryEvent};
use crate::llm_client::LlmResponse;
use crate::orchestrator::VibePilotOrchestrator;
use crate::orchestrator::helpers::{find_executable_for_window};
use crate::screen_capture::WindowAnchorResult;

impl VibePilotOrchestrator {
    /// Checks if the action needs manual confirmation and waits for user input if required.
    /// Returns Ok(true) if the action is approved (or doesn't need approval),
    /// and Ok(false) if the action was cancelled or timed out.
    pub(crate) async fn check_action_confirmation(
        &self,
        res: &LlmResponse,
        config: &crate::config::SavedConfig,
        offsets: &[crate::peripheral_controller::ScreenOffset],
    ) -> Result<bool, String> {
        let auto_val = config.auto_validate;
        let auto_val_dangerous = config.auto_validate_dangerous;

        let is_sensitive_clipboard = res.action == "CLIPBOARD" && res.clipboard_op == "get_text" && !config.allow_clipboard_read_without_confirm;
        let is_dangerous_shortcut = res.action == "SHORTCUT" && crate::config::is_dangerous_shortcut(&res.shortcut_name);
        let is_dangerous = if res.action == "CLICK_AND_TYPE" {
            crate::config::contains_dangerous_command(&res.text_to_type)
        } else {
            false
        } || is_sensitive_clipboard || is_dangerous_shortcut;

        let needs_confirmation = if is_dangerous {
            !auto_val_dangerous
        } else {
            !auto_val
        };

        let requires_approval = res.action == "CLICK_AND_TYPE"
            || res.action == "SCROLL"
            || res.action == "CLIPBOARD"
            || res.action == "KEY_COMBO"
            || res.action == "DRAG_DROP"
            || res.action == "RIGHT_CLICK"
            || res.action == "DOUBLE_CLICK"
            || res.action == "MIDDLE_CLICK"
            || res.action == "MOUSE_MOVE_RELATIVE"
            || res.action == "SHORTCUT"
            || res.action == "TYPE_WITH_DELAY"
            || res.action == "SMOOTH_SCROLL"
            || res.action == "KEY_HOLD"
            || res.action == "KEY_RELEASE";

        if needs_confirmation && requires_approval {
            *self.state.lock().unwrap() = crate::orchestrator::OrchestratorState::AwaitingApproval;
            struct ApprovalGuard<'a> {
                state: &'a std::sync::Arc<std::sync::Mutex<crate::orchestrator::OrchestratorState>>,
            }
            impl<'a> Drop for ApprovalGuard<'a> {
                fn drop(&mut self) {
                    let mut s = self.state.lock().unwrap();
                    if *s == crate::orchestrator::OrchestratorState::AwaitingApproval {
                        *s = crate::orchestrator::OrchestratorState::Running;
                    }
                }
            }
            let _approval_guard = ApprovalGuard { state: &self.state };

            if res.action == "CLICK_AND_TYPE" || res.action == "RIGHT_CLICK" || res.action == "DOUBLE_CLICK" || res.action == "MIDDLE_CLICK" {
                let x_rel = res.relative_click_position.first().copied().unwrap_or(0.5);
                let y_rel = res.relative_click_position.get(1).copied().unwrap_or(0.5);
                let (abs_x, abs_y) = self.controller.compute_absolute_coordinates(x_rel, y_rel, offsets);
                if let Ok(mut t) = self.last_simulated_input_time.lock() {
                    *t = std::time::Instant::now();
                }
                #[cfg(target_os = "windows")]
                unsafe {
                    if !cfg!(test) {
                        let _scope = crate::screen_capture::DpiAwarenessScope::enter_per_monitor_v2();
                        let _ = windows::Win32::UI::WindowsAndMessaging::SetCursorPos(abs_x, abs_y);
                    }
                }
                #[cfg(not(target_os = "windows"))]
                let _ = (abs_x, abs_y);
            } else if res.action == "DRAG_DROP" {
                let fx_rel = res.drag_from.first().copied().unwrap_or(0.5);
                let fy_rel = res.drag_from.get(1).copied().unwrap_or(0.5);
                let (abs_x, abs_y) = self.controller.compute_absolute_coordinates(fx_rel, fy_rel, offsets);
                if let Ok(mut t) = self.last_simulated_input_time.lock() {
                    *t = std::time::Instant::now();
                }
                #[cfg(target_os = "windows")]
                unsafe {
                    if !cfg!(test) {
                        let _scope = crate::screen_capture::DpiAwarenessScope::enter_per_monitor_v2();
                        let _ = windows::Win32::UI::WindowsAndMessaging::SetCursorPos(abs_x, abs_y);
                    }
                }
                #[cfg(not(target_os = "windows"))]
                let _ = (abs_x, abs_y);
            }

            self.bus.emit_command(CommandEvent::ShowActionConfirmation {
                action: res.action.clone(),
                text: if res.action == "CLICK_AND_TYPE" {
                    res.text_to_type.clone()
                } else if res.action == "CLIPBOARD" {
                    format!("op: {}", res.clipboard_op)
                } else if res.action == "KEY_COMBO" {
                    res.keys_to_press.join("+")
                } else if res.action == "DRAG_DROP" {
                    format!("from {:?} to {:?}", res.drag_from, res.drag_to)
                } else if res.action == "MOUSE_MOVE_RELATIVE" {
                    format!("move {:?}", res.relative_move)
                } else if res.action == "SHORTCUT" {
                    format!("shortcut: {}", res.shortcut_name)
                } else if res.action == "TYPE_WITH_DELAY" {
                    format!("type with delay: '{}'", res.text_to_type)
                } else if res.action == "KEY_HOLD" {
                    format!("hold key: {}", res.keys_to_press.first().cloned().unwrap_or_else(|| res.text_to_type.clone()))
                } else if res.action == "KEY_RELEASE" {
                    format!("release key: {}", res.keys_to_press.first().cloned().unwrap_or_else(|| res.text_to_type.clone()))
                } else {
                    format!("coordinates: {:?}", res.relative_click_position)
                },
                scroll: if res.action == "SCROLL" || res.action == "SMOOTH_SCROLL" { res.scroll_value } else { 0 },
            });

            self.bus.emit_notification(NotificationEvent::Log("Waiting for user approval of AI action...".to_string()));

            // Confirmation loop with timeout (60 seconds)
            let confirmation_timeout = Duration::from_secs(60);
            let confirmation_start = tokio::time::Instant::now();

            loop {
                let status = self.bus.emit_query(QueryEvent::GetActionConfirmationStatus);
                if status == "approved" {
                    if let Ok(mut t) = self.last_simulated_input_time.lock() {
                        *t = std::time::Instant::now();
                    }
                    break;
                } else if status == "cancelled" {
                    self.bus.emit_notification(NotificationEvent::Log("AI action cancelled by user.".to_string()));
                    self.bus.emit_notification(NotificationEvent::ClearActionConfirmation);
                    return Ok(false);
                }

                if confirmation_start.elapsed() > confirmation_timeout {
                    self.bus.emit_notification(NotificationEvent::Log(
                        "⚠️ Action confirmation timed out (60s). Cancelling action.".to_string()
                    ));
                    self.bus.emit_notification(NotificationEvent::ClearActionConfirmation);
                    return Ok(false);
                }

                if self.is_paused() {
                    sleep(Duration::from_millis(500)).await;
                    continue;
                }

                sleep(Duration::from_millis(200)).await;
            }
            self.bus.emit_notification(NotificationEvent::ClearActionConfirmation);
        }

        Ok(true)
    }

    /// Wait if the user is active (typing or moving mouse).
    pub(crate) async fn wait_if_user_active(&self) {
        loop {
            let config = self.config_repo.load_config();
            if !config.detecter_activite_utilisateur {
                break;
            }
            if self.is_paused() {
                sleep(Duration::from_millis(500)).await;
                continue;
            }

            let last_sim = self.last_simulated_input_time.lock()
                .map(|t| *t)
                .unwrap_or_else(|_| std::time::Instant::now());
            let last_sim_elapsed = last_sim.elapsed().as_millis();
            let mut is_active = false;
            #[cfg(target_os = "windows")]
            {
                use windows::Win32::UI::Input::KeyboardAndMouse::{GetLastInputInfo, LASTINPUTINFO};
                use windows::Win32::System::SystemInformation::GetTickCount;

                let mut lii = LASTINPUTINFO { cbSize: std::mem::size_of::<LASTINPUTINFO>() as u32, ..Default::default() };
                unsafe {
                    if GetLastInputInfo(&mut lii).as_bool() {
                        let tick_count = GetTickCount();
                        let idle_millis = tick_count.wrapping_sub(lii.dwTime);
                        if idle_millis < 5000 {
                            let diff = (last_sim_elapsed as i64 - idle_millis as i64).abs();
                            if diff > 300 {
                                is_active = true;
                            }
                        }
                    }
                }
            }

            if is_active {
                let status_msg = if config.langue == "Français" {
                    "Utilisateur actif - En attente..."
                } else {
                    "User active - Waiting..."
                };
                self.bus.emit_notification(NotificationEvent::UpdateStatus {
                    text: status_msg.to_string(),
                    color: "orange".to_string(),
                });
                sleep(Duration::from_secs(1)).await;
            } else {
                break;
            }
        }
    }

    /// Attempts to recover a lost target window.
    pub(crate) async fn attempt_window_recovery(&self, target_title: &str) -> bool {
        // Step 1: Try re-anchoring one more time after a short delay
        sleep(Duration::from_secs(1)).await;
        self.capturer.refresh_windows();
        let anchor = self.capturer.ensure_window_foreground(target_title);
        if anchor != WindowAnchorResult::WindowNotFound {
            return true;
        }

        // Step 2: Try to find and launch the executable
        if let Some(exe_name) = find_executable_for_window(target_title) {
            self.bus.emit_notification(NotificationEvent::Log(format!(
                "🔄 Attempting to relaunch: '{}'...", exe_name
            )));

            #[cfg(target_os = "windows")]
            {
                if cfg!(test) {
                    self.bus.emit_notification(NotificationEvent::Log(format!(
                        "🔄 [TEST] Skipping real launch of: '{}'", exe_name
                    )));
                    return true;
                }
                use std::os::windows::process::CommandExt;
                const CREATE_NO_WINDOW: u32 = 0x08000000;
                let result = std::process::Command::new("cmd")
                    .args(["/C", "start", "", &exe_name])
                    .creation_flags(CREATE_NO_WINDOW)
                    .spawn();

                if let Err(e) = result {
                    self.bus.emit_notification(NotificationEvent::Log(format!(
                        "❌ Failed to launch '{}': {}", exe_name, e
                    )));
                    return false;
                }
            }

            // Wait up to 10 seconds for the window to reappear
            for i in 0..10 {
                sleep(Duration::from_secs(1)).await;
                self.capturer.refresh_windows();
                if self.capturer.is_target_visible(&[target_title.to_string()]) {
                    self.bus.emit_notification(NotificationEvent::Log(format!(
                        "✅ Window '{}' reappeared after {}s.", target_title, i + 1
                    )));
                    // Give it a moment to fully render
                    sleep(Duration::from_secs(1)).await;
                    let _ = self.capturer.ensure_window_foreground(target_title);
                    return true;
                }
            }

            self.bus.emit_notification(NotificationEvent::Log(format!(
                "⏰ Timeout: window '{}' did not reappear after 10s.", target_title
            )));
        } else {
            self.bus.emit_notification(NotificationEvent::Log(format!(
                "❓ No known executable mapping for window title '{}'.", target_title
            )));
        }

        false
    }
}
