use std::sync::Arc;
use crate::event_bus::EventBus;
use crate::orchestrator::VibePilotOrchestrator;
use crate::llm_client::LlmResponse;
use crate::config::SavedConfig;
use super::{MockConfigRepo, MockLlmClient, MockScreenCapturer, MockPeripheralController, MockWaitManager};

#[tokio::test]
async fn test_orchestrator_run_loop_success() {
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
        pipeline_vision_avance: false,
        activer_reflexion: false,
        activer_systeme_fast_slow: false,
        ..SavedConfig::default()
    };

    let config_repo = Arc::new(MockConfigRepo {
        config: std::sync::Mutex::new(config),
    });

    let llm_client = Arc::new(MockLlmClient {
        next_response: std::sync::Mutex::new(LlmResponse {
            status_display: "Done".to_string(),
            action: "SUCCESS".to_string(),
            relative_click_position: vec![0.5, 0.5],
            text_to_type: String::new(),
            scroll_value: 0,
            wait_seconds: 0,
            report: Some("Objective accomplished".to_string()),
            ..LlmResponse::default()
        }),
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

    let handle = tokio::spawn(async move {
        orchestrator.run_loop().await
    });

    let res = tokio::time::timeout(std::time::Duration::from_secs(30), handle).await;
    assert!(res.is_ok(), "run_loop should terminate successfully");
    
    running.store(false, std::sync::atomic::Ordering::Relaxed);
}

#[tokio::test]
async fn test_orchestrator_handle_decision_methods() {
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

    // Test CLICK_AND_TYPE
    let decision = LlmResponse {
        action: "CLICK_AND_TYPE".to_string(),
        relative_click_position: vec![0.1, 0.2],
        text_to_type: "hello".to_string(),
        ..LlmResponse::default()
    };
    let res = orchestrator.handle_decision(decision).await;
    assert!(res.is_ok());

    // Test SCROLL
    let decision = LlmResponse {
        action: "SCROLL".to_string(),
        scroll_value: 100,
        ..LlmResponse::default()
    };
    let res = orchestrator.handle_decision(decision).await;
    assert!(res.is_ok());

    // Test WAIT
    let decision = LlmResponse {
        action: "WAIT".to_string(),
        wait_seconds: 1,
        ..LlmResponse::default()
    };
    let res = orchestrator.handle_decision(decision).await;
    assert!(res.is_ok());

    // Test KEY_COMBO
    let decision = LlmResponse {
        action: "KEY_COMBO".to_string(),
        keys_to_press: vec!["ctrl".to_string(), "c".to_string()],
        ..LlmResponse::default()
    };
    let res = orchestrator.handle_decision(decision).await;
    assert!(res.is_ok());

    // Test CLIPBOARD
    let decision = LlmResponse {
        action: "CLIPBOARD".to_string(),
        clipboard_op: "copy".to_string(),
        ..LlmResponse::default()
    };
    let res = orchestrator.handle_decision(decision).await;
    assert!(res.is_ok());
}

#[tokio::test]
async fn test_orchestrator_run_loop_with_task_graph() {
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
        decomposer_taches: true,
        max_tentatives_par_tache: 3,
        pipeline_vision_avance: false,
        activer_reflexion: false,
        activer_systeme_fast_slow: false,
        ..SavedConfig::default()
    };

    let config_repo = Arc::new(MockConfigRepo {
        config: std::sync::Mutex::new(config),
    });

    let llm_client = Arc::new(MockLlmClient {
        next_response: std::sync::Mutex::new(LlmResponse {
            status_display: "Done Task".to_string(),
            action: "SUCCESS".to_string(),
            relative_click_position: vec![0.5, 0.5],
            report: Some("Task done".to_string()),
            ..LlmResponse::default()
        }),
        decompose_response: std::sync::Mutex::new("[{\"id\": 1, \"description\": \"Task 1\", \"depends_on\": []}]".to_string()),
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

    let handle = tokio::spawn(async move {
        orchestrator.run_loop().await
    });

    let res = tokio::time::timeout(std::time::Duration::from_secs(30), handle).await;
    assert!(res.is_ok(), "run_loop with task graph should terminate successfully");
    
    running.store(false, std::sync::atomic::Ordering::Relaxed);
}

#[tokio::test]
async fn test_orchestrator_run_loop_fast_slow_and_reflection() {
    tokio::time::pause();
    let (bus, rx) = EventBus::new();
    
    tokio::spawn(async move {
        while let Ok(event) = rx.recv_async().await {
            println!("[TEST BUS] {:?}", event);
        }
    });
    
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
        activer_systeme_fast_slow: true,
        activer_reflexion: true,
        pipeline_vision_avance: true,
        fenetres_surveillees: vec!["Mock Window".to_string()],
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
    
    *llm_client.next_response.lock().unwrap() = LlmResponse {
        status_display: "Done".to_string(),
        action: "SUCCESS".to_string(),
        relative_click_position: vec![0.5, 0.5],
        report: Some("Done".to_string()),
        ..LlmResponse::default()
    };

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

    let handle = tokio::spawn(async move {
        let run_res = orchestrator.run_loop().await;
        println!("[TEST LOOP END] Result: {:?}", run_res);
        run_res
    });

    let res = tokio::time::timeout(std::time::Duration::from_secs(30), handle).await;
    println!("[TEST TIMEOUT END] Result: {:?}", res);
    assert!(res.is_ok(), "run_loop should terminate successfully");
    
    running.store(false, std::sync::atomic::Ordering::Relaxed);
}

#[tokio::test]
async fn test_orchestrator_handle_decision_confirm_approved() {
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
        Box::new(|_| "approved".to_string()),
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
