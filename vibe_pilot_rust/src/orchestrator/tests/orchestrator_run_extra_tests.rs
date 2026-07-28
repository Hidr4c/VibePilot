use std::sync::Arc;
use crate::event_bus::EventBus;
use crate::orchestrator::VibePilotOrchestrator;
use crate::llm_client::LlmResponse;
use crate::config::{SavedConfig, StateManagement};
use crate::peripheral_controller::CoordinateMapping;
use super::{MockConfigRepo, MockLlmClient, MockScreenCapturer, MockPeripheralController, MockWaitManager};

#[tokio::test]
async fn test_orchestrator_undo_history_recording_and_pop() {
    let (bus, _rx) = EventBus::new();
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
        config_repo,
        llm_client,
        capturer,
        controller.clone(),
        Arc::new(MockWaitManager),
        bus,
    );

    // Initial state: history is empty
    {
        let history = orchestrator.undo_history.lock().unwrap();
        assert!(history.is_empty());
    }

    let snapshot = crate::orchestrator::session_memory::ActionSnapshot {
        timestamp: "12:00:00".to_string(),
        action_type: "CLICK_AND_TYPE".to_string(),
        action_description: "Clicked something".to_string(),
        relative_click_position: vec![0.5, 0.5],
        text_typed: Some("hello".to_string()),
        scroll_value: 0,
        screenshot_hash: 0,
        window_title: "Mock Window".to_string(),
        reflection_feedback: None,
        active_task_id: Some(42),
    };

    {
        let mut history = orchestrator.undo_history.lock().unwrap();
        history.push(snapshot);
    }

    // Now verify popping
    let popped = {
        let mut history = orchestrator.undo_history.lock().unwrap();
        history.pop()
    };
    assert!(popped.is_some());
    let popped_val = popped.unwrap();
    assert_eq!(popped_val.action_type, "CLICK_AND_TYPE");
    assert_eq!(popped_val.active_task_id, Some(42));
}

#[tokio::test]
async fn test_orchestrator_run_loop_task_graph_retries() {
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
        decomposer_taches: true,
        max_tentatives_par_tache: 3,
        pipeline_vision_avance: false,
        activer_reflexion: true, // Enable reflection to trigger NoEffect on identical mock frames
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
    assert!(res.is_ok(), "run_loop with retries should terminate successfully");
    
    let final_graph = config_repo.load_task_graph().unwrap();
    let task = final_graph.tasks.get(&crate::memory::TaskId(1)).unwrap();
    assert_eq!(task.status, crate::memory::TaskStatus::Completed);
    // Initial attempt + 1 retry = 2 attempts total
    assert_eq!(task.attempts, 2);

    running.store(false, std::sync::atomic::Ordering::Relaxed);
}

