use crate::app::{VibePilotApp, PendingAction, StructuredStep};
use crate::event_bus::{NotificationEvent, CommandEvent, QueryEvent, Event, EventResponse};
use crate::config::{SavedConfig, ConfigRepository, EnginePresets};
use crate::llm_client::{LlmTextProvider, LlmMetadataProvider};
use std::sync::Arc;
use tokio::time::Duration;

impl VibePilotApp {
    pub fn poll_events(&mut self) {
        while let Ok(event) = self.event_receiver.try_recv() {
            match event {
                Event::Emit { event } => match event {
                    NotificationEvent::Log(msg) => {
                        let timestamp = chrono::Local::now().format("%H:%M:%S");
                        let formatted_log = format!("[{}] {}", timestamp, msg);
                        self.logs.push(formatted_log.clone());
                        if self.logs.len() > 500 {
                            self.logs.remove(0);
                        }

                        // Populate structured steps
                        if let Some(step) = self.structured_steps.last_mut() {
                            step.logs.push(formatted_log);
                        } else {
                            let mut init_step = StructuredStep::default();
                            init_step.action_type = "STARTUP".to_string();
                            init_step.logs.push(formatted_log);
                            self.structured_steps.push(init_step);
                        }

                        let logger = self.action_logger.clone();
                        let msg_clone = msg.clone();
                        self.rt.spawn(async move {
                            logger.info(&msg_clone);
                        });
                    }
                    NotificationEvent::UpdateStatus { text, color } => {
                        self.status_text = text;
                        self.status_color = color;
                    }
                    NotificationEvent::AppendAction { action, display } => {
                        self.action_history.push((action.clone(), display.clone()));

                        // Populate structured steps
                        self.structured_steps.push(StructuredStep {
                            action_type: action.clone(),
                            tooltip: display.clone(),
                            logs: Vec::new(),
                            report: String::new(),
                        });

                        let logger = self.action_logger.clone();
                        let action_clone = action.clone();
                        let display_clone = display.clone();
                        self.rt.spawn(async move {
                            logger.log(&action_clone, &display_clone, "executed");
                        });
                    }
                    NotificationEvent::AppendReport(report) => {
                        let timestamp = chrono::Local::now().format("%H:%M:%S");
                        let formatted_report = format!("[{}] {}", timestamp, report);
                        if self.report_content.is_empty() {
                            self.report_content = formatted_report.clone();
                        } else {
                            self.report_content = format!("{}\n\n{}", self.report_content, formatted_report);
                        }

                        // Populate structured steps
                        if let Some(step) = self.structured_steps.last_mut() {
                            if step.report.is_empty() {
                                step.report = formatted_report;
                            } else {
                                step.report = format!("{}\n\n{}", step.report, formatted_report);
                            }
                        }
                    }
                    NotificationEvent::LoopDetectedAlert { message } => {
                        self.status_text = message.clone();
                        self.status_color = "red".to_string();
                        self.send_webhook_notification(&format!("🚨 Loop detected alert: {}", message));
                    }
                    NotificationEvent::ClearActionConfirmation => {
                        self.pending_action = None;
                        if let Ok(mut status) = self.action_confirmation_status.lock() {
                            *status = "pending".to_string();
                        }
                    }
                },
                Event::Command { event } => match event {
                    CommandEvent::SelectTab(idx) => {
                        self.active_tab = match idx {
                            0 => crate::app::Tab::GlobalConfig,
                            1 => crate::app::Tab::PromptEditor,
                            2 => crate::app::Tab::Console,
                            3 => crate::app::Tab::TaskGraph,
                            4 => crate::app::Tab::Setup,
                            _ => self.active_tab,
                        };
                    }
                    CommandEvent::UpdateField { field, value } => {
                        match field.as_str() {
                            "contexte" => self.current_config.contexte = value,
                            "task" => self.current_config.task = value,
                            "objectif" => self.current_config.objectif = value,
                            "directives" => self.current_config.directives = value,
                            "user_feedback" => self.user_feedback_input = value,
                            _ => {}
                        }
                    }
                    CommandEvent::UpdateAllFields { contexte, task, objectif, directives } => {
                        self.current_config.contexte = contexte;
                        self.current_config.task = task;
                        self.current_config.objectif = objectif;
                        self.current_config.directives = directives;
                        if !self.selected_profile.is_empty() {
                            self.config_repo.save_profile(&self.selected_profile, &self.current_config);
                            self.last_saved_config = self.current_config.clone();
                        }
                    }
                     CommandEvent::ShowActionConfirmation { action, text, scroll } => {
                        self.pending_action = Some(PendingAction {
                            action: action.clone(),
                            text_to_type: text.clone(),
                            scroll_value: scroll,
                        });
                        if let Ok(mut status) = self.action_confirmation_status.lock() {
                            *status = "pending".to_string();
                        }
                        self.send_webhook_notification(&format!("⏳ Awaiting human approval for action: {} (details: {})", action, text));
                    }
                    CommandEvent::SetGeneratingPrompts(status) => {
                        self.is_generating_prompts = status;
                    }
                    CommandEvent::UpdateModelsList { engine, models } => {
                        if engine == "LM Studio" {
                            self.engine_presets.lm_studio.modeles = models;
                        } else if engine == "Ollama" {
                            self.engine_presets.ollama.modeles = models;
                        } else if engine == "Perso / Autre" {
                            self.engine_presets.custom.modeles = models;
                        } else {
                            if let Some(preset) = self.engine_presets.custom_engines.get_mut(&engine) {
                                preset.modeles = models;
                            }
                        }
                        self.config_repo.save_engines(&self.engine_presets);
                    }
                    CommandEvent::CreateProfileWithPrompts { profile_name, contexte, task, objectif, directives } => {
                        self.current_config.contexte = contexte;
                        self.current_config.task = task;
                        self.current_config.objectif = objectif;
                        self.current_config.directives = directives;
                        if !profile_name.is_empty() {
                            self.selected_profile = profile_name.clone();
                            self.config_repo.save_profile(&self.selected_profile, &self.current_config);
                            self.last_saved_config = self.current_config.clone();
                            self.quick_start_profile_name = self.get_first_available_profile_name();
                            self.show_profile_ready_popup = Some(profile_name);
                        }
                    }
                    CommandEvent::ShowProfileReadyPopup { profile_name } => {
                        self.show_profile_ready_popup = Some(profile_name);
                    }
                    CommandEvent::StopOrchestrator => {
                        self.stop_orchestrator();
                    }
                    CommandEvent::StartOrchestrator => {
                        self.start_orchestrator();
                    }
                    CommandEvent::LoadProfile(profile_name) => {
                        self.load_profile(&profile_name);
                    }
                },
                Event::Query { event, sender } => {
                    let response = self.bus.emit_query(event.clone());
                    let _ = sender.send(EventResponse {
                        event_type: event,
                        response,
                    });
                }
            }
        }
    }

