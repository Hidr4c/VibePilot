pub mod session_memory;
pub mod loop_breaker;
pub mod helpers;
pub mod run_loop;
pub mod run_loop_helpers;
pub mod run_loop_utils;
pub mod decision_handler;

pub mod decision_helpers;
pub mod workspace;

pub use workspace::{Workspace, WorkspaceResolver, AdaptiveWorkspaceResolver};

pub use session_memory::{ActionStep, SessionMemory};

#[cfg(test)]
pub mod tests;

use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use crate::config::ConfigurationRepository;
use crate::event_bus::{EventBus, QueryEvent, NotificationEvent};
use crate::llm_client::LlmProvider;
use crate::peripheral_controller::{PeripheralInput, ScreenOffset};
use crate::screen_capture::ScreenCapturerTrait;
use crate::append_store::AppendOnlyStore;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub enum OrchestratorState {
    Idle,
    Running,
    Paused,
    AwaitingApproval,
    Reanchoring,
    Error(String),
}

pub struct VibePilotOrchestrator {
    pub config_repo: Arc<dyn ConfigurationRepository>,
    pub llm_client: Arc<dyn LlmProvider>,
    pub capturer: Arc<dyn ScreenCapturerTrait>,
    pub controller: Arc<dyn PeripheralInput>,
    pub wait_manager: Arc<dyn crate::services::wait_manager::WaitManagerTrait>,
    pub bus: EventBus,
    pub last_simulated_input_time: std::sync::Mutex<std::time::Instant>,
    pub journal: Option<AppendOnlyStore>,
    pub skip_wait: Arc<AtomicBool>,
    pub zoom_strategy: Arc<dyn crate::vision::ZoomStrategy>,
    pub zoom_cache: Mutex<crate::vision::ZoomCache>,
    pub step_mode: Arc<AtomicBool>,
    pub step_signal: Arc<AtomicBool>,
    pub undo_history: Mutex<Vec<crate::orchestrator::session_memory::ActionSnapshot>>,
    pub workspace_resolver: Arc<dyn WorkspaceResolver>,
    pub active_workspace_offsets: Mutex<Vec<ScreenOffset>>,
    pub cleanup_guard: CleanupGuard,
    pub state: Arc<Mutex<OrchestratorState>>,
    pub pii_masker: Arc<dyn crate::screen_capture::PiiMasker>,
    pub replay_manager: Arc<dyn crate::screen_capture::SessionReplayManager>,
    pub auto_calibrator: Arc<dyn crate::services::calibration::AutoCalibrator>,
    pub accessibility_parser: Arc<dyn crate::services::accessibility::AccessibilityParser>,
}

impl VibePilotOrchestrator {
    /// Creates a new orchestrator instance.
    pub fn new(
        config_repo: Arc<dyn ConfigurationRepository>,
        llm_client: Arc<dyn LlmProvider>,
        capturer: Arc<dyn ScreenCapturerTrait>,
        controller: Arc<dyn PeripheralInput>,
        wait_manager: Arc<dyn crate::services::wait_manager::WaitManagerTrait>,
        bus: EventBus,
    ) -> Self {
        let base_dir = config_repo.get_base_dir();
        let journal_path = base_dir.join("session_journal.enc");
        
        let cleanup_stop_flag = Arc::new(AtomicBool::new(false));
        let cleanup_stop_flag_clone = cleanup_stop_flag.clone();
        let mut cleanup_thread_handle = None;

        let journal = match AppendOnlyStore::new(journal_path) {
            Ok(store) => {
                // Initial cleanup asynchronously on start
                let store_clone = store.clone();
                let bus_clone = bus.clone();
                std::thread::spawn(move || {
                    match store_clone.retain_recent(7) {
                        Ok(removed) => {
                            if removed > 0 {
                                bus_clone.emit_notification(NotificationEvent::Log(format!("Journal cleanup: {} old entries removed", removed)));
                            }
                        }
                        Err(e) => {
                            bus_clone.emit_notification(NotificationEvent::Log(format!("⚠️ Journal cleanup failed: {}", e)));
                        }
                    }
                });

                // Periodic daily cleanup thread with clean shutdown support
                let store_clone_periodic = store.clone();
                let bus_clone_periodic = bus.clone();
                let handle = std::thread::spawn(move || {
                    let mut last_cleanup = std::time::Instant::now();
                    while !cleanup_stop_flag_clone.load(Ordering::Relaxed) {
                        std::thread::sleep(std::time::Duration::from_secs(1));
                        if last_cleanup.elapsed() >= std::time::Duration::from_secs(86400) {
                            if let Err(e) = store_clone_periodic.retain_recent(7) {
                                bus_clone_periodic.emit_notification(NotificationEvent::Log(format!("⚠️ Periodic journal cleanup failed: {}", e)));
                            }
                            last_cleanup = std::time::Instant::now();
                        }
                    }
                });
                cleanup_thread_handle = Some(handle);

                Some(store)
            }
            Err(e) => {
                bus.emit_notification(NotificationEvent::Log(format!("⚠️ Failed to initialize incremental journal: {}", e)));
                None
            }
        };

        let lang = config_repo.load_config().langue.clone();
        let lang_code = if lang == crate::config::LANG_FR { "fra+eng" } else { "eng+fra" };
        let pii_masker = crate::screen_capture::PiiMaskerFactory::create(lang_code.to_string());
        let replay_manager = crate::screen_capture::SessionReplayFactory::create(base_dir.clone());
        let auto_calibrator = crate::services::calibration::AutoCalibratorFactory::create();
        let accessibility_parser = crate::services::accessibility::AccessibilityParserFactory::create();

        Self {
            config_repo,
            llm_client,
            capturer,
            controller,
            wait_manager,
            bus,
            last_simulated_input_time: std::sync::Mutex::new(std::time::Instant::now()),
            journal,
            skip_wait: Arc::new(AtomicBool::new(false)),
            zoom_strategy: Arc::new(crate::vision::NullZoomStrategy::new()),
            zoom_cache: Mutex::new(crate::vision::ZoomCache::default()),
            step_mode: Arc::new(AtomicBool::new(false)),
            step_signal: Arc::new(AtomicBool::new(false)),
            undo_history: Mutex::new(Vec::new()),
            workspace_resolver: Arc::new(AdaptiveWorkspaceResolver::new()),
            active_workspace_offsets: Mutex::new(Vec::new()),
            cleanup_guard: CleanupGuard {
                stop_flag: cleanup_stop_flag,
                thread: Mutex::new(cleanup_thread_handle),
            },
            state: Arc::new(Mutex::new(OrchestratorState::Idle)),
            pii_masker,
            replay_manager,
            auto_calibrator,
            accessibility_parser,
        }
    }

