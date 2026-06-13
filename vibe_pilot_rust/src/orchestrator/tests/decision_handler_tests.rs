use super::*;
use crate::orchestrator::VibePilotOrchestrator;
use crate::event_bus::EventBus;
use crate::llm_client::{LlmResponse, LlmClient};
use crate::config::{ConfigManagement, ProfileManagement, EngineManagement, StateManagement, ConfigPaths, SavedConfig};
use std::sync::Arc;

fn make_orchestrator() -> VibePilotOrchestrator {
    let (bus, _rx) = EventBus::new();
    let capturer = Arc::new(crate::screen_capture::MockScreenCapturer::new());
    let controller = Arc::new(crate::peripheral_controller::MockPeripheralInput::new());
    let llm_client = Arc::new(LlmClient::new());

    struct TestConfigRepo;
    impl ConfigManagement for TestConfigRepo {
        fn load_config(&self) -> SavedConfig {
            let mut c = SavedConfig::default();
            c.auto_validate = true;
            c.auto_validate_dangerous = false;
            c.verifier_placement_souris = false;
            c.detecter_activite_utilisateur = false;
            c.fenetres_surveillees = vec!["ALL SCREENS (Virtual Desktop)".to_string()];
            c
        }
        fn save_config(&self, _: &SavedConfig) {}
        fn save_now(&self) -> Result<(), String> { Ok(()) }
        fn get_save_path(&self) -> std::path::PathBuf { std::env::temp_dir().join("save.enc") }
        fn get_key_path(&self) -> std::path::PathBuf { std::env::temp_dir().join("key.enc") }
    }
    impl ProfileManagement for TestConfigRepo {
        fn load_profile(&self, _: &str) -> Option<SavedConfig> { None }
        fn save_profile(&self, _: &str, _: &SavedConfig) -> bool { false }
        fn delete_profile(&self, _: &str) -> bool { false }
        fn list_profiles(&self) -> Vec<String> { vec![] }
        fn get_profiles_dir(&self) -> std::path::PathBuf { std::env::temp_dir().join("profiles") }
        fn export_profiles(&self, _: &std::path::Path) -> Result<(), String> { Ok(()) }
        fn import_profiles(&self, _: &std::path::Path) -> Result<usize, String> { Ok(0) }
        fn export_single_profile(&self, _: &str, _: &std::path::Path) -> Result<(), String> { Ok(()) }
        fn import_single_profile(&self, _: &std::path::Path) -> Result<String, String> { Ok("test".to_string()) }
    }
    impl EngineManagement for TestConfigRepo {
        fn load_engines(&self) -> crate::config::EnginePresets { Default::default() }
        fn save_engines(&self, _: &crate::config::EnginePresets) {}
        fn get_engines_path(&self) -> std::path::PathBuf { std::env::temp_dir().join("engines.enc") }
        fn export_engines(&self, _: &std::path::Path) -> Result<(), String> { Ok(()) }
        fn import_engines(&self, _: &std::path::Path) -> Result<(), String> { Ok(()) }
        fn export_single_engine(&self, _: &str, _: &std::path::Path) -> Result<(), String> { Ok(()) }
        fn import_single_engine(&self, _: &std::path::Path) -> Result<String, String> { Ok("test".to_string()) }
    }
    impl StateManagement for TestConfigRepo {
        fn load_task_graph(&self) -> Option<crate::memory::TaskGraph> { None }
        fn save_task_graph(&self, _: Option<crate::memory::TaskGraph>) {}
        fn export_config(&self, _: &std::path::Path) -> Result<(), String> { Ok(()) }
        fn import_config(&self, _: &std::path::Path) -> Result<(), String> { Ok(()) }
    }
    impl ConfigPaths for TestConfigRepo {
        fn get_base_dir(&self) -> std::path::PathBuf { std::env::temp_dir() }
        fn get_store_path(&self) -> std::path::PathBuf { std::env::temp_dir().join("vibepilot_data.enc") }
    }

    VibePilotOrchestrator::new(
        Arc::new(TestConfigRepo),
        llm_client,
        capturer,
        controller,
        Arc::new(crate::services::wait_manager::WaitManager::new()),
        bus,
    )
}

#[test]
fn test_wait_if_user_active_returns_quickly_when_detection_disabled() {
    let orch = make_orchestrator();
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        orch.wait_if_user_active().await;
    });
}

#[test]
fn test_wait_if_user_active_returns_quickly_when_paused() {
    let orch = make_orchestrator();
    orch.set_pause_mode(true);
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        orch.wait_if_user_active().await;
    });
}

#[test]
fn test_attempt_window_recovery_returns_false_when_no_exe_mapped() {
    let orch = make_orchestrator();
    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(async {
        orch.attempt_window_recovery("NonExistentWindowXYZ123").await
    });
    assert!(!result);
}

#[test]
fn test_attempt_window_recovery_with_known_window() {
    let orch = make_orchestrator();
    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(async {
        orch.attempt_window_recovery("Google Chrome").await
    });
    let _ = result;
}

#[test]
fn test_handle_decision_success_returns_stop() {
    let orch = make_orchestrator();
    let rt = tokio::runtime::Runtime::new().unwrap();
    let response = LlmResponse {
        status_display: "Done".to_string(),
        action: "SUCCESS".to_string(),
        ..Default::default()
    };
    let result = rt.block_on(async {
        orch.handle_decision(response).await
    });
    assert_eq!(result.unwrap_err(), "STOP_SUCCESS");
}

#[test]
fn test_handle_decision_fail_returns_ok() {
    let orch = make_orchestrator();
    let rt = tokio::runtime::Runtime::new().unwrap();
    let response = LlmResponse {
        status_display: "Failed".to_string(),
        action: "FAIL".to_string(),
        wait_seconds: 0,
        ..Default::default()
    };
    let result = rt.block_on(async {
        orch.handle_decision(response).await
    });
    assert!(result.is_ok());
}

#[test]
fn test_handle_decision_wait_returns_ok() {
    let orch = make_orchestrator();
    let rt = tokio::runtime::Runtime::new().unwrap();
    let response = LlmResponse {
        status_display: "Waiting".to_string(),
        action: "WAIT".to_string(),
        wait_seconds: 0,
        ..Default::default()
    };
    let result = rt.block_on(async {
        orch.handle_decision(response).await
    });
    assert!(result.is_ok());
}

#[test]
fn test_handle_decision_unknown_action_logs() {
    let orch = make_orchestrator();
    let rt = tokio::runtime::Runtime::new().unwrap();
    let response = LlmResponse {
        status_display: "Unknown".to_string(),
        action: "UNKNOWN_ACTION".to_string(),
        ..Default::default()
    };
    let result = rt.block_on(async {
        orch.handle_decision(response).await
    });
    assert!(result.is_ok());
}
