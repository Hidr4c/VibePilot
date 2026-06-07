//! Main application state for VibePilot.
//!
//! Manages the egui app lifecycle, event processing, orchestrator
//! lifecycle, and config persistence.

use crate::config::{ActionLogger, ConfigRepository, SavedConfig, EnginePresets};
use crate::event_bus::{Event, EventBus, EventType, EventResponse};
use crate::orchestrator::VibePilotOrchestrator;
use eframe::egui;
use std::sync::Arc;
use tokio::runtime::Runtime;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct PendingAction {
    pub action: String,
    pub text_to_type: String,
    pub scroll_value: i32,
}

pub struct VibePilotApp {
    pub config_repo: Arc<ConfigRepository>,
    pub action_logger: Arc<ActionLogger>,
    pub bus: EventBus,
    pub orchestrator: Arc<VibePilotOrchestrator>,
    pub rt: Runtime,
    pub current_config: SavedConfig,
    pub engine_presets: EnginePresets,
    pub logs: Vec<String>,
    pub status_text: String,
    pub status_color: String,
    pub action_history: Vec<(String, String)>,
    pub active_tab: Tab,
    pub custom_storage_dir: String,
    pub storage_status_message: String,
    pub is_running: bool,
    pub pause_mode: bool,
    event_receiver: flume::Receiver<Event>,
    pause_ref: std::sync::Arc<std::sync::atomic::AtomicBool>,
    running_ref: std::sync::Arc<std::sync::atomic::AtomicBool>,

    pub last_saved_config: SavedConfig,
    pub top_panel_height_fraction: f32,

    // UI state fields
    pub selected_profile: String,
    pub show_add_engine: bool,
    pub new_engine_name: String,
    pub new_engine_url: String,
    pub show_delete_confirm: Option<String>,
    pub show_reset_confirm: bool,
    pub show_rename_profile: Option<String>,
    pub rename_profile_new_name: String,
    pub quick_start_profile_name: String,
    pub is_generating_prompts: bool,
    pub report_content: String,
    pub bottom_tab: usize,
    pub auto_scroll_logs: bool,

    pub selected_profile_to_export: String,
    pub selected_engine_to_export: String,
    pub show_profile_ready_popup: Option<String>,

    pub pending_action: Option<PendingAction>,
    pub action_confirmation_status: std::sync::Arc<std::sync::Mutex<String>>,

    pub user_feedback_input: String,
    pub accumulated_user_feedback: std::sync::Arc<std::sync::Mutex<String>>,

