//! Main application state for VibePilot.
//!
//! Manages the egui app lifecycle, event processing, orchestrator
//! lifecycle, and config persistence.

use crate::config::{ActionLogger, ConfigRepository, SavedConfig, EnginePresets, ConfigurationRepository};
use crate::event_bus::{EventBus, NotificationEvent, CommandEvent, QueryEvent};
use crate::orchestrator::VibePilotOrchestrator;
use crate::app_services::set_sleep_prevented;
use eframe::egui;
use std::sync::Arc;
use tokio::runtime::Runtime;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Default)]
pub struct StructuredStep {
    pub action_type: String,
    pub tooltip: String,
    pub logs: Vec<String>,
    pub report: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct PendingAction {
    pub action: String,
    pub text_to_type: String,
    pub scroll_value: i32,
}

pub struct VibePilotApp {
    pub structured_steps: Vec<StructuredStep>,
    pub config_repo: Arc<dyn ConfigurationRepository>,
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
    pub(crate) event_receiver: flume::Receiver<crate::event_bus::Event>,
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
    pub show_save_success_popup: Option<String>,

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
    Console,
    TaskGraph,
    Setup,
}

impl VibePilotApp {
    pub fn new(egui_ctx: &egui::Context) -> Self {
        let (bus, receiver) = EventBus::new();
        
        let base_dir = match crate::config::load_bootstrap_config().storage_dir {
            Some(dir_str) if !dir_str.is_empty() => std::path::PathBuf::from(&dir_str),
            _ => std::env::current_dir().unwrap_or_default(),
        };
        let custom_dir_str = base_dir.to_string_lossy().to_string();

        let config_repo = crate::config::ConfigRepositoryFactory::create(base_dir.clone());
        let log_path = base_dir.join("vibepilot.log");
        let action_logger = Arc::new(ActionLogger::new(log_path));

        let capturer = crate::screen_capture::ScreenCapturerFactory::create();
        let controller = crate::peripheral_controller::PeripheralControllerFactory::create(capturer.clone());
        let llm_client = crate::llm_client::LlmClientFactory::create();
        let wait_manager = crate::services::WaitManagerFactory::create();

        let orchestrator = Arc::new(VibePilotOrchestrator::new(
            config_repo.clone(),
            llm_client.clone(),
            capturer.clone(),
            controller.clone(),
            wait_manager.clone(),
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
            structured_steps: Vec::new(),
            config_repo,
            action_logger,
            bus: bus.clone(),
            orchestrator,
            rt: Runtime::new().expect("FATAL: failed to create Tokio async runtime — VibePilot cannot operate without it"),
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
            show_rename_profile: Option::None,
            rename_profile_new_name: String::new(),
            quick_start_profile_name: String::new(),
            is_generating_prompts: false,
            report_content: String::new(),
            bottom_tab: 0,
            auto_scroll_logs: true,

            selected_profile_to_export: String::new(),
            selected_engine_to_export: String::new(),
            show_profile_ready_popup: Option::None,
            show_save_success_popup: Option::None,

            pending_action: Option::None,
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
            QueryEvent::GetActionConfirmationStatus,
            Box::new(move |_| {
                status_clone.lock()
                    .map(|g| g.clone())
                    .unwrap_or_else(|_| "pending".to_string())
            }),
        );

        bus.register_query(
            QueryEvent::GetUserFeedback,
            Box::new(move |_| {
                if let Ok(mut guard) = feedback_clone_for_query.lock() {
                    let feedback = guard.clone();
                    guard.clear();
                    feedback
                } else {
                    String::new()
                }
            }),
        );

        bus.register_query(
            QueryEvent::GetModePauseForcee,
            Box::new(move |_| {
                if pause_clone.load(std::sync::atomic::Ordering::Relaxed) {
                    "true".to_string()
                } else {
                    "false".to_string()
                }
            }),
        );

        bus.register_query(
            QueryEvent::GetOrchestratorRunning,
            Box::new(move |_| {
                if running_clone.load(std::sync::atomic::Ordering::Relaxed) {
                    "true".to_string()
                } else {
                    "false".to_string()
                }
            }),
        );

        // Start lightweight HTTP RPC server on background task
        let mut port = 4040;
        let mut token = None;
        let mut args = std::env::args();
        while let Some(arg) = args.next() {
            if arg == "--port" {
                if let Some(port_str) = args.next() {
                    if let Ok(p) = port_str.parse::<u16>() {
                        port = p;
                    }
                }
            } else if arg == "--token" {
                if let Some(token_str) = args.next() {
                    token = Some(token_str);
                }
            }
        }
        let api_server = crate::services::api::WorkerApiServer::new(bus.clone(), port, token);
        app.rt.spawn(async move {
            let _ = api_server.start().await;
        });

        app
    }

    /// Translation helper method.
    pub fn t(&self, key: &str) -> String {
        let is_fr = self.current_config.langue == "Français";
        let map = if is_fr { &self.lexicon_fr } else { &self.lexicon_en };
        map.get(key).copied().unwrap_or(key).to_string()
    }

    pub fn start_orchestrator(&mut self) {
        if self.is_running {
            self.action_logger.info("Orchestrator is already running. Ignoring duplicate start request.");
            return;
        }
        self.is_running = true;
        self.running_ref.store(true, std::sync::atomic::Ordering::Relaxed);
        if self.current_config.garder_ecran_actif {
            set_sleep_prevented(true);
        }
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
                Err(e) if e == "LOOP_DETECTED" => {
                    action_logger.info("Orchestration paused due to loop detection");
                }
                Err(e) => {
                    action_logger.error(&format!("Orchestrator error: {}", e));
                    println!("Orchestrator error: {}", e);
                }
            }
            orchestrator.bus.emit_command(crate::event_bus::CommandEvent::StopOrchestrator);
        });
    }

    pub fn stop_orchestrator(&mut self) {
        self.is_running = false;
        self.running_ref.store(false, std::sync::atomic::Ordering::Relaxed);
        set_sleep_prevented(false);
    }

    pub fn load_profile(&mut self, name: &str) -> bool {
        if let Some(mut cfg) = self.config_repo.load_profile(name) {
            self.stop_orchestrator();
            self.action_history.clear();
            self.logs.clear();
            self.structured_steps.clear();
            self.report_content.clear();
            self.user_feedback_input.clear();
            if let Ok(mut guard) = self.accumulated_user_feedback.lock() {
                guard.clear();
            }
            self.status_text = "Ready".to_string();
            self.status_color = "grey".to_string();
            cfg.dernier_profil = name.to_string();
            self.current_config = cfg.clone();
            self.last_saved_config = cfg;
            self.selected_profile = name.to_string();
            self.quick_start_profile_name = name.to_string();
            self.bus.emit_notification(NotificationEvent::Log(format!("Profile '{}' loaded. Session reset.", name)));
            true
        } else {
            false
        }
    }

    pub fn get_pause_ref(&self) -> &std::sync::Arc<std::sync::atomic::AtomicBool> {
        &self.pause_ref
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
        if self.current_config != self.last_saved_config {
            self.config_repo.save_config(&self.current_config);
            self.last_saved_config = self.current_config.clone();
        }
        ctx.request_repaint_after(std::time::Duration::from_millis(250));
    }
}