    pub fn optimize_prompt_field(&mut self, field_type: &str) {
        let url = self.current_config.url_api.clone();
        let model = self.current_config.nom_modele.clone();
        let auth_mode = self.current_config.auth_mode.clone();
        let auth_api_key = self.current_config.auth_api_key.clone();
        let auth_login = self.current_config.auth_login.clone();
        let auth_password = self.current_config.auth_password.clone();
        let timeout_secs = self.current_config.request_timeout_secs;
        let text = match field_type {
            "contexte" => self.current_config.contexte.clone(),
            "task" => self.current_config.task.clone(),
            "objectif" => self.current_config.objectif.clone(),
            "directives" => self.current_config.directives.clone(),
            "user_feedback" => self.user_feedback_input.clone(),
            _ => return,
        };
        let field = field_type.to_string();
        let bus = self.bus.clone();
        let rt = self.rt.handle().clone();

        bus.emit_notification(NotificationEvent::Log(format!(
            "📡 Sending Text LLM Request (Optimize Field) | Model: '{}' | Endpoint: '{}' | Field: '{}'",
            model, url, field
        )));

        rt.spawn(async move {
            let client = crate::llm_client::LlmClientFactory::create();
            match client.optimize_field(&text, &field, &url, &model, &auth_mode, &auth_api_key, &auth_login, &auth_password, timeout_secs).await {
                Ok(optimized) => {
                    bus.emit_notification(NotificationEvent::Log(format!("📥 Received LLM Response (Optimize Field) | Success (Field '{}' optimized)", field)));
                    bus.emit_command(CommandEvent::UpdateField { field, value: optimized });
                }
                Err(e) => {
                    bus.emit_notification(NotificationEvent::Log(format!("❌ LLM Error (Optimize Field) | Failed to optimize field '{}': {}", field, e)));
                }
            }
        });
    }

