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
    #[serde(skip)]
    pub action_image: Option<Vec<u8>>,
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
    pub action_textures: std::collections::HashMap<usize, egui::TextureHandle>,

    pub macro_recorder: std::sync::Arc<dyn crate::macro_recorder::MacroRecorderTrait>,
    pub macro_sequence: crate::macro_recorder::MacroSequence,
    pub macro_session_manager: crate::macro_recorder::MacroSessionManager,
    pub macro_hotkey_config: crate::macro_recorder::HotkeyConfig,
    pub macro_iterations: u32,
    pub macro_start_delay_sec: u32,
    pub macro_record_start_delay_sec: u32,
    pub macro_countdown_remaining: std::sync::Arc<std::sync::Mutex<Option<u32>>>,
    pub macro_recording_countdown_remaining: std::sync::Arc<std::sync::Mutex<Option<u32>>>,
    pub macro_active_playback_step: std::sync::Arc<std::sync::Mutex<Option<(u32, usize)>>>,
    pub is_playing_macro: bool,
    pub macro_playback_cancel: std::sync::Arc<std::sync::atomic::AtomicBool>,
    pub selected_macro_node: Option<u64>,
    pub macro_undo_stack: Vec<crate::macro_recorder::MacroSequence>,
    pub macro_redo_stack: Vec<crate::macro_recorder::MacroSequence>,
    pub was_recording_macro: bool,
    pub node_binding_capture: Option<u64>,
    pub show_macro_save_confirm: bool,
    pub macro_script_text: String,
    pub macro_view_mode_text: bool,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Tab {
    GlobalConfig,
    PromptEditor,
    Console,
    TaskGraph,
    MacroRecorder,
    Setup,
}