    pub lexicon_fr: std::collections::HashMap<&'static str, &'static str>,
    pub lexicon_en: std::collections::HashMap<&'static str, &'static str>,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Tab {
    GlobalConfig,
    PromptEditor,
    Setup,
    Help,
    Console,
}

impl VibePilotApp {
    pub fn new(egui_ctx: &egui::Context) -> Self {
        let (bus, receiver) = EventBus::new();
        
        let base_dir = match crate::config::load_bootstrap_config().storage_dir {
            Some(dir_str) if !dir_str.is_empty() => std::path::PathBuf::from(&dir_str),
            _ => std::env::current_dir().unwrap_or_default(),
        };
        let custom_dir_str = base_dir.to_string_lossy().to_string();

        let config_repo = Arc::new(ConfigRepository::new(base_dir.clone()));
        let log_path = base_dir.join("vibepilot.log");
        let action_logger = Arc::new(ActionLogger::new(log_path));
        let orchestrator = Arc::new(VibePilotOrchestrator::new(
            config_repo.clone(),
            bus.clone(),
        ));

        let pause_ref = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let pause_clone = pause_ref.clone();

        let running_ref = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let running_clone = running_ref.clone();

        let confirmation_status = std::sync::Arc::new(std::sync::Mutex::new("pending".to_string()));
        let status_clone = confirmation_status.clone();

        let accumulated_user_feedback = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
        let feedback_clone_for_query = accumulated_user_feedback.clone();

        let mut app = Self {
            config_repo,
            action_logger,
            bus: bus.clone(),
            orchestrator,
            rt: Runtime::new().unwrap(),
            current_config: SavedConfig::default(),
            last_saved_config: SavedConfig::default(),
            top_panel_height_fraction: 0.55,
            engine_presets: EnginePresets::default(),
            logs: Vec::new(),
            status_text: "Ready".to_string(),
            status_color: "grey".to_string(),
            action_history: Vec::new(),
            active_tab: Tab::GlobalConfig,
            custom_storage_dir: custom_dir_str,
            storage_status_message: String::new(),
            is_running: false,
            pause_mode: false,
            event_receiver: receiver,
            pause_ref,
            running_ref,

            selected_profile: String::new(),
            show_add_engine: false,
            new_engine_name: String::new(),
            new_engine_url: String::new(),
            show_delete_confirm: None,
            show_reset_confirm: false,
            show_rename_profile: None,
            rename_profile_new_name: String::new(),
            quick_start_profile_name: String::new(),
            is_generating_prompts: false,
            report_content: String::new(),
            bottom_tab: 0,
            auto_scroll_logs: true,

            selected_profile_to_export: String::new(),
            selected_engine_to_export: String::new(),
            show_profile_ready_popup: None,

            pending_action: None,
            action_confirmation_status: confirmation_status,

            user_feedback_input: String::new(),
            accumulated_user_feedback,

            lexicon_fr: crate::content::lexicon_fr(),
            lexicon_en: crate::content::lexicon_en(),
        };

        app.current_config = app.config_repo.load_config();
        app.last_saved_config = app.current_config.clone();
        app.engine_presets = app.config_repo.load_engines();
        let engines = app.engine_presets.all_keys();
        if !engines.is_empty() {
            app.selected_engine_to_export = engines[0].clone();
        }

        let last_profile = app.current_config.dernier_profil.clone();
        if !last_profile.is_empty() && app.config_repo.list_profiles().contains(&last_profile) {
            app.selected_profile = last_profile.clone();
            app.quick_start_profile_name = last_profile;
        } else {
            app.selected_profile = app.get_first_available_profile_name();
            app.quick_start_profile_name = app.get_first_available_profile_name();
        }

        if let Some(zoom) = app.current_config.zoom_facteur {
            egui_ctx.set_pixels_per_point(zoom);
        }

        bus.register_query(
            EventType::GetActionConfirmationStatus,
            Box::new(move |_| {
                status_clone.lock().unwrap().clone()
            }),
        );

        bus.register_query(
            EventType::GetUserFeedback,
            Box::new(move |_| {
                let mut guard = feedback_clone_for_query.lock().unwrap();
                let feedback = guard.clone();
                guard.clear();
                feedback
            }),
        );

        bus.register_query(
            EventType::GetModePauseForcee,
            Box::new(move |_| {
                if pause_clone.load(std::sync::atomic::Ordering::Relaxed) {
                    "true".to_string()
                } else {
                    "false".to_string()
                }
            }),
        );

        bus.register_query(
            EventType::GetOrchestratorRunning,
            Box::new(move |_| {
                if running_clone.load(std::sync::atomic::Ordering::Relaxed) {
                    "true".to_string()
                } else {
                    "false".to_string()
                }
            }),
        );

        app
    }

    /// Translation helper method.
    pub fn t(&self, key: &str) -> String {
        let is_fr = self.current_config.langue == "Français";
        let map = if is_fr { &self.lexicon_fr } else { &self.lexicon_en };
        map.get(key).copied().unwrap_or(key).to_string()
    }

    pub fn start_orchestrator(&mut self) {
        self.is_running = true;
        self.running_ref.store(true, std::sync::atomic::Ordering::Relaxed);
        let orchestrator = self.orchestrator.clone();
        let action_logger = self.action_logger.clone();
        let rt = self.rt.handle().clone();
        rt.spawn(async move {
            match orchestrator.run_loop().await {
                Ok(()) => {
                    action_logger.info("Orchestration loop ended normally");
                }
                Err(e) if e == "STOP_SUCCESS" => {
                    action_logger.info("Orchestration stopped by LLM SUCCESS action");
                }
                Err(e) => {
                    action_logger.error(&format!("Orchestrator error: {}", e));
                    println!("Orchestrator error: {}", e);
                }
            }
        });
    }

