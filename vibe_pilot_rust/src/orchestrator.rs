//! VibePilot Orchestrator - Core logic for visual automation.
//!
//! Handles the decision loop, interaction with LLM, and coordinates
//! between the screen capturer and peripheral controller.
//!
//! # Safety
//!
//! This module executes physical actions (mouse clicks, keyboard input)
//! based on LLM decisions. Always ensure the user has supervisory control
//! over the automation loop.

use crate::config::ConfigRepository;
use crate::event_bus::{EventBus, EventType};
use crate::llm_client::{LlmClient, LlmResponse};
use crate::peripheral_controller::{ActionPayload, PeripheralController, ScreenOffset};
use crate::screen_capture::ScreenCapturer;
use std::sync::Arc;
use tokio::time::{sleep, Duration};

/// Maximum number of iterations before the loop exits automatically.
const MAX_ITERATIONS: u64 = 1000;

/// Global timeout in seconds for the entire orchestration session.
const MAX_SESSION_DURATION_SECS: u64 = 3600; // 1 hour

/// Maximum consecutive repetitions of the same action before alerting.
const MAX_ACTION_REPETITIONS: u32 = 3;

pub struct VibePilotOrchestrator {
    config_repo: Arc<ConfigRepository>,
    llm_client: LlmClient,
    capturer: Arc<ScreenCapturer>,
    controller: PeripheralController,
    bus: EventBus,
    last_simulated_input_time: std::sync::Mutex<std::time::Instant>,
}

impl VibePilotOrchestrator {
    /// Creates a new orchestrator instance.
    pub fn new(config_repo: Arc<ConfigRepository>, bus: EventBus) -> Self {
        let capturer = Arc::new(ScreenCapturer::new());
        let controller = PeripheralController::new(capturer.clone());
        Self {
            config_repo,
            llm_client: LlmClient::new(),
            capturer,
            controller,
            bus,
            last_simulated_input_time: std::sync::Mutex::new(std::time::Instant::now()),
        }
    }

