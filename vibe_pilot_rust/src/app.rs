//! Main application state for VibePilot.

use crate::config::{ConfigRepository, SavedConfig, EnginePresets};
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

    pub pending_action: Option<PendingAction>,
    pub action_confirmation_status: std::sync::Arc<std::sync::Mutex<String>>,

    pub lexicon_fr: std::collections::HashMap<&'static str, &'static str>,
    pub lexicon_en: std::collections::HashMap<&'static str, &'static str>,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Tab {
    GlobalConfig,
    PromptEditor,
    Setup,
    Help,
}

impl VibePilotApp {
    pub fn new(egui_ctx: &egui::Context) -> Self {
        let (bus, receiver) = EventBus::new();
        
        let base_dir = match crate::config::load_bootstrap_config().storage_dir {
            Some(dir_str) if !dir_str.is_empty() => std::path::PathBuf::from(&dir_str),
            _ => std::env::current_dir().unwrap_or_default(),
        };
        let custom_dir_str = base_dir.to_string_lossy().to_string();

        let config_repo = Arc::new(ConfigRepository::new(base_dir));
        let orchestrator = Arc::new(VibePilotOrchestrator::new(
            config_repo.clone(),
            bus.clone(),
        ));

        let pause_ref = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let pause_clone = pause_ref.clone();

        let confirmation_status = std::sync::Arc::new(std::sync::Mutex::new("pending".to_string()));
        let status_clone = confirmation_status.clone();

        let mut app = Self {
            config_repo,
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

            selected_profile: String::new(),
            show_add_engine: false,
            new_engine_name: String::new(),
            new_engine_url: String::new(),
            show_delete_confirm: None,
            show_reset_confirm: false,
            show_rename_profile: None,
            rename_profile_new_name: String::new(),
            quick_start_profile_name: String::new(),

            pending_action: None,
            action_confirmation_status: confirmation_status,

            lexicon_fr: crate::content::lexicon_fr(),
            lexicon_en: crate::content::lexicon_en(),
        };

        app.current_config = app.config_repo.load_config();
        app.last_saved_config = app.current_config.clone();
        app.engine_presets = app.config_repo.load_engines();
        app.selected_profile = app.get_first_available_profile_name();
        app.quick_start_profile_name = app.get_first_available_profile_name();

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
            EventType::GetModePauseForcee,
            Box::new(move |_| {
                if pause_clone.load(std::sync::atomic::Ordering::Relaxed) {
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
        let orchestrator = self.orchestrator.clone();
        let rt = self.rt.handle().clone();
        rt.spawn(async move {
            if let Err(e) = orchestrator.run_loop().await {
                if e != "STOP_SUCCESS" {
                    println!("Orchestrator error: {}", e);
                }
            }
        });
    }

    pub fn stop_orchestrator(&mut self) {
        self.is_running = false;
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
                    }
                    EventType::UpdateStatus { text, color } => {
                        self.status_text = text;
                        self.status_color = color;
                    }
                    EventType::AppendAction { action, display } => {
                        self.action_history.push((action, display));
                    }
                    EventType::SelectTab(idx) => {
                        self.active_tab = match idx {
                            0 => Tab::GlobalConfig,
                            1 => Tab::PromptEditor,
                            2 => Tab::Help,
                            3 => Tab::Setup,
                            _ => self.active_tab,
                        };
                    }
                    EventType::UpdateField { field, value } => {
                        match field.as_str() {
                            "contexte" => self.current_config.contexte = value,
                            "task" => self.current_config.task = value,
                            "objectif" => self.current_config.objectif = value,
                            "directives" => self.current_config.directives = value,
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
        let text = match field_type {
            "contexte" => self.current_config.contexte.clone(),
            "task" => self.current_config.task.clone(),
            "objectif" => self.current_config.objectif.clone(),
            "directives" => self.current_config.directives.clone(),
            _ => return,
        };
        let field = field_type.to_string();
        let bus = self.bus.clone();
        let rt = self.rt.handle().clone();

        bus.emit(EventType::Log(format!("Optimizing field '{}' via AI...", field)));

        rt.spawn(async move {
            let client = crate::llm_client::LlmClient::new();
            match client.optimize_field(&text, &field, &url, &model).await {
                Ok(optimized) => {
                    bus.emit(EventType::Log(format!("Field '{}' optimized successfully!", field)));
                    bus.emit(EventType::UpdateField { field, value: optimized });
                }
                Err(e) => {
                    bus.emit(EventType::Log(format!("Failed to optimize field '{}': {}", field, e)));
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

        let new_profile = self.quick_start_profile_name.trim().to_string();
        if !new_profile.is_empty() {
            self.selected_profile = new_profile;
            self.config_repo.save_profile(&self.selected_profile, &self.current_config);
            self.last_saved_config = self.current_config.clone();
            self.quick_start_profile_name = self.get_first_available_profile_name();
        }

        let bus = self.bus.clone();
        let rt = self.rt.handle().clone();

        bus.emit(EventType::Log("Generating full prompt configuration via AI...".to_string()));

        rt.spawn(async move {
            let client = crate::llm_client::LlmClient::new();
            match client.generate_config(&request, &url, &model).await {
                Ok(json_value) => {
                    bus.emit(EventType::Log("Prompt configuration generated successfully!".to_string()));
                    let contexte = json_value["contexte"].as_str().unwrap_or("").to_string();
                    let task = json_value["task"].as_str().unwrap_or("").to_string();
                    let objectif = json_value["objectif"].as_str().unwrap_or("").to_string();
                    let directives = json_value["directives"].as_str().unwrap_or("").to_string();
                    bus.emit(EventType::UpdateAllFields {
                        contexte,
                        task,
                        objectif,
                        directives,
                    });
                }
                Err(e) => {
                    bus.emit(EventType::Log(format!("Failed to generate prompts: {}", e)));
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
