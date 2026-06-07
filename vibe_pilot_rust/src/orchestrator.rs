//! VibePilot Orchestrator - Core logic for visual automation.
//!
//! Handles the decision loop, interaction with LLM, and coordinates 
//! between the screen capturer and peripheral controller.

use crate::config::ConfigRepository;
use crate::event_bus::{EventBus, EventType};
use crate::llm_client::{LlmClient, LlmResponse};
use crate::peripheral_controller::{ActionPayload, PeripheralController, ScreenOffset};
use crate::screen_capture::ScreenCapturer;
use std::sync::Arc;
use tokio::time::{sleep, Duration};

pub struct VibePilotOrchestrator {
    config_repo: Arc<ConfigRepository>,
    llm_client: LlmClient,
    capturer: Arc<ScreenCapturer>,
    controller: PeripheralController,
    bus: EventBus,
}

impl VibePilotOrchestrator {
    pub fn new(config_repo: Arc<ConfigRepository>, bus: EventBus) -> Self {
        let capturer = Arc::new(ScreenCapturer::new());
        let controller = PeripheralController::new(capturer.clone());
        Self {
            config_repo,
            llm_client: LlmClient::new(),
            capturer,
            controller,
            bus,
        }
    }

    /// The main execution loop.
    pub async fn run_loop(&self) -> Result<(), String> {
        self.bus.emit(EventType::Log("Starting orchestration loop...".to_string()));

        loop {
            // Hot-reload configuration dynamically
            let config = self.config_repo.load_config();

            // 1. Check for forced pause
            if self.is_paused() {
                self.bus.emit(EventType::UpdateStatus { 
                    text: "PAUSED".to_string(), 
                    color: "red".to_string() 
                });
                sleep(Duration::from_secs(1)).await;
                continue;
            }

            // 2. Verify target window visibility
            if !self.capturer.is_target_visible(&config.fenetres_surveillees) {
                self.bus.emit(EventType::UpdateStatus { 
                    text: "Waiting for target window...".to_string(), 
                    color: "orange".to_string() 
                });
                sleep(Duration::from_secs(2)).await;
                continue;
            }

            // 3. Wait for engine availability
            if self.llm_client.is_engine_busy(&config.url_api, &config.nom_modele).await {
                self.bus.emit(EventType::UpdateStatus { 
                    text: "Engine busy...".to_string(), 
                    color: "blue".to_string() 
                });
                sleep(Duration::from_secs(2)).await;
                continue;
            }

            // 4. Capture and Decide
            self.bus.emit(EventType::UpdateStatus { 
                text: "Analyzing...".to_string(), 
                color: "blue".to_string() 
            });

            let screenshot = self.capturer.capture_desktop()
                .or_else(|| {
                    if !config.fenetres_surveillees.is_empty() {
                        self.capturer.capture_window_by_title(&config.fenetres_surveillees[0])
                    } else {
                        None
                    }
                });

            if let Some(img) = screenshot {
                let response = self.llm_client.execute_decision(
                    &img,
                    &config.contexte,
                    &config.objectif,
                    &config.task,
                    &config.directives,
                    &config.url_api,
                    &config.nom_modele,
                ).await;

                match response {
                    Ok(res) => self.handle_decision(res).await?,
                    Err(e) => {
                        self.bus.emit(EventType::Log(format!("Error: {}", e)));
                        sleep(Duration::from_secs(5)).await;
                    }
                }
            } else {
                self.bus.emit(EventType::Log("Failed to capture screen".to_string()));
                sleep(Duration::from_secs(2)).await;
            }

            sleep(Duration::from_millis(500)).await;
        }
    }

    async fn handle_decision(&self, res: LlmResponse) -> Result<(), String> {
        self.bus.emit(EventType::UpdateStatus { 
            text: res.status_display.clone(), 
            color: "green".to_string() 
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

        if needs_confirmation && (res.action == "CLICK_AND_TYPE" || res.action == "SCROLL") {
            self.bus.emit(EventType::ShowActionConfirmation {
                action: res.action.clone(),
                text: if res.action == "CLICK_AND_TYPE" { res.text_to_type.clone() } else { String::new() },
                scroll: if res.action == "SCROLL" { res.scroll_value } else { 0 },
            });
            
            self.bus.emit(EventType::Log("Waiting for user approval of AI action...".to_string()));
            
            loop {
                let status = self.bus.emit_query(EventType::GetActionConfirmationStatus);
                if status == "approved" {
                    break;
                } else if status == "cancelled" {
                    self.bus.emit(EventType::Log("AI action cancelled by user.".to_string()));
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
                    display: res.text_to_type.clone() 
                });
                let payload = ActionPayload {
                    action: "CLICK_AND_TYPE".to_string(),
                    relative_click_position: res.relative_click_position.clone(),
                    text_to_type: res.text_to_type.clone(),
                    scroll_value: res.scroll_value,
                    wait_seconds: res.wait_seconds,
                };
                let offsets: Vec<ScreenOffset> = self.capturer.get_monitors()
                    .into_iter()
                    .map(ScreenOffset::from_screen_info)
                    .collect();
                self.controller.execute_action(&payload, &offsets);
                sleep(Duration::from_secs(res.wait_seconds as u64)).await;
            },
            "SCROLL" => {
                self.bus.emit(EventType::AppendAction { 
                    action: "Scroll".to_string(), 
                    display: format!("Value: {}", res.scroll_value) 
                });
                let payload = ActionPayload {
                    action: "SCROLL".to_string(),
                    relative_click_position: res.relative_click_position.clone(),
                    text_to_type: String::new(),
                    scroll_value: res.scroll_value,
                    wait_seconds: res.wait_seconds,
                };
                let offsets: Vec<ScreenOffset> = self.capturer.get_monitors()
                    .into_iter()
                    .map(ScreenOffset::from_screen_info)
                    .collect();
                self.controller.execute_action(&payload, &offsets);
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

    pub fn is_paused(&self) -> bool {
        self.bus.emit_query(EventType::GetModePauseForcee) == "true"
    }

    /// Set pause mode (for testing/configuration).
    pub fn set_pause_mode(&self, paused: bool) {
        self.bus.register_query(
            EventType::GetModePauseForcee,
            Box::new(move |_| {
                if paused { "true".to_string() } else { "false".to_string() }
            }),
        );
    }

    pub fn refresh_windows(&self) {
        self.capturer.refresh_windows();
    }

    pub fn get_available_windows(&self) -> Vec<String> {
        let mut wins = vec![crate::config::ALL_SCREENS_KEY.to_string()];
        for win in self.capturer.get_windows() {
            if !win.title.is_empty() && !wins.contains(&win.title) {
                wins.push(win.title.clone());
            }
        }
        wins
    }
}