    /// The main execution loop with safety limits.
    ///
    /// This loop runs until stopped by the user, until the LLM returns
    /// a `SUCCESS` action, or until safety limits are reached:
    /// - Maximum iterations: 1000
    /// - Maximum session duration: 1 hour
    pub async fn run_loop(&self) -> Result<(), String> {
        self.bus.emit(EventType::Log("Starting orchestration loop...".to_string()));

        let mut iteration_count = 0u64;
        let session_start = std::time::Instant::now();
        let mut last_action_key = String::new();
        let mut action_repetition_count = 0u32;
        let mut escalate_to_desktop = false;

        loop {
            // Safety check: iteration limit
            iteration_count += 1;
            if iteration_count > MAX_ITERATIONS {
                self.bus.emit(EventType::Log(format!(
                    "Stopping: reached maximum iterations ({})", MAX_ITERATIONS
                )));
                return Err(format!("Maximum iterations ({}) reached", MAX_ITERATIONS));
            }

            // Safety check: session timeout
            let elapsed = session_start.elapsed().as_secs();
            if elapsed > MAX_SESSION_DURATION_SECS {
                self.bus.emit(EventType::Log(format!(
                    "Stopping: session timeout ({:.0}s > {:.0}s)",
                    elapsed, MAX_SESSION_DURATION_SECS
                )));
                return Err(format!("Session timeout after {:.0} seconds", elapsed));
            }

            // Hot-reload configuration dynamically
            let config = self.config_repo.load_config();

            // 0. Check if orchestrator has been stopped
            if !self.is_running() {
                self.bus.emit(EventType::Log("Orchestration loop stopped.".to_string()));
                break;
            }

            // 1. Check for forced pause
            if self.is_paused() {
                self.bus.emit(EventType::UpdateStatus {
                    text: "PAUSED".to_string(),
                    color: "red".to_string(),
                });
                sleep(Duration::from_secs(1)).await;
                continue;
            }
 
            // 1.5. We do NOT wait for user activity here anymore. 
            // This allows the LLM query to run in the background while the user is active.
            // We only block/wait immediately before executing a peripheral action (CLICK, SCROLL).



            // 2. Verify target window visibility
            if !self.capturer.is_target_visible(&config.fenetres_surveillees) {
                self.bus.emit(EventType::UpdateStatus {
                    text: "Waiting for target window...".to_string(),
                    color: "orange".to_string(),
                });
                sleep(Duration::from_secs(2)).await;
                continue;
            }

            // 2.5. Check GPU Busy Status for Local URLs
            if is_local_url(&config.url_api) {
                if let Some(gpu_usage) = get_gpu_utilization() {
                    if gpu_usage >= 90 {
                        self.bus.emit(EventType::UpdateStatus {
                            text: format!("GPU busy ({}%)...", gpu_usage),
                            color: "blue".to_string(),
                        });
                        sleep(Duration::from_secs(2)).await;
                        continue;
                    }
                }
            }

            // 3. Capture and Decide
            self.bus.emit(EventType::UpdateStatus {
                text: "Analyzing...".to_string(),
                color: "blue".to_string(),
            });
            self.bus.emit(EventType::AppendAction {
                action: "THINK".to_string(),
                display: format!("Sending request to {} (URL: {})...", config.nom_modele, config.url_api),
            });

            let screenshot = if escalate_to_desktop {
                self.bus.emit(EventType::Log("Escalating capture scope: capturing full desktop...".to_string()));
                self.capturer.capture_desktop()
            } else {
                let target_window_capture = if !config.fenetres_surveillees.is_empty()
                    && !self.controller.is_desktop_title(&config.fenetres_surveillees[0])
                {
                    self.capturer.capture_window_by_title(&config.fenetres_surveillees[0])
                } else {
                    None
                };
                target_window_capture.or_else(|| self.capturer.capture_desktop())
            };

            if let Some(img) = screenshot {
                self.bus.emit(EventType::Log(if config.langue == "Français" {
                    "🔍 Analyse du nouvel état de l'écran pour évaluer l'action précédente..."
                } else {
                    "🔍 Analyzing new screen state to verify previous action..."
                }.to_string()));

                let (target_url, target_model, auth_mode, auth_key, auth_login, auth_pass, timeout) = if config.utiliser_moteur_vision_dedie {
                    (
                        &config.url_api_vision,
                        &config.nom_modele_vision,
                        &config.auth_mode_vision,
                        &config.auth_api_key_vision,
                        &config.auth_login_vision,
                        &config.auth_password_vision,
                        config.request_timeout_secs_vision,
                    )
                } else {
                    (
                        &config.url_api,
                        &config.nom_modele,
                        &config.auth_mode,
                        &config.auth_api_key,
                        &config.auth_login,
                        &config.auth_password,
                        config.request_timeout_secs,
                    )
                };

                self.bus.emit(EventType::Log(format!(
                    "📡 Sending Vision LLM Request (Image + Text) | Model: '{}' | Endpoint: '{}'",
                    target_model, target_url
                )));

                let user_feedback = self.bus.emit_query(EventType::GetUserFeedback);
                if !user_feedback.is_empty() {
                    self.bus.emit(EventType::Log(format!(
                        "💬 [Feedback User] prise en compte de l'indication : \"{}\"",
                        user_feedback
                    )));
                }

                let response = self.llm_client.execute_decision(
                    &img,
                    &config.contexte,
                    &config.objectif,
                    &config.task,
                    &config.directives,
                    &user_feedback,
                    target_url,
                    target_model,
                    auth_mode,
                    auth_key,
                    auth_login,
                    auth_pass,
                    timeout,
                ).await;

                match response {
                    Ok(res) => {
                        self.bus.emit(EventType::Log(format!(
                            "📥 Received LLM Response | Action: '{}' | Status: '{}'",
                            res.action, res.status_display
                        )));
                        // Check for action repetition
                        let action_key = format!("{}:{:.2}:{:.2}:{}",
                            res.action,
                            res.relative_click_position.get(0).copied().unwrap_or(0.0),
                            res.relative_click_position.get(1).copied().unwrap_or(0.0),
                            res.text_to_type.len()
                        );

                        if let Some(ref r) = res.report {
                            if !r.is_empty() {
                                self.bus.emit(EventType::AppendReport(r.clone()));
                            }
                        }

                        let is_fail_or_wait = res.action == "FAIL" || res.action == "WAIT";
                        if action_key == last_action_key {
                            action_repetition_count += 1;
                            if action_repetition_count >= MAX_ACTION_REPETITIONS {
                                self.bus.emit(EventType::Log(format!(
                                    "⚠️ Warning: Same action repeated {} times. The previous click/type may not have had the expected effect, or the UI is not responding.",
                                    action_repetition_count
                                )));
                                action_repetition_count = 0;
                            }
                            if !is_fail_or_wait {
                                self.bus.emit(EventType::Log("Action repeated. Triggering desktop capture escalation...".to_string()));
                                escalate_to_desktop = true;
                            }
                        } else {
                            last_action_key = action_key.clone();
                            action_repetition_count = 0;
                            if res.action == "FAIL" {
                                self.bus.emit(EventType::Log("Explicit AI failure action. Triggering desktop capture escalation...".to_string()));
                                escalate_to_desktop = true;
                            } else if res.action != "WAIT" {
                                escalate_to_desktop = false;
                            }
                        }

                        match self.handle_decision(res).await {
                            Ok(()) => {},
                            Err(ref e) if e == "STOP_SUCCESS" => break,
                            Err(e) => {
                                self.bus.emit(EventType::Log(format!("Error: {}", e)));
                                self.bus.emit(EventType::AppendAction {
                                    action: "ERROR".to_string(),
                                    display: format!("Execution error: {}", e),
                                });
                                sleep(Duration::from_secs(5)).await;
                            }
                        }
                    }
                    Err(e) => {
                        self.bus.emit(EventType::Log(format!("LLM error: {}", e)));
                        self.bus.emit(EventType::AppendAction {
                            action: "ERROR".to_string(),
                            display: format!("LLM error: {}", e),
                        });
                        sleep(Duration::from_secs(5)).await;
                    }
                }
            } else {
                self.bus.emit(EventType::Log("Failed to capture screen".to_string()));
                self.bus.emit(EventType::AppendAction {
                    action: "ERROR".to_string(),
                    display: "Failed to capture screen".to_string(),
                });
                sleep(Duration::from_secs(2)).await;
            }

            sleep(Duration::from_millis(500)).await;
        }

        self.bus.emit(EventType::Log(format!(
            "Loop ended after {} iterations in {:.0}s",
            iteration_count,
            session_start.elapsed().as_secs()
        )));

        Ok(())
    }