    /// Sets the zoom strategy for precision targeting.
    pub fn with_zoom_strategy(self, strategy: Arc<dyn crate::vision::ZoomStrategy>) -> Self {
        Self { zoom_strategy: strategy, ..self }
    }

    /// Sets the workspace resolver strategy.
    pub fn with_workspace_resolver(self, resolver: Arc<dyn WorkspaceResolver>) -> Self {
        Self { workspace_resolver: resolver, ..self }
    }

    /// Forcefully skips any active wait operation.
    pub fn skip_current_wait(&self) {
        self.skip_wait.store(true, std::sync::atomic::Ordering::Relaxed);
    }

    /// Checks if the orchestrator is in pause mode.
    pub fn is_paused(&self) -> bool {
        matches!(*self.state.lock().unwrap(), OrchestratorState::Paused) ||
        self.bus.emit_query(QueryEvent::GetModePauseForcee) == "true"
    }

    /// Checks if the orchestrator has running status enabled.
    pub fn is_running(&self) -> bool {
        let bus_running = self.bus.emit_query(QueryEvent::GetOrchestratorRunning);
        if bus_running == "false" {
            return false;
        }
        if bus_running == "true" {
            return true;
        }
        let current = self.state.lock().unwrap().clone();
        !matches!(current, OrchestratorState::Idle)
    }

    /// Sets pause mode (for testing/configuration).
    pub fn set_pause_mode(&self, paused: bool) {
        let mut state = self.state.lock().unwrap();
        if paused {
            *state = OrchestratorState::Paused;
        } else if *state == OrchestratorState::Paused {
            *state = OrchestratorState::Running;
        }
        self.bus.register_query(
            QueryEvent::GetModePauseForcee,
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

        let wins = self.capturer.get_windows();
        for win in wins {
            if crate::screen_capture::is_window_title_match(&win.title, first_target) {
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

    pub(crate) fn get_active_offsets(
        &self,
        config: &crate::config::SavedConfig,
        escalate_to_desktop: bool,
    ) -> Vec<ScreenOffset> {
        if config.activer_roi {
            return vec![ScreenOffset {
                titre: "ROI".to_string(),
                left: config.roi_x,
                top: config.roi_y,
                width: config.roi_width,
                height: config.roi_height,
                y_offset: config.roi_y,
            }];
        }
        if config.activer_recadrage_workspace {
            let workspace = self.workspace_resolver.resolve_workspace(config, escalate_to_desktop, self.capturer.as_ref());
            vec![ScreenOffset {
                titre: workspace.title.clone(),
                left: workspace.bbox.left,
                top: workspace.bbox.top,
                width: workspace.bbox.width,
                height: workspace.bbox.height,
                y_offset: workspace.bbox.top,
            }]
        } else {
            self.get_target_offsets(config)
        }
    }
}


pub struct CleanupGuard {
    pub stop_flag: Arc<AtomicBool>,
    pub thread: Mutex<Option<std::thread::JoinHandle<()>>>,
}

impl Drop for CleanupGuard {
    fn drop(&mut self) {
        self.stop_flag.store(true, Ordering::Relaxed);
        if let Ok(mut lock) = self.thread.lock() {
            if let Some(handle) = lock.take() {
                let _ = handle.join();
            }
        }
    }
}