impl VibePilotApp {
    pub fn new(egui_ctx: &egui::Context) -> Self {
        let (bus, receiver) = EventBus::new();
        
        let is_test = cfg!(test)
            || std::env::var("VIBEPILOT_TEST").is_ok()
            || std::thread::current().name().unwrap_or("main").contains("test")
            || std::env::args().skip(1).any(|arg| arg.contains("test"));

        let base_dir = match crate::config::load_bootstrap_config().storage_dir {
            Some(dir_str) if !dir_str.is_empty() => std::path::PathBuf::from(&dir_str),
            _ => {
                if is_test {
                    let thread_name = std::thread::current().name().unwrap_or("main").to_string();
                    let safe_name = thread_name.chars()
                        .filter(|c| c.is_alphanumeric() || *c == '_' || *c == ':')
                        .map(|c| if c == ':' { '_' } else { c })
                        .collect::<String>();
                    std::env::temp_dir().join(format!("vibepilot_test_{}", safe_name))
                } else {
                    std::env::current_dir().unwrap_or_default()
                }
            }
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
            action_textures: std::collections::HashMap::new(),
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

            macro_recorder: crate::macro_recorder::MacroRecorderFactory::create(),
            macro_sequence: crate::macro_recorder::MacroSequence::new("Run #1"),
            macro_session_manager: crate::macro_recorder::MacroSessionManager::new(),
            macro_hotkey_config: crate::macro_recorder::HotkeyConfig::default(),
            macro_iterations: 1,
            macro_start_delay_sec: 0,
            macro_record_start_delay_sec: 0,
            macro_countdown_remaining: std::sync::Arc::new(std::sync::Mutex::new(None)),
            macro_recording_countdown_remaining: std::sync::Arc::new(std::sync::Mutex::new(None)),
            macro_active_playback_step: std::sync::Arc::new(std::sync::Mutex::new(None)),
            is_playing_macro: false,
            macro_playback_cancel: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            selected_macro_node: None,
            macro_undo_stack: Vec::new(),
            macro_redo_stack: Vec::new(),
            was_recording_macro: false,
            node_binding_capture: None,
            show_macro_save_confirm: false,
            macro_script_text: String::new(),
            macro_view_mode_text: false,
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
            app.load_profile(&last_profile);
        } else {
            app.selected_profile = app.get_first_available_profile_name();
            app.quick_start_profile_name = app.get_first_available_profile_name();
        }

        if let Some(zoom) = app.current_config.zoom_facteur {
            egui_ctx.set_pixels_per_point(zoom);
        }

        let sessions_path = app.config_repo.get_save_path().parent().unwrap_or(std::path::Path::new(".")).join("macro_sessions.json");
        if let Ok(loaded_sessions) = crate::macro_recorder::storage::MacroStorage::load_sessions(&sessions_path) {
            if !loaded_sessions.runs.is_empty() {
                app.macro_session_manager = loaded_sessions;
                if let Some(r) = app.macro_session_manager.active_run() {
                    app.macro_sequence = r.sequence.clone();
                    app.macro_recorder.set_sequence(app.macro_sequence.clone());
                    app.macro_iterations = r.iterations;
                    app.macro_start_delay_sec = r.start_delay_sec;
                    app.macro_record_start_delay_sec = r.record_start_delay_sec;
                }
            }
        }

        let hotkeys_path = app.config_repo.get_save_path().parent().unwrap_or(std::path::Path::new(".")).join("macro_hotkeys.json");
        if let Ok(loaded_hotkeys) = crate::macro_recorder::storage::MacroStorage::load_hotkeys(&hotkeys_path) {
            app.macro_hotkey_config = loaded_hotkeys.clone();
            app.macro_recorder.set_hotkeys(loaded_hotkeys);
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
            self.structured_steps.clear();
            self.action_textures.clear();
            self.logs.clear();
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
        use std::collections::HashSet;
        let existing_profiles: HashSet<String> = self.config_repo.list_profiles().into_iter().collect();
        let mut i = 1;
        loop {
            let name = format!("profile_{:02}", i);
            if !existing_profiles.contains(&name) {
                return name;
            }
            i += 1;
        }
    }

    pub fn is_profile_modified(&self) -> bool {
        if let Some(saved) = self.config_repo.load_profile(&self.selected_profile) {
            let mut c1 = self.current_config.clone();
            let mut c2 = saved;
            c1.dernier_profil = String::new();
            c2.dernier_profil = String::new();
            c1.prompt_reprise = None;
            c2.prompt_reprise = None;
            c1 != c2
        } else {
            !self.selected_profile.is_empty()
        }
    }

    pub fn push_macro_undo(&mut self) {
        self.macro_undo_stack.push(self.macro_sequence.clone());
        if self.macro_undo_stack.len() > 50 {
            self.macro_undo_stack.remove(0);
        }
        self.macro_redo_stack.clear();
    }

    pub fn undo_macro_action(&mut self) -> bool {
        if let Some(prev) = self.macro_undo_stack.pop() {
            self.macro_redo_stack.push(self.macro_sequence.clone());
            self.macro_sequence = prev.clone();
            if let Some(run) = self.macro_session_manager.active_run_mut() {
                run.sequence = prev.clone();
            }
            self.macro_recorder.set_sequence(prev);
            self.save_macro_sessions_now();
            true
        } else {
            false
        }
    }

    pub fn redo_macro_action(&mut self) -> bool {
        if let Some(next) = self.macro_redo_stack.pop() {
            self.macro_undo_stack.push(self.macro_sequence.clone());
            self.macro_sequence = next.clone();
            if let Some(run) = self.macro_session_manager.active_run_mut() {
                run.sequence = next.clone();
            }
            self.macro_recorder.set_sequence(next);
            self.save_macro_sessions_now();
            true
        } else {
            false
        }
    }

    pub fn save_macro_sessions_now(&mut self) {
        if let Some(run) = self.macro_session_manager.active_run_mut() {
            run.sequence = self.macro_sequence.clone();
            run.iterations = self.macro_iterations;
            run.start_delay_sec = self.macro_start_delay_sec;
            run.record_start_delay_sec = self.macro_record_start_delay_sec;
        }
        let rec_hotkeys = self.macro_recorder.get_hotkeys();
        if rec_hotkeys != crate::macro_recorder::HotkeyConfig::default() {
            self.macro_hotkey_config = rec_hotkeys;
        }
        let dir = self.config_repo.get_save_path().parent().unwrap_or(std::path::Path::new(".")).to_path_buf();
        let _ = crate::macro_recorder::storage::MacroStorage::save_sessions(&self.macro_session_manager, dir.join("macro_sessions.json"));
        let _ = crate::macro_recorder::storage::MacroStorage::save_hotkeys(&self.macro_hotkey_config, dir.join("macro_hotkeys.json"));
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

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.save_macro_sessions_now();
    }
}
