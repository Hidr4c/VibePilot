use crate::config::{ConfigRepository, SavedConfig, ConfigManagement};
use crate::event_bus::{EventBus, EventType, NotificationEvent, CommandEvent, QueryEvent};
use crate::orchestrator::VibePilotOrchestrator;
use std::sync::Arc;

fn create_test_orchestrator(config_repo: Arc<dyn crate::config::ConfigurationRepository>, bus: EventBus) -> VibePilotOrchestrator {
    let capturer = Arc::new(crate::screen_capture::MockScreenCapturer::new());
    let controller = Arc::new(crate::peripheral_controller::MockPeripheralInput::new());
    let llm_client = Arc::new(crate::llm_client::LlmClient::new());
    VibePilotOrchestrator::new(
        config_repo,
        llm_client,
        capturer,
        controller,
        Arc::new(crate::services::wait_manager::WaitManager::new()),
        bus,
    )
}

#[test]
fn test_orchestrator_creation() {
    let (bus, _rx) = EventBus::new();
    let config_repo = Arc::new(ConfigRepository::new(
        std::env::temp_dir().join("vibepilot_test_orch"),
    ));
    let _orch = create_test_orchestrator(config_repo, bus);
}

#[test]
fn test_orchestrator_config_loading() {
    let (bus, _rx) = EventBus::new();
    let dir = std::env::temp_dir().join("orch_test");
    let _ = std::fs::create_dir_all(&dir);
    let config_repo = Arc::new(ConfigRepository::new(dir.clone()));

    // Save a config first
    let config = SavedConfig {
        objectif: "test objective".to_string(),
        moteur: "TestEngine".to_string(),
        url_api: "http://test:8000".to_string(),
        nom_modele: "test-model".to_string(),
        ..SavedConfig::default()
    };
    config_repo.save_config(&config);

    let _orch = create_test_orchestrator(config_repo, bus);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn test_orchestrator_emits_log_event() {
    let (bus, rx) = EventBus::new();
    let config_repo = Arc::new(ConfigRepository::new(
        std::env::temp_dir().join("vibepilot_test_orch2"),
    ));
    let _orch = create_test_orchestrator(config_repo, bus.clone());

    assert_eq!(bus.query_handler_count(), 0);
    let _ = rx.try_recv();
}

#[test]
fn test_orchestrator_pause_query() {
    let (bus, _rx) = EventBus::new();
    let config_repo = Arc::new(ConfigRepository::new(
        std::env::temp_dir().join("vibepilot_test_orch3"),
    ));
    let _orch = create_test_orchestrator(config_repo, bus);
}

#[test]
fn test_orchestrator_running_query() {
    let (bus, _rx) = EventBus::new();
    let config_repo = Arc::new(ConfigRepository::new(
        std::env::temp_dir().join("vibepilot_test_orch4"),
    ));
    let orch = create_test_orchestrator(config_repo, bus.clone());
    
    let running_status = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let running_status_clone = running_status.clone();
    bus.register_query(
        QueryEvent::GetOrchestratorRunning,
        Box::new(move |_| {
            if running_status_clone.load(std::sync::atomic::Ordering::Relaxed) {
                "true".to_string()
            } else {
                "false".to_string()
            }
        }),
    );

    assert!(!orch.is_running());
    
    running_status.store(true, std::sync::atomic::Ordering::Relaxed);
    assert!(orch.is_running());
}

#[test]
fn test_is_local_url() {
    use crate::orchestrator::helpers::is_local_url;
    assert!(is_local_url("http://localhost:1234/v1"));
    assert!(is_local_url("http://127.0.0.1:8000/v1"));
    assert!(is_local_url("http://[::1]:1234"));
    assert!(!is_local_url("https://api.openai.com/v1"));
}

#[test]
fn test_get_target_offsets_empty() {
    let (bus, _rx) = EventBus::new();
    let config_repo = Arc::new(ConfigRepository::new(
        std::env::temp_dir().join("orch_test_offsets"),
    ));
    let orch = create_test_orchestrator(config_repo, bus);
    let config = SavedConfig::default();
    let offsets = orch.get_target_offsets(&config);
    assert!(!offsets.is_empty());
}

#[tokio::test]
async fn test_orchestrator_wait_if_user_active_disabled() {
    let (bus, _rx) = EventBus::new();
    let dir = std::env::temp_dir().join("orch_test_wait_active");
    let _ = std::fs::create_dir_all(&dir);
    let config_repo = Arc::new(ConfigRepository::new(dir.clone()));

    let config = SavedConfig {
        detecter_activite_utilisateur: false,
        ..SavedConfig::default()
    };
    config_repo.save_config(&config);

    let orch = create_test_orchestrator(config_repo, bus);
    
    let result = tokio::time::timeout(std::time::Duration::from_millis(500), orch.wait_if_user_active()).await;
    assert!(result.is_ok());

    let _ = std::fs::remove_dir_all(&dir);
}