    /// Handles an LLM decision and executes the corresponding action.
    async fn handle_decision(&self, res: LlmResponse) -> Result<(), String> {
        self.bus.emit(EventType::UpdateStatus {
            text: res.status_display.clone(),
            color: "green".to_string(),
        });

        // Intercept action for manual verification if auto-validate is disabled
        let config = self.config_repo.load_config();
        let auto_val = config.auto_validate;
        let auto_val_dangerous = config.auto_validate_dangerous;

        let is_dangerous = if res.action == "CLICK_AND_TYPE" {
            crate::config::contains_dangerous_command(&res.text_to_type)
        } else {
            false
        };

        let needs_confirmation = if is_dangerous {
            !auto_val_dangerous
        } else {
            !auto_val
        };

        let offsets = self.get_target_offsets(&config);

        if needs_confirmation && (res.action == "CLICK_AND_TYPE" || res.action == "SCROLL") {
            if res.action == "CLICK_AND_TYPE" {
                let x_rel = res.relative_click_position.get(0).copied().unwrap_or(0.5);
                let y_rel = res.relative_click_position.get(1).copied().unwrap_or(0.5);
                let (abs_x, abs_y) = self.controller.compute_absolute_coordinates(x_rel, y_rel, &offsets);
                *self.last_simulated_input_time.lock().unwrap() = std::time::Instant::now();
                unsafe {
                    let _ = windows::Win32::UI::WindowsAndMessaging::SetCursorPos(abs_x, abs_y);
                }
            }

            self.bus.emit(EventType::ShowActionConfirmation {
                action: res.action.clone(),
                text: if res.action == "CLICK_AND_TYPE" { res.text_to_type.clone() } else { String::new() },
                scroll: if res.action == "SCROLL" { res.scroll_value } else { 0 },
            });

            self.bus.emit(EventType::Log("Waiting for user approval of AI action...".to_string()));

            // Confirmation loop with timeout (60 seconds)
            let confirmation_timeout = Duration::from_secs(60);
            let confirmation_start = std::time::Instant::now();

            loop {
                let status = self.bus.emit_query(EventType::GetActionConfirmationStatus);
                if status == "approved" {
                    *self.last_simulated_input_time.lock().unwrap() = std::time::Instant::now();
                    break;
                } else if status == "cancelled" {
                    self.bus.emit(EventType::Log("AI action cancelled by user.".to_string()));
                    self.bus.emit(EventType::ClearActionConfirmation);
                    return Ok(());
                }

                if confirmation_start.elapsed() > confirmation_timeout {
                    self.bus.emit(EventType::Log(
                        "⚠️ Action confirmation timed out (60s). Cancelling action.".to_string()
                    ));
                    self.bus.emit(EventType::ClearActionConfirmation);
                    return Ok(());
                }

                if self.is_paused() {
                    sleep(Duration::from_millis(500)).await;
                    continue;
                }

                sleep(Duration::from_millis(200)).await;
            }
            self.bus.emit(EventType::ClearActionConfirmation);
        }

        match res.action.as_str() {
            "SUCCESS" => {
                self.bus.emit(EventType::Log("Task completed successfully!".to_string()));
                return Err("STOP_SUCCESS".to_string());
            },
            "FAIL" => {
                self.bus.emit(EventType::Log("LLM reported failure.".to_string()));
                sleep(Duration::from_secs(res.wait_seconds as u64)).await;
            },
            "CLICK_AND_TYPE" => {
                self.bus.emit(EventType::AppendAction {
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
                let x_rel = res.relative_click_position.get(0).copied().unwrap_or(0.5);
                let y_rel = res.relative_click_position.get(1).copied().unwrap_or(0.5);
                let (abs_x, abs_y) = self.controller.compute_absolute_coordinates(x_rel, y_rel, &offsets);
                if !offsets.is_empty() {
                    let target = &offsets[0];
                    self.bus.emit(EventType::Log(format!(
                        "🤖 Target window: '{}' [left={}, top={}, width={}, height={}]. Relative target: [x={:.3}, y={:.3}] -> absolute screen: [x={}, y={}]",
                        target.titre, target.left, target.top, target.width, target.height, x_rel, y_rel, abs_x, abs_y
                    )));
                } else {
                    self.bus.emit(EventType::Log(format!(
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

                self.bus.emit(EventType::Log(format!(
                    "🖱️ Action execution: CLICK_AND_TYPE | Mouse verification: {} | {}",
                    verify_str, text_str
                )));

                *self.last_simulated_input_time.lock().unwrap() = std::time::Instant::now();
                self.controller.execute_action(&payload, &offsets, config.verifier_placement_souris);
                sleep(Duration::from_secs(res.wait_seconds as u64)).await;
            },
            "SCROLL" => {
                self.bus.emit(EventType::AppendAction {
                    action: "Scroll".to_string(),
                    display: format!("Value: {}", res.scroll_value),
                });
                let payload = ActionPayload {
                    action: "SCROLL".to_string(),
                    relative_click_position: res.relative_click_position.clone(),
                    text_to_type: String::new(),
                    scroll_value: res.scroll_value,
                    wait_seconds: res.wait_seconds,
                };

                // Wait if user is active
                self.wait_if_user_active().await;

                *self.last_simulated_input_time.lock().unwrap() = std::time::Instant::now();
                self.controller.execute_action(&payload, &offsets, false);
                sleep(Duration::from_secs(res.wait_seconds as u64)).await;
            },
            "WAIT" => {
                sleep(Duration::from_secs(res.wait_seconds as u64)).await;
            },
            _ => {
                self.bus.emit(EventType::Log(format!("Unknown action: {}", res.action)));
            }
        }

        Ok(())
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

            let last_sim = *self.last_simulated_input_time.lock().unwrap();
            let last_sim_elapsed = last_sim.elapsed().as_millis();
            let mut is_active = false;
            #[cfg(target_os = "windows")]
            {
                use windows::Win32::UI::Input::KeyboardAndMouse::{GetLastInputInfo, LASTINPUTINFO};
                use windows::Win32::System::SystemInformation::GetTickCount;

                let mut lii = LASTINPUTINFO::default();
                lii.cbSize = std::mem::size_of::<LASTINPUTINFO>() as u32;
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
                self.bus.emit(EventType::UpdateStatus {
                    text: status_msg.to_string(),
                    color: "orange".to_string(),
                });
                sleep(Duration::from_secs(1)).await;
            } else {
                break;
            }
        }
    }

    /// Checks if the orchestrator is in pause mode.
    pub fn is_paused(&self) -> bool {
        self.bus.emit_query(EventType::GetModePauseForcee) == "true"
    }

    /// Checks if the orchestrator has running status enabled.
    pub fn is_running(&self) -> bool {
        self.bus.emit_query(EventType::GetOrchestratorRunning) == "true"
    }

    /// Sets pause mode (for testing/configuration).
    pub fn set_pause_mode(&self, paused: bool) {
        self.bus.register_query(
            EventType::GetModePauseForcee,
            Box::new(move |_| {
                if paused { "true".to_string() } else { "false".to_string() }
            }),
        );
    }

    /// Refreshes the list of available windows.
    pub fn refresh_windows(&self) {
        self.capturer.refresh_windows();
    }

    /// Returns a list of available window titles for targeting.
    pub fn get_available_windows(&self) -> Vec<String> {
        let mut wins = vec![crate::config::ALL_SCREENS_KEY.to_string()];
        for win in self.capturer.get_windows() {
            if !win.title.is_empty() && !wins.contains(&win.title) {
                wins.push(win.title.clone());
            }
        }
        wins
    }

    /// Resolve screen offsets based on currently monitored target window.
    pub(crate) fn get_target_offsets(&self, config: &crate::config::SavedConfig) -> Vec<ScreenOffset> {
        let targets = &config.fenetres_surveillees;
        if targets.is_empty() {
            return self.capturer.get_monitors()
                .into_iter()
                .map(ScreenOffset::from_screen_info)
                .collect();
        }
        
        let first_target = &targets[0];
        if self.controller.is_desktop_title(first_target) {
            return self.capturer.get_monitors()
                .into_iter()
                .map(ScreenOffset::from_screen_info)
                .collect();
        }

        let title_lower = first_target.to_lowercase();
        let wins = self.capturer.get_windows();
        for win in wins {
            if win.title.to_lowercase().contains(&title_lower) || title_lower.contains(&win.title.to_lowercase()) {
                return vec![ScreenOffset {
                    titre: win.title.clone(),
                    left: win.bbox.left,
                    top: win.bbox.top,
                    width: win.bbox.width,
                    height: win.bbox.height,
                    y_offset: win.bbox.top,
                }];
            }
        }

        self.capturer.get_monitors()
            .into_iter()
            .map(ScreenOffset::from_screen_info)
            .collect()
    }
}

/// Helper function to check if the target API URL is local.
pub(crate) fn is_local_url(url: &str) -> bool {
    url.contains("localhost") || url.contains("127.0.0.1") || url.contains("::1")
}

/// Helper function to query local GPU utilization on Windows/Linux using nvidia-smi.
fn get_gpu_utilization() -> Option<u32> {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        let output = std::process::Command::new("nvidia-smi")
            .args(&["--query-gpu=utilization.gpu", "--format=csv,noheader,nounits"])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .ok()?;
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            stdout.trim().parse::<u32>().ok()
        } else {
            None
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        let output = std::process::Command::new("nvidia-smi")
            .args(&["--query-gpu=utilization.gpu", "--format=csv,noheader,nounits"])
            .output()
            .ok()?;
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            stdout.trim().parse::<u32>().ok()
        } else {
            None
        }
    }
}