    pub fn generate_prompts_from_request(&mut self) {
        let url = self.current_config.url_api.clone();
        let model = self.current_config.nom_modele.clone();
        let request = self.current_config.demande_generique.clone();
        if request.is_empty() {
            self.bus.emit_notification(NotificationEvent::Log("Please enter a request first!".to_string()));
            return;
        }

        self.is_generating_prompts = true;

        let profile_name = self.quick_start_profile_name.trim().to_string();
        let auth_mode = self.current_config.auth_mode.clone();
        let auth_api_key = self.current_config.auth_api_key.clone();
        let auth_login = self.current_config.auth_login.clone();
        let auth_password = self.current_config.auth_password.clone();
        let timeout_secs = self.current_config.request_timeout_secs;

        let bus = self.bus.clone();
        let rt = self.rt.handle().clone();

        bus.emit_notification(NotificationEvent::Log(format!(
            "📡 Sending Text LLM Request (Generate Prompts) | Model: '{}' | Endpoint: '{}'",
            model, url
        )));

        rt.spawn(async move {
            let client = crate::llm_client::LlmClientFactory::create();
            match client.generate_config(&request, &url, &model, &auth_mode, &auth_api_key, &auth_login, &auth_password, timeout_secs).await {
                Ok(json_value) => {
                    bus.emit_notification(NotificationEvent::Log("📥 Received LLM Response (Generate Prompts) | Success".to_string()));
                    
                    let get_value = |json: &serde_json::Value, keys: &[&str]| -> serde_json::Value {
                        for key in keys {
                            if let Some(val) = json.get(key) {
                                if !val.is_null() {
                                    return val.clone();
                                }
                            }
                        }
                        serde_json::Value::Null
                    };

                    let parse_to_string = |val: &serde_json::Value| -> String {
                        if val.is_null() {
                            return String::new();
                        }
                        if let Some(s) = val.as_str() {
                            s.to_string()
                        } else if let Some(arr) = val.as_array() {
                            arr.iter()
                                .map(|v| {
                                    if let Some(s) = v.as_str() {
                                        s.to_string()
                                    } else {
                                        v.to_string()
                                    }
                                })
                                .collect::<Vec<String>>()
                                .join("\n")
                        } else {
                            val.to_string()
                        }
                    };

                    let contexte = parse_to_string(&get_value(&json_value, &["contexte", "context"]));
                    let task = parse_to_string(&get_value(&json_value, &["task", "tache", "instructions"]));
                    let objectif = parse_to_string(&get_value(&json_value, &["objectif", "stop_condition", "objective"]));
                    let directives = parse_to_string(&get_value(&json_value, &["directives", "rules", "system_rules", "directives_systeme"]));

                    bus.emit_command(CommandEvent::CreateProfileWithPrompts {
                        profile_name,
                        contexte,
                        task,
                        objectif,
                        directives,
                    });
                }
                Err(e) => {
                    bus.emit_notification(NotificationEvent::Log(format!("❌ LLM Error (Generate Prompts) | Failed to generate prompts: {}", e)));
                }
            }
            bus.emit_command(CommandEvent::SetGeneratingPrompts(false));
        });
    }

    pub fn scan_models(&mut self) {
        let url = self.current_config.url_api.clone();
        let engine = self.current_config.moteur.clone();
        let auth_mode = self.current_config.auth_mode.clone();
        let auth_api_key = self.current_config.auth_api_key.clone();
        let auth_login = self.current_config.auth_login.clone();
        let auth_password = self.current_config.auth_password.clone();
        let bus = self.bus.clone();
        let rt = self.rt.handle().clone();

        bus.emit_notification(NotificationEvent::Log(format!("Scanning models for engine '{}'...", engine)));

        rt.spawn(async move {
            let client = crate::llm_client::LlmClientFactory::create();
            match client.fetch_models(&url, &auth_mode, &auth_api_key, &auth_login, &auth_password).await {
                Ok(models) => {
                    bus.emit_notification(NotificationEvent::Log(format!("Scan success! Found {} models.", models.len())));
                    bus.emit_command(CommandEvent::UpdateModelsList { engine, models });
                }
                Err(e) => {
                    bus.emit_notification(NotificationEvent::Log(format!("Model scan failed: {}", e)));
                }
            }
        });
    }

    pub fn scan_models_vision(&mut self) {
        let url = self.current_config.url_api_vision.clone();
        let engine = self.current_config.moteur_vision.clone();
        let auth_mode = self.current_config.auth_mode_vision.clone();
        let auth_api_key = self.current_config.auth_api_key_vision.clone();
        let auth_login = self.current_config.auth_login_vision.clone();
        let auth_password = self.current_config.auth_password_vision.clone();
        let bus = self.bus.clone();
        let rt = self.rt.handle().clone();

        bus.emit_notification(NotificationEvent::Log(format!("Scanning models for vision engine '{}'...", engine)));

        rt.spawn(async move {
            let client = crate::llm_client::LlmClientFactory::create();
            match client.fetch_models(&url, &auth_mode, &auth_api_key, &auth_login, &auth_password).await {
                Ok(models) => {
                    bus.emit_notification(NotificationEvent::Log(format!("Vision scan success! Found {} models.", models.len())));
                    bus.emit_command(CommandEvent::UpdateModelsList { engine, models });
                }
                Err(e) => {
                    bus.emit_notification(NotificationEvent::Log(format!("Vision model scan failed: {}", e)));
                }
            }
        });
    }

