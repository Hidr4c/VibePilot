use std::sync::Arc;
use crate::event_bus::EventBus;
use crate::orchestrator::VibePilotOrchestrator;
use crate::llm_client::LlmResponse;
use crate::config::SavedConfig;
use super::{MockConfigRepo, MockLlmClient, MockScreenCapturer, MockPeripheralController, MockWaitManager};

#[tokio::test]
async fn test_orchestrator_handle_decision_confirm_cancelled() {
    let (bus, _rx) = EventBus::new();
    let config = SavedConfig {
        auto_validate: false,
        ..SavedConfig::default()
    };
    let config_repo = Arc::new(MockConfigRepo {
        config: std::sync::Mutex::new(config),
    });

    let llm_client = Arc::new(MockLlmClient {
        next_response: std::sync::Mutex::new(LlmResponse::default()),
        decompose_response: std::sync::Mutex::new(String::new()),
        call_count: std::sync::Mutex::new(0),
    });
    let capturer = Arc::new(MockScreenCapturer {
        window_foreground_result: std::sync::Mutex::new(crate::screen_capture::WindowAnchorResult::AlreadyFocused),
    });
    let controller = Arc::new(MockPeripheralController {
        actions: std::sync::Mutex::new(vec![]),
    });

    let orchestrator = VibePilotOrchestrator::new(
        config_repo,
        llm_client,
        capturer,
        controller.clone(),
        Arc::new(MockWaitManager),
        bus.clone(),
    );

    // Mock confirmation query
    bus.register_query(
        crate::event_bus::QueryEvent::GetActionConfirmationStatus,
        Box::new(|_| "cancelled".to_string()),
    );

    let decision = LlmResponse {
        action: "CLICK_AND_TYPE".to_string(),
        relative_click_position: vec![0.1, 0.2],
        text_to_type: "hello".to_string(),
        ..LlmResponse::default()
    };
    let res = orchestrator.handle_decision(decision).await;
    assert!(res.is_ok());
}

#[tokio::test]
async fn test_orchestrator_handle_decision_confirm_timeout() {
    tokio::time::pause();
    let (bus, _rx) = EventBus::new();
    let config = SavedConfig {
        auto_validate: false,
        ..SavedConfig::default()
    };
    let config_repo = Arc::new(MockConfigRepo {
        config: std::sync::Mutex::new(config),
    });

    let llm_client = Arc::new(MockLlmClient {
        next_response: std::sync::Mutex::new(LlmResponse::default()),
        decompose_response: std::sync::Mutex::new(String::new()),
        call_count: std::sync::Mutex::new(0),
    });
    let capturer = Arc::new(MockScreenCapturer {
        window_foreground_result: std::sync::Mutex::new(crate::screen_capture::WindowAnchorResult::AlreadyFocused),
    });
    let controller = Arc::new(MockPeripheralController {
        actions: std::sync::Mutex::new(vec![]),
    });

    let orchestrator = VibePilotOrchestrator::new(
        config_repo,
        llm_client,
        capturer,
        controller.clone(),
        Arc::new(MockWaitManager),
        bus.clone(),
    );

    // Mock confirmation query as pending
    bus.register_query(
        crate::event_bus::QueryEvent::GetActionConfirmationStatus,
        Box::new(|_| "pending".to_string()),
    );

    let decision = LlmResponse {
        action: "CLICK_AND_TYPE".to_string(),
        relative_click_position: vec![0.1, 0.2],
        text_to_type: "hello".to_string(),
        ..LlmResponse::default()
    };

    // The handler will wait up to 60 seconds. Since tokio time is paused, it should time out and complete immediately in real time.
    let handle = tokio::spawn(async move {
        orchestrator.handle_decision(decision).await
    });

    let res = tokio::time::timeout(std::time::Duration::from_secs(65), handle).await;
    assert!(res.is_ok(), "The confirmation loop should complete and return Ok(()) due to timeout");
}

#[tokio::test]
async fn test_orchestrator_handle_decision_more_actions() {
    let (bus, _rx) = EventBus::new();
    let config = SavedConfig::default();
    let config_repo = Arc::new(MockConfigRepo {
        config: std::sync::Mutex::new(config),
    });

    let llm_client = Arc::new(MockLlmClient {
        next_response: std::sync::Mutex::new(LlmResponse::default()),
        decompose_response: std::sync::Mutex::new(String::new()),
        call_count: std::sync::Mutex::new(0),
    });
    let capturer = Arc::new(MockScreenCapturer {
        window_foreground_result: std::sync::Mutex::new(crate::screen_capture::WindowAnchorResult::AlreadyFocused),
    });
    let controller = Arc::new(MockPeripheralController {
        actions: std::sync::Mutex::new(vec![]),
    });

    let orchestrator = VibePilotOrchestrator::new(
        config_repo,
        llm_client,
        capturer,
        controller.clone(),
        Arc::new(MockWaitManager),
        bus,
    );

    // Test DRAG_DROP
    let decision = LlmResponse {
        action: "DRAG_DROP".to_string(),
        drag_from: vec![0.1, 0.2],
        drag_to: vec![0.3, 0.4],
        ..LlmResponse::default()
    };
    let res = orchestrator.handle_decision(decision).await;
    assert!(res.is_ok());

    // Test RIGHT_CLICK
    let decision = LlmResponse {
        action: "RIGHT_CLICK".to_string(),
        relative_click_position: vec![0.5, 0.5],
        ..LlmResponse::default()
    };
    let res = orchestrator.handle_decision(decision).await;
    assert!(res.is_ok());

    // Test DOUBLE_CLICK
    let decision = LlmResponse {
        action: "DOUBLE_CLICK".to_string(),
        relative_click_position: vec![0.5, 0.5],
        ..LlmResponse::default()
    };
    let res = orchestrator.handle_decision(decision).await;
    assert!(res.is_ok());

    // Test MIDDLE_CLICK
    let decision = LlmResponse {
        action: "MIDDLE_CLICK".to_string(),
        relative_click_position: vec![0.5, 0.5],
        ..LlmResponse::default()
    };
    let res = orchestrator.handle_decision(decision).await;
    assert!(res.is_ok());

    // Test MOUSE_MOVE_RELATIVE
    let decision = LlmResponse {
        action: "MOUSE_MOVE_RELATIVE".to_string(),
        relative_move: vec![0.05, 0.05],
        ..LlmResponse::default()
    };
    let res = orchestrator.handle_decision(decision).await;
    assert!(res.is_ok());

    // Test SUCCESS (returns stopping err)
    let decision = LlmResponse {
        action: "SUCCESS".to_string(),
        ..LlmResponse::default()
    };
    let res = orchestrator.handle_decision(decision).await;
    assert!(res.is_err());
    assert_eq!(res.unwrap_err(), "STOP_SUCCESS");
}