#[tokio::test]
async fn test_orchestrator_programmatic_coordinate_calibration() {
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
        decomposer_taches: true,
        max_tentatives_par_tache: 5,
        pipeline_vision_avance: false,
        activer_reflexion: true,
        activer_systeme_fast_slow: false,
        ..SavedConfig::default()
    };

    let config_repo = Arc::new(MockConfigRepo {
        config: std::sync::Mutex::new(config),
    });

    // Mock client that ALWAYS returns CLICK_AND_TYPE at [0.5, 0.5]
    let llm_client = Arc::new(MockLlmClient {
        next_response: std::sync::Mutex::new(LlmResponse {
            status_display: "Mock Clicking".to_string(),
            action: "CLICK_AND_TYPE".to_string(),
            relative_click_position: vec![0.5, 0.5],
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

    // Advance loop in virtual time so multiple iterations run
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    tokio::time::advance(std::time::Duration::from_secs(10)).await;
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    tokio::time::advance(std::time::Duration::from_secs(10)).await;

    // Timeout the run loop
    running.store(false, std::sync::atomic::Ordering::Relaxed);
    let _ = handle.await;

    let actions = controller.actions.lock().unwrap();
    println!("Executed actions list in test: {:?}", *actions);
    
    // We expect the first action to be execute at 0.500, 0.500.
    // The second action should have shifted coordinates (e.g. 0.485, 0.500).
    // Let's assert that at least one action is at the shifted coordinate if calibration works.
    assert!(!actions.is_empty(), "Actions should not be empty");
    let has_shifted = actions.iter().any(|act| act.contains("0.485"));
    assert!(has_shifted, "Should apply programmatic calibration shift in second attempt");
}

#[tokio::test]
async fn test_orchestrator_coordinate_normalization() {
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
        objectif: "Test Normalization".to_string(),
        nom_modele: "mock-model".to_string(),
        url_api: "http://localhost:8000/v1".to_string(),
        moteur: "LM Studio".to_string(),
        decomposer_taches: false,
        pipeline_vision_avance: false,
        activer_reflexion: false,
        activer_systeme_fast_slow: false,
        ..SavedConfig::default()
    };

    let config_repo = Arc::new(MockConfigRepo {
        config: std::sync::Mutex::new(config),
    });

    // Mock client that returns pixel-based coordinates: x = 0.180, y = 79.000
    // Target window height is 1589, so 79.0 / 1589.0 should be around 0.0497
    let llm_client = Arc::new(MockLlmClient {
        next_response: std::sync::Mutex::new(LlmResponse {
            status_display: "Mock Clicking Out-of-bounds".to_string(),
            action: "CLICK_AND_TYPE".to_string(),
            relative_click_position: vec![0.180, 79.000],
            ..LlmResponse::default()
        }),
        decompose_response: std::sync::Mutex::new(String::new()),
        call_count: std::sync::Mutex::new(0),
    });

    // Setup MockScreenCapturer
    let capturer = Arc::new(MockScreenCapturer {
        window_foreground_result: std::sync::Mutex::new(crate::screen_capture::WindowAnchorResult::AlreadyFocused),
    });

    // Setup MockPeripheralController
    let controller = Arc::new(MockPeripheralController {
        actions: std::sync::Mutex::new(vec![]),
    });

    let orchestrator = Arc::new(VibePilotOrchestrator::new(
        config_repo.clone(),
        llm_client.clone(),
        capturer.clone(),
        controller.clone(),
        Arc::new(MockWaitManager),
        bus.clone(),
    ));

    // Manually configure active_workspace_offsets to mimic Brave window size: [left=-609, top=1694, width=1872, height=1589]
    {
        let mut lock = orchestrator.active_workspace_offsets.lock().unwrap();
        lock.push(crate::peripheral_controller::ScreenOffset {
            titre: "Brave".to_string(),
            left: -609,
            top: 1694,
            width: 1872,
            height: 1589,
            y_offset: 1694,
        });
    }

    let orchestrator_clone = orchestrator.clone();
    let handle = tokio::spawn(async move {
        orchestrator_clone.run_loop().await
    });

    // Advance loop in virtual time so two iterations run
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    tokio::time::advance(std::time::Duration::from_secs(10)).await;
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // Stop orchestrator
    running.store(false, std::sync::atomic::Ordering::Relaxed);
    *orchestrator.state.lock().unwrap() = crate::orchestrator::OrchestratorState::Idle;
    loop {
        tokio::time::advance(std::time::Duration::from_millis(100)).await;
        tokio::task::yield_now().await;
        if handle.is_finished() {
            break;
        }
    }
    let _ = handle.await;

    let actions = controller.actions.lock().unwrap();
    println!("Executed actions list in test: {:?}", *actions);

    // Verify that the action was executed, and that its coordinates were NOT clamped or prematurely normalized in process_llm_decision.
    // Instead they should be passed down to execute_action as is.
    assert!(!actions.is_empty(), "Actions should not be empty");
    let has_raw = actions.iter().any(|act| act.contains("CLICK_AND_TYPE at 0.180, 79.000"));
    assert!(has_raw, "Should pass raw pixel coordinates down to execute_action");

    // Verify that the real compute_absolute_coordinates maps these coordinates correctly.
    let real_capturer = Arc::new(crate::screen_capture::MockScreenCapturer::new());
    let real_controller = crate::peripheral_controller::concrete::PeripheralController::new(real_capturer);
    let test_offsets = vec![crate::peripheral_controller::ScreenOffset {
        titre: "Brave".to_string(),
        left: -609,
        top: 1694,
        width: 1872,
        height: 1589,
        y_offset: 1694,
    }];
    let (abs_x, abs_y) = real_controller.compute_absolute_coordinates(0.180, 79.000, &test_offsets);
    assert_eq!(abs_x, -273);
    assert_eq!(abs_y, 1773);
}

#[tokio::test]
async fn test_orchestrator_roi_resolving() {
    let (bus, _rx) = EventBus::new();
    let config = SavedConfig {
        objectif: "Test ROI".to_string(),
        nom_modele: "mock-model".to_string(),
        url_api: "http://localhost:8000/v1".to_string(),
        moteur: "LM Studio".to_string(),
        activer_roi: true,
        roi_x: 100,
        roi_y: 200,
        roi_width: 300,
        roi_height: 400,
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
        controller,
        Arc::new(MockWaitManager),
        bus,
    );

    let active_config = orchestrator.config_repo.load_config();
    let offsets = orchestrator.get_active_offsets(&active_config, false);
    assert_eq!(offsets.len(), 1);
    let target = &offsets[0];
    assert_eq!(target.titre, "ROI");
    assert_eq!(target.left, 100);
    assert_eq!(target.top, 200);
    assert_eq!(target.width, 300);
    assert_eq!(target.height, 400);
}