    pub fn stop_orchestrator(&mut self) {
        self.is_running = false;
        self.running_ref.store(false, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn load_profile(&mut self, name: &str) -> bool {
        if let Some(mut cfg) = self.config_repo.load_profile(name) {
            // Stop orchestrator if running
            self.stop_orchestrator();
            
            // Clear history, logs, report, and reset status
            self.action_history.clear();
            self.logs.clear();
            self.report_content.clear();
            self.user_feedback_input.clear();
            self.accumulated_user_feedback.lock().unwrap().clear();
            self.status_text = "Ready".to_string();
            self.status_color = "grey".to_string();
            
            // Apply loaded config
            cfg.dernier_profil = name.to_string();
            self.current_config = cfg.clone();
            self.last_saved_config = cfg;
            self.selected_profile = name.to_string();
            self.quick_start_profile_name = name.to_string();
            
            self.bus.emit(EventType::Log(format!("Profile '{}' loaded. Session reset.", name)));
            true
        } else {
            false
        }
    }

    pub fn poll_events(&mut self) {
        while let Ok(event) = self.event_receiver.try_recv() {
            match event {
                Event::Emit { event } => match event {
                    EventType::Log(msg) => {
                        let timestamp = chrono::Local::now().format("%H:%M:%S");
                        self.logs.push(format!("[{}] {}", timestamp, msg));
                        if self.logs.len() > 500 {
                            self.logs.remove(0);
                        }
                        // Persistent file logging (non-blocking)
                        let logger = self.action_logger.clone();
                        let msg_clone = msg.clone();
                        self.rt.spawn(async move {
                            logger.info(&msg_clone);
                        });
                    }
                    EventType::UpdateStatus { text, color } => {
                        self.status_text = text;
                        self.status_color = color;
                    }
                    EventType::AppendAction { action, display } => {
                        self.action_history.push((action.clone(), display.clone()));
                        // Persistent file logging for actions
                        let logger = self.action_logger.clone();
                        let action_clone = action.clone();
                        let display_clone = display.clone();
                        self.rt.spawn(async move {
                            logger.log(&action_clone, &display_clone, "executed");
                        });
                    }
                    EventType::SelectTab(idx) => {
                        self.active_tab = match idx {
                            0 => Tab::GlobalConfig,
                            1 => Tab::PromptEditor,
                            2 => Tab::Help,
                            3 => Tab::Setup,
                            4 => Tab::Console,
                            _ => self.active_tab,
                        };
                    }
                    EventType::UpdateField { field, value } => {
                        match field.as_str() {
                            "contexte" => self.current_config.contexte = value,
                            "task" => self.current_config.task = value,
                            "objectif" => self.current_config.objectif = value,
                            "directives" => self.current_config.directives = value,
                            "user_feedback" => self.user_feedback_input = value,
                            _ => {}
                        }
                    }
                    EventType::UpdateAllFields { contexte, task, objectif, directives } => {
                        self.current_config.contexte = contexte;
                        self.current_config.task = task;
                        self.current_config.objectif = objectif;
                        self.current_config.directives = directives;
                        if !self.selected_profile.is_empty() {
                            self.config_repo.save_profile(&self.selected_profile, &self.current_config);
                            self.last_saved_config = self.current_config.clone();
                        }
                    }
                    EventType::ShowActionConfirmation { action, text, scroll } => {
                        self.pending_action = Some(PendingAction {
                            action,
                            text_to_type: text,
                            scroll_value: scroll,
                        });
                        if let Ok(mut status) = self.action_confirmation_status.lock() {
                            *status = "pending".to_string();
                        }
                    }
                    EventType::ClearActionConfirmation => {
                        self.pending_action = None;
                        if let Ok(mut status) = self.action_confirmation_status.lock() {
                            *status = "pending".to_string();
                        }
                    }
                    EventType::SetGeneratingPrompts(status) => {
                        self.is_generating_prompts = status;
                    }
                    EventType::AppendReport(report) => {
                        let timestamp = chrono::Local::now().format("%H:%M:%S");
                        if self.report_content.is_empty() {
                            self.report_content = format!("[{}] {}", timestamp, report);
                        } else {
                            self.report_content = format!("{}\n\n[{}] {}", self.report_content, timestamp, report);
                        }
                    }
                    EventType::UpdateModelsList { engine, models } => {
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
                    EventType::CreateProfileWithPrompts { profile_name, contexte, task, objectif, directives } => {
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
                    _ => {}
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

        bus.emit(EventType::Log(format!(
            "📡 Sending Text LLM Request (Optimize Field) | Model: '{}' | Endpoint: '{}' | Field: '{}'",
            model, url, field
        )));

        rt.spawn(async move {
            let client = crate::llm_client::LlmClient::new();
            match client.optimize_field(&text, &field, &url, &model, &auth_mode, &auth_api_key, &auth_login, &auth_password, timeout_secs).await {
                Ok(optimized) => {
                    bus.emit(EventType::Log(format!("📥 Received LLM Response (Optimize Field) | Success (Field '{}' optimized)", field)));
                    bus.emit(EventType::UpdateField { field, value: optimized });
                }
                Err(e) => {
                    bus.emit(EventType::Log(format!("❌ LLM Error (Optimize Field) | Failed to optimize field '{}': {}", field, e)));
                }
            }
        });
    }

    pub fn generate_prompts_from_request(&mut self) {
        let url = self.current_config.url_api.clone();
        let model = self.current_config.nom_modele.clone();
        let request = self.current_config.demande_generique.clone();
        if request.is_empty() {
            self.bus.emit(EventType::Log("Please enter a request first!".to_string()));
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

        bus.emit(EventType::Log(format!(
            "📡 Sending Text LLM Request (Generate Prompts) | Model: '{}' | Endpoint: '{}'",
            model, url
        )));

        rt.spawn(async move {
            let client = crate::llm_client::LlmClient::new();
            match client.generate_config(&request, &url, &model, &auth_mode, &auth_api_key, &auth_login, &auth_password, timeout_secs).await {
                Ok(json_value) => {
                    bus.emit(EventType::Log("📥 Received LLM Response (Generate Prompts) | Success".to_string()));
                    
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

                    bus.emit(EventType::CreateProfileWithPrompts {
                        profile_name,
                        contexte,
                        task,
                        objectif,
                        directives,
                    });
                }
                Err(e) => {
                    bus.emit(EventType::Log(format!("❌ LLM Error (Generate Prompts) | Failed to generate prompts: {}", e)));
                }
            }
            bus.emit(EventType::SetGeneratingPrompts(false));
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

        bus.emit(EventType::Log(format!("Scanning models for engine '{}'...", engine)));

        rt.spawn(async move {
            let client = crate::llm_client::LlmClient::new();
            match client.fetch_models(&url, &auth_mode, &auth_api_key, &auth_login, &auth_password).await {
                Ok(models) => {
                    bus.emit(EventType::Log(format!("Scan success! Found {} models.", models.len())));
                    bus.emit(EventType::UpdateModelsList { engine, models });
                }
                Err(e) => {
                    bus.emit(EventType::Log(format!("Model scan failed: {}", e)));
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

        bus.emit(EventType::Log(format!("Scanning models for vision engine '{}'...", engine)));

        rt.spawn(async move {
            let client = crate::llm_client::LlmClient::new();
            match client.fetch_models(&url, &auth_mode, &auth_api_key, &auth_login, &auth_password).await {
                Ok(models) => {
                    bus.emit(EventType::Log(format!("Vision scan success! Found {} models.", models.len())));
                    bus.emit(EventType::UpdateModelsList { engine, models });
                }
                Err(e) => {
                    bus.emit(EventType::Log(format!("Vision model scan failed: {}", e)));
                }
            }
        });
    }

    pub fn get_pause_ref(&self) -> &std::sync::Arc<std::sync::atomic::AtomicBool> {
        &self.pause_ref
    }

    pub fn migrate_storage(&mut self, new_path: String) -> Result<(), String> {
        use std::fs;
        use std::path::PathBuf;

        let new_base = PathBuf::from(&new_path);
        if let Err(e) = fs::create_dir_all(&new_base) {
            return Err(format!("Failed to create directory: {}", e));
        }

        // Save bootstrap config
        let mut bootstrap = crate::config::load_bootstrap_config();
        bootstrap.storage_dir = Some(new_path.clone());
        crate::config::save_bootstrap_config(&bootstrap);

        // Retrieve old paths from current config_repo
        let old_save = self.config_repo.get_save_path();
        let old_engines = self.config_repo.get_engines_path();
        let old_profiles = self.config_repo.get_profiles_dir();

        // Define new paths
        let new_save = new_base.join("save.enc");
        let new_engines = new_base.join("engines.enc");
        let new_profiles = new_base.join("profiles");

        // Copy save.enc
        if old_save.exists() {
            let _ = fs::copy(&old_save, &new_save);
        }
        // Copy engines.enc
        if old_engines.exists() {
            let _ = fs::copy(&old_engines, &new_engines);
        }
        // Copy profiles
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

        // Update config_repo and orchestrator
        self.config_repo = Arc::new(ConfigRepository::new(new_base));
        self.orchestrator = Arc::new(crate::orchestrator::VibePilotOrchestrator::new(
            self.config_repo.clone(),
            self.bus.clone(),
        ));

        // Reload config
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
        self.bus.emit(EventType::Log("Everything reset to default".to_string()));
    }

    pub fn get_first_available_profile_name(&self) -> String {
        let existing_profiles = self.config_repo.list_profiles();
        for i in 1..=9995 {
            let name = format!("profile_{:02}", i);
            if !existing_profiles.contains(&name) {
                return name;
            }
        }
        "profile_9995".to_string()
    }
}

impl eframe::App for VibePilotApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_events();

        crate::ui::render_main_window(ctx, self);

        // Auto-save configuration on change
        if self.current_config != self.last_saved_config {
            self.config_repo.save_config(&self.current_config);
            self.last_saved_config = self.current_config.clone();
        }

        ctx.request_repaint_after(std::time::Duration::from_millis(250));
    }
}