#[tokio::test]
async fn test_orchestrator_window_recovery_failure() {
    let (bus, _rx) = EventBus::new();
    let config = SavedConfig::default();
    let config_repo = Arc::new(MockConfigRepo {
        config: std::sync::Mutex::new(config),
    });

    let llm_client = Arc::new(MockLlmClient {
        next_response: std::sync::Mutex::new(LlmResponse::default()),
        decompose_response: std::sync::Mutex::new(String::new()),
        call_count: std::sync::Mutex::new(0),
    });
    // Configure screen capturer to say target is not visible, and ensure_window_foreground fails
    let capturer = Arc::new(MockScreenCapturer {
        window_foreground_result: std::sync::Mutex::new(crate::screen_capture::WindowAnchorResult::WindowNotFound),
    });
    let controller = Arc::new(MockPeripheralController {
        actions: std::sync::Mutex::new(vec![]),
    });

    let orchestrator = VibePilotOrchestrator::new(
        config_repo,
        llm_client,
        capturer,
        controller.clone(),
        Arc::new(MockWaitManager),
        bus,
    );

    let recovered = orchestrator.attempt_window_recovery("Target Window").await;
    assert!(!recovered);
}

#[tokio::test]
async fn test_orchestrator_run_loop_session_timeout() {
    tokio::time::pause();
    let (bus, _rx) = EventBus::new();
    
    let running = Arc::new(std::sync::atomic::AtomicBool::new(true));
    let running_clone = running.clone();
    bus.register_query(
        crate::event_bus::QueryEvent::GetOrchestratorRunning,
        Box::new(move |_| {
            if running_clone.load(std::sync::atomic::Ordering::Relaxed) {
                "true".to_string()
            } else {
                "false".to_string()
            }
        }),
    );

    bus.register_query(
        crate::event_bus::QueryEvent::GetModePauseForcee,
        Box::new(|_| "false".to_string()),
    );

    bus.register_query(
        crate::event_bus::QueryEvent::GetUserFeedback,
        Box::new(|_| String::new()),
    );

    let config = SavedConfig {
        objectif: "Test Objectif".to_string(),
        nom_modele: "mock-model".to_string(),
        url_api: "http://localhost:8000/v1".to_string(),
        moteur: "LM Studio".to_string(),
        ..SavedConfig::default()
    };

    let config_repo = Arc::new(MockConfigRepo {
        config: std::sync::Mutex::new(config),
    });

    let llm_client = Arc::new(MockLlmClient {
        next_response: std::sync::Mutex::new(LlmResponse::default()),
        decompose_response: std::sync::Mutex::new(String::new()),
        call_count: std::sync::Mutex::new(0),
    });

    let capturer = Arc::new(MockScreenCapturer {
        window_foreground_result: std::sync::Mutex::new(crate::screen_capture::WindowAnchorResult::AlreadyFocused),
    });

    let controller = Arc::new(MockPeripheralController {
        actions: std::sync::Mutex::new(vec![]),
    });

    let orchestrator = VibePilotOrchestrator::new(
        config_repo.clone(),
        llm_client.clone(),
        capturer.clone(),
        controller.clone(),
        Arc::new(MockWaitManager),
        bus.clone(),
    );

    // We start the loop, then advance time past 3600 seconds
    let handle = tokio::spawn(async move {
        orchestrator.run_loop().await
    });

    // Let the loop run its first iteration and initialize session_start
    tokio::time::sleep(std::time::Duration::from_millis(1)).await;

    // Advance virtual time by 3605 seconds
    tokio::time::advance(std::time::Duration::from_secs(3605)).await;

    let res = tokio::time::timeout(std::time::Duration::from_secs(5), handle).await;
    assert!(res.is_ok(), "run_loop should terminate");
    let join_res = res.unwrap().unwrap(); // Unwrap the JoinHandle, then get the Result<(), String>
    assert!(join_res.is_err());
    let err_str = join_res.err().unwrap();
    assert!(err_str.contains("Session timeout"));

    running.store(false, std::sync::atomic::Ordering::Relaxed);
}