    pub fn migrate_storage(&mut self, new_path: String) -> Result<(), String> {
        use std::fs;
        use std::path::PathBuf;

        let new_base = PathBuf::from(&new_path);
        if let Err(e) = fs::create_dir_all(&new_base) {
            return Err(format!("Failed to create directory: {}", e));
        }

        let mut bootstrap = crate::config::load_bootstrap_config();
        bootstrap.storage_dir = Some(new_path.clone());
        crate::config::save_bootstrap_config(&bootstrap);

        let old_save = self.config_repo.get_save_path();
        let old_engines = self.config_repo.get_engines_path();
        let old_profiles = self.config_repo.get_profiles_dir();
        let old_store = self.config_repo.get_store_path();
        let old_key = self.config_repo.get_key_path();

        let new_save = new_base.join("save.enc");
        let new_engines = new_base.join("engines.enc");
        let new_profiles = new_base.join("profiles");
        let new_store = new_base.join("vibepilot_data.enc");
        let new_key = new_base.join("key.enc");

        if old_store.exists() { let _ = fs::copy(&old_store, &new_store); }
        if old_key.exists() { let _ = fs::copy(&old_key, &new_key); }
        if old_save.exists() { let _ = fs::copy(&old_save, &new_save); }
        if old_engines.exists() { let _ = fs::copy(&old_engines, &new_engines); }
        if old_profiles.exists() {
            let _ = fs::create_dir_all(&new_profiles);
            if let Ok(entries) = fs::read_dir(&old_profiles) {
                for entry in entries.filter_map(|e| e.ok()) {
                    let path = entry.path();
                    if path.is_file() {
                        if let Some(name) = path.file_name() {
                            let _ = fs::copy(&path, new_profiles.join(name));
                        }
                    }
                }
            }
        }

        self.config_repo = crate::config::ConfigRepositoryFactory::create(new_base);
        self.orchestrator = Arc::new(crate::orchestrator::VibePilotOrchestrator::new(
            self.config_repo.clone(),
            self.orchestrator.llm_client.clone(),
            self.orchestrator.capturer.clone(),
            self.orchestrator.controller.clone(),
            self.orchestrator.wait_manager.clone(),
            self.bus.clone(),
        ));

        self.current_config = self.config_repo.load_config();
        self.last_saved_config = self.current_config.clone();
        self.engine_presets = self.config_repo.load_engines();

        Ok(())
    }

    pub fn reset_all_settings(&mut self) {
        self.current_config.contexte = crate::config::DEFAULT_CONTEXT.to_string();
        self.current_config.objectif = crate::config::DEFAULT_OBJECTIF.to_string();
        self.current_config.task = crate::config::DEFAULT_TASK.to_string();
        self.current_config.directives = crate::config::DEFAULT_DIRECTIVES.to_string();
        self.current_config.fenetres_surveillees = vec![crate::config::ALL_SCREENS_KEY.to_string()];
        self.current_config.langue = "English".to_string();
        self.current_config.activer_son = true;
        self.current_config.activer_tooltips = true;
        self.current_config.auto_validate = true;
        self.current_config.auto_validate_dangerous = false;
        self.current_config.theme_sombre = true;
        self.current_config.prompt_reprise = None;
        self.current_config.zoom_facteur = None;
        self.selected_profile = self.get_first_available_profile_name();
        self.quick_start_profile_name = self.get_first_available_profile_name();
        self.bus.emit_notification(NotificationEvent::Log("Everything reset to default".to_string()));
    }

    pub fn send_webhook_notification(&self, text: &str) {
        if let Some(ref url) = self.current_config.webhook_url {
            if url.trim().is_empty() {
                return;
            }
            let url_clone = url.clone();
            let text_clone = text.to_string();
            let client = reqwest::Client::new();
            self.rt.spawn(async move {
                let payload = serde_json::json!({
                    "content": text_clone,
                    "text": text_clone,
                });
                let _ = client.post(&url_clone)
                    .json(&payload)
                    .send()
                    .await;
            });
        }
    }
}

#[cfg(target_os = "windows")]
pub fn set_sleep_prevented(prevented: bool) {
    use windows::Win32::System::Power::{
        SetThreadExecutionState, ES_CONTINUOUS, ES_DISPLAY_REQUIRED, ES_SYSTEM_REQUIRED
    };
    unsafe {
        if prevented {
            let _ = SetThreadExecutionState(ES_CONTINUOUS | ES_DISPLAY_REQUIRED | ES_SYSTEM_REQUIRED);
        } else {
            let _ = SetThreadExecutionState(ES_CONTINUOUS);
        }
    }
}

#[cfg(not(target_os = "windows"))]
pub fn set_sleep_prevented(_prevented: bool) {}
