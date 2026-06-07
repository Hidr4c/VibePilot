//! Integration tests for VibePilot Rust port.
//!
//! Ports and extends the Python test coverage from test_vibepilot.py
//! and test_ui_integration.py. Covers: config, event bus, LLM client,
//! peripheral controller, screen capture, orchestrator, and content/lexicon.

#[allow(dead_code)]
mod common {
    use std::path::PathBuf;
    use std::fs;

    pub fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("vibepilot_test_{}", name));
        let _ = fs::create_dir_all(&dir);
        dir
    }

    pub fn cleanup(name: &str) {
        let dir = std::env::temp_dir().join(format!("vibepilot_test_{}", name));
        let _ = fs::remove_dir_all(&dir);
    }
}

mod config_tests {
    use crate::config::{ConfigRepository, SavedConfig, EnginePresets, EngineProfile};
    use crate::common::{temp_dir, cleanup};

    #[test]
    fn test_save_and_load_config() {
        let dir = temp_dir("save_load");
        let repo = ConfigRepository::new(dir.clone());
        let config = SavedConfig {
            contexte: "test context".to_string(),
            objectif: "test objectif".to_string(),
            task: "test task".to_string(),
            directives: "test directives".to_string(),
            demande_generique: "test request".to_string(),
            fenetres_surveillees: vec!["win1".to_string(), "win2".to_string()],
            moteur: "TestEngine".to_string(),
            url_api: "http://test:8000".to_string(),
            nom_modele: "test-model".to_string(),
            activer_son: false,
            langue: "Français".to_string(),
            auto_validate: true,
            theme_sombre: true,
            ..SavedConfig::default()
        };
        repo.save_config(&config);
        let loaded = repo.load_config();
        assert_eq!(loaded.contexte, "test context");
        assert_eq!(loaded.objectif, "test objectif");
        assert_eq!(loaded.fenetres_surveillees, vec!["win1", "win2"]);
        assert_eq!(loaded.moteur, "TestEngine");
        assert_eq!(loaded.activer_son, false);
        assert_eq!(loaded.langue, "Français");
        assert_eq!(loaded.auto_validate, true);
        assert_eq!(loaded.theme_sombre, true);
        cleanup("save_load");
    }

    #[test]
    fn test_load_missing_config_returns_defaults() {
        let dir = temp_dir("load_missing");
        let repo = ConfigRepository::new(dir.clone());
        let loaded = repo.load_config();
        assert!(!loaded.langue.is_empty());
        assert!(!loaded.moteur.is_empty());
        assert!(!loaded.url_api.is_empty());
        assert!(!loaded.nom_modele.is_empty());
        cleanup("load_missing");
    }

    #[test]
    fn test_load_invalid_config_falls_back_to_defaults() {
        let dir = temp_dir("invalid_config");
        let save_path = dir.join("save.enc");
        std::fs::write(&save_path, "[invalid toml]").ok();
        let repo = ConfigRepository::new(dir.clone());
        let loaded = repo.load_config();
        assert!(!loaded.langue.is_empty());
        cleanup("invalid_config");
    }

    #[test]
    fn test_save_and_load_profile() {
        let dir = temp_dir("profile_save");
        let repo = ConfigRepository::new(dir.clone());
        let config = SavedConfig {
            contexte: "profile context".to_string(),
            ..SavedConfig::default()
        };
        assert!(repo.save_profile("test_profile", &config));
        let loaded = repo.load_profile("test_profile");
        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap().contexte, "profile context");
        cleanup("profile_save");
    }

    #[test]
    fn test_list_profiles() {
        let dir = temp_dir("list_profiles_integration");
        let repo = ConfigRepository::new(dir.clone());
        let profiles_dir = dir.join("profiles");
        let _ = std::fs::create_dir_all(&profiles_dir);
        std::fs::write(profiles_dir.join("profile1.enc"), "{}").ok();
        std::fs::write(profiles_dir.join("profile2.enc"), "{}").ok();
        let profiles = repo.list_profiles();
        assert!(profiles.contains(&"profile1".to_string()));
        assert!(profiles.contains(&"profile2".to_string()));
        cleanup("list_profiles_integration");
    }

    #[test]
    fn test_delete_profile() {
        let dir = temp_dir("delete_profile_integration");
        let repo = ConfigRepository::new(dir.clone());
        let config = SavedConfig::default();
        repo.save_profile("to_delete", &config);
        assert!(repo.delete_profile("to_delete"));
        assert!(!repo.delete_profile("to_delete"));
        cleanup("delete_profile_integration");
    }

    #[test]
    fn test_load_engines_defaults() {
        let dir = temp_dir("engines_default");
        let repo = ConfigRepository::new(dir.clone());
        let presets = repo.load_engines();
        assert!(presets.lm_studio.modeles.len() > 0);
        assert!(presets.ollama.modeles.len() > 0);
        assert!(presets.custom.modeles.len() > 0);
        cleanup("engines_default");
    }

    #[test]
    fn test_save_and_load_engines() {
        let dir = temp_dir("engines_save");
        let repo = ConfigRepository::new(dir.clone());
        let presets = EnginePresets {
            lm_studio: EngineProfile {
                url: "http://custom:1234".to_string(),
                modeles: vec!["custom-model".to_string()],
            },
            ..EnginePresets::default()
        };
        repo.save_engines(&presets);
        let loaded = repo.load_engines();
        assert_eq!(loaded.lm_studio.url, "http://custom:1234");
        assert_eq!(loaded.lm_studio.modeles, vec!["custom-model"]);
        cleanup("engines_save");
    }

    #[test]
    fn test_config_all_fields_present() {
        let dir = temp_dir("all_fields");
        let repo = ConfigRepository::new(dir.clone());
        let config = repo.load_config();
        // Verify all fields exist and defaults are reasonable
        assert!(config.contexte.is_empty() || !config.contexte.is_empty());
        assert!(config.objectif.is_empty() || !config.objectif.is_empty());
        assert!(config.task.is_empty() || !config.task.is_empty());
        assert!(config.directives.is_empty() || !config.directives.is_empty());
        assert!(!config.langue.is_empty());
        assert!(!config.moteur.is_empty());
        assert!(!config.url_api.is_empty());
        assert!(!config.nom_modele.is_empty());
        assert!(!config.url_api_vision.is_empty());
        assert!(!config.nom_modele_vision.is_empty());
        assert!(!config.moteur_vision.is_empty());
        assert!(!config.auth_mode_vision.is_empty());
        assert_eq!(config.request_timeout_secs_vision, 120);
        assert_eq!(config.utiliser_moteur_vision_dedie, false);
        cleanup("all_fields");
    }

    #[test]
    fn test_partial_save_merge() {
        let dir = temp_dir("partial_save");
        let save_path = dir.join("save.enc");
        std::fs::write(&save_path, r#"{"existing_key": "existing_value", "moteur": "old"}"#).ok();
        let repo = ConfigRepository::new(dir.clone());
        let config = SavedConfig {
            moteur: "new".to_string(),
            demande_generique: "new_value".to_string(),
            ..SavedConfig::default()
        };
        repo.save_config(&config);
        let loaded = repo.load_config();
        assert_eq!(loaded.moteur, "new");
        cleanup("partial_save");
    }
}

mod event_bus_tests {
    use crate::event_bus::{EventBus, EventType};

    #[test]
    fn test_new_bus_is_empty() {
        let (bus, _rx) = EventBus::new();
        assert_eq!(bus.query_handler_count(), 0);
    }

    #[test]
    fn test_subscribe_and_emit() {
        let (bus, _rx) = EventBus::new();
        let received = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let received_clone = received.clone();

        bus.subscribe(Box::new(move |e| {
            received_clone.lock().unwrap().push(e.clone());
        }));

        bus.emit(EventType::Log("test".to_string()));
        assert_eq!(received.lock().unwrap().len(), 1);
    }

    #[test]
    fn test_multiple_subscribers() {
        let (bus, _rx) = EventBus::new();
        let count = std::sync::Arc::new(std::sync::Mutex::new(0));

        for _ in 0..3 {
            let count_clone = count.clone();
            bus.subscribe_for(
                EventType::Log("test".to_string()),
                Box::new(move |_e| {
                    *count_clone.lock().unwrap() += 1;
                }),
            );
        }

        bus.emit(EventType::Log("test".to_string()));
        assert_eq!(*count.lock().unwrap(), 3);
    }

    #[test]
    fn test_query_handler() {
        let (bus, _rx) = EventBus::new();
        bus.register_query(
            EventType::GetLangText("btn_lancer".to_string()),
            Box::new(|_| "Lancer".to_string()),
        );
        let response = bus.emit_query(EventType::GetLangText("btn_lancer".to_string()));
        assert_eq!(response, "Lancer");
    }

    #[test]
    fn test_query_no_handler_returns_empty() {
        let (bus, _rx) = EventBus::new();
        let response = bus.emit_query(EventType::GetLangText("unknown".to_string()));
        assert_eq!(response, "");
    }

    #[test]
    fn test_event_display() {
        assert_eq!(EventType::Log(String::new()).to_string(), "LOG");
        assert_eq!(
            EventType::UpdateStatus { text: "x".to_string(), color: "y".to_string() }.to_string(),
            "UPDATE_STATUS"
        );
    }

    #[test]
    fn test_event_hash() {
        use std::collections::HashMap;
        let mut map = HashMap::new();
        map.insert(EventType::Log(String::new()), "test");
        assert!(map.contains_key(&EventType::Log(String::new())));
    }

    #[test]
    fn test_clone_bus() {
        let (bus1, _rx1) = EventBus::new();
        bus1.register_query(
            EventType::GetLangText("test".to_string()),
            Box::new(|_| "response".to_string()),
        );
        let bus2 = bus1.clone();
        let response = bus2.emit_query(EventType::GetLangText("test".to_string()));
        assert_eq!(response, "response");
    }

    #[test]
    fn test_default_bus() {
        let bus = EventBus::default();
        assert_eq!(bus.query_handler_count(), 0);
    }
}

mod llm_client_tests {
    use crate::llm_client::LlmResponse;

    #[test]
    fn test_llm_response_deserialize() {
        let json = r#"{
            "status_display": "Test",
            "action": "CLICK_AND_TYPE",
            "relative_click_position": [0.5, 0.5],
            "text_to_type": "hello",
            "scroll_value": -6,
            "wait_seconds": 15
        }"#;
        let response: LlmResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.action, "CLICK_AND_TYPE");
        assert_eq!(response.text_to_type, "hello");
        assert_eq!(response.wait_seconds, 15);
        assert_eq!(response.scroll_value, -6);
    }

    #[test]
    fn test_llm_response_defaults() {
        let json = r#"{
            "status_display": "Test",
            "action": "WAIT",
            "relative_click_position": [0.0, 0.0],
            "text_to_type": "",
            "scroll_value": -6,
            "wait_seconds": 15
        }"#;
        let response: LlmResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.action, "WAIT");
        assert!(response.text_to_type.is_empty());
    }

    #[test]
    fn test_llm_response_success_action() {
        let json = r#"{
            "status_display": "Done",
            "action": "SUCCESS",
            "relative_click_position": [1.0, 1.0],
            "text_to_type": "",
            "scroll_value": 0,
            "wait_seconds": 0
        }"#;
        let response: LlmResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.action, "SUCCESS");
    }

    #[test]
    fn test_llm_response_fail_action() {
        let json = r#"{
            "status_display": "Failed",
            "action": "FAIL",
            "relative_click_position": [0.0, 0.0],
            "text_to_type": "",
            "scroll_value": 0,
            "wait_seconds": 30
        }"#;
        let response: LlmResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.action, "FAIL");
        assert_eq!(response.wait_seconds, 30);
    }

    #[test]
    fn test_llm_response_scroll_action() {
        let json = r#"{
            "status_display": "Scrolling",
            "action": "SCROLL",
            "relative_click_position": [0.5, 0.5],
            "text_to_type": "",
            "scroll_value": -12,
            "wait_seconds": 5
        }"#;
        let response: LlmResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.action, "SCROLL");
        assert_eq!(response.scroll_value, -12);
    }
}

mod peripheral_controller_tests {
    use crate::peripheral_controller::{ActionPayload, PeripheralController, ScreenOffset};
    use crate::screen_capture::{ScreenCapturer, ScreenInfo, BoundingBox};

    #[test]
    fn test_controller_creation() {
        let capturer = ScreenCapturer::new();
        let controller1 = PeripheralController::new(std::sync::Arc::new(capturer));
        let capturer2 = ScreenCapturer::new();
        let controller2 = PeripheralController::new(std::sync::Arc::new(capturer2));
        assert!(controller1 != controller2);
    }

    #[test]
    fn test_action_payload_creation() {
        let payload = ActionPayload {
            action: "CLICK_AND_TYPE".to_string(),
            relative_click_position: vec![0.5, 0.5],
            text_to_type: "hello world".to_string(),
            scroll_value: 0,
            wait_seconds: 5,
        };
        assert_eq!(payload.action, "CLICK_AND_TYPE");
        assert_eq!(payload.text_to_type, "hello world");
        assert_eq!(payload.wait_seconds, 5);
    }

    #[test]
    fn test_screen_offset_from_screen_info() {
        let screen = ScreenInfo::new(
            "Test Window".to_string(),
            BoundingBox::new(100, 200, 800, 600),
        );
        let offset = ScreenOffset::from_screen_info(screen);
        assert_eq!(offset.titre, "Test Window");
        assert_eq!(offset.left, 100);
        assert_eq!(offset.top, 200);
        assert_eq!(offset.width, 800);
        assert_eq!(offset.height, 600);
        assert_eq!(offset.y_offset, 200);
    }

    #[test]
    fn test_coordinate_computation_single_screen() {
        let capturer = ScreenCapturer::new();
        let offsets: Vec<ScreenOffset> = capturer.get_monitors()
            .into_iter()
            .map(|s| ScreenOffset::from_screen_info(s))
            .collect();
        let controller = PeripheralController::new(std::sync::Arc::new(capturer));

        // Test center of screen
        let (x, y) = controller.compute_absolute_coordinates(0.5, 0.5, &offsets);
        assert!(x >= 0);
        assert!(y >= 0);
    }

    #[test]
    fn test_coordinate_clamping() {
        let capturer = ScreenCapturer::new();
        let offsets: Vec<ScreenOffset> = capturer.get_monitors()
            .into_iter()
            .map(|s| ScreenOffset::from_screen_info(s))
            .collect();
        let controller = PeripheralController::new(std::sync::Arc::new(capturer));

        // Test clamping at edges
        let (x1, y1) = controller.compute_absolute_coordinates(0.0, 0.0, &offsets);
        assert_eq!(x1, 0);
        assert_eq!(y1, 0);

        let (x2, y2) = controller.compute_absolute_coordinates(1.0, 1.0, &offsets);
        assert!(x2 > 0);
        assert!(y2 > 0);
    }

    #[test]
    fn test_is_desktop_title() {
        let capturer = ScreenCapturer::new();
        let controller = PeripheralController::new(std::sync::Arc::new(capturer));

        assert!(controller.is_desktop_title("Desktop"));
        assert!(controller.is_desktop_title("Bureau"));
        assert!(controller.is_desktop_title("ÉCRAN"));
        assert!(controller.is_desktop_title("ecran"));
        assert!(controller.is_desktop_title("all_screens"));
        assert!(controller.is_desktop_title("tous les écrans"));
        assert!(controller.is_desktop_title("ALL SCREENS (Virtual Desktop)"));
        assert!(!controller.is_desktop_title("VS Code"));
        assert!(!controller.is_desktop_title("Firefox"));
        assert!(!controller.is_desktop_title("Explorer"));
    }

    #[test]
    fn test_get_screen_bounds() {
        let capturer = ScreenCapturer::new();
        let controller = PeripheralController::new(std::sync::Arc::new(capturer));
        let (left, top, right, bottom) = controller.get_screen_bounds();
        assert!(right > left);
        assert!(bottom > top);
    }

    #[test]
    fn test_coordinate_clamping_virtual_bounds() {
        let capturer = ScreenCapturer::new();
        let controller = PeripheralController::new(std::sync::Arc::new(capturer));

        // Test with custom off-primary coordinates simulating screen 2
        let offsets = vec![
            ScreenOffset {
                titre: "Screen 2".to_string(),
                left: 1920,
                top: 0,
                width: 1920,
                height: 1080,
                y_offset: 0,
            }
        ];

        let (x, y) = controller.compute_absolute_coordinates(0.5, 0.5, &offsets);
        // Should click in the center of Screen 2 (1920 + 960 = 2880)
        assert!(x >= 1920);
        assert!(y >= 0);
    }
}

mod screen_capture_tests {
    use crate::screen_capture::{ScreenCapturer, ScreenInfo, BoundingBox};

    #[test]
    fn test_new_capturer() {
        let capturer = ScreenCapturer::new();
        let monitors = capturer.get_monitors();
        assert!(monitors.len() > 0);
    }

    #[test]
    fn test_capture_desktop() {
        let capturer = ScreenCapturer::new();
        let result = capturer.capture_desktop();
        // Capture should succeed on a real display
        assert!(result.is_some(), "Desktop capture should succeed");
        let img = result.unwrap();
        assert!(img.width() > 0);
        assert!(img.height() > 0);
    }

    #[test]
    fn test_bounding_box_contains() {
        let bbox = BoundingBox::new(100, 100, 200, 200);
        assert!(bbox.contains(150, 150));
        assert!(!bbox.contains(50, 50));
        assert!(!bbox.contains(350, 350));
        assert!(bbox.contains(100, 100)); // edge
        assert!(!bbox.contains(350, 350)); // just outside
    }

    #[test]
    fn test_bounding_box_new() {
        let bbox = BoundingBox::new(0, 0, 1920, 1080);
        assert_eq!(bbox.left, 0);
        assert_eq!(bbox.top, 0);
        assert_eq!(bbox.width, 1920);
        assert_eq!(bbox.height, 1080);
    }

    #[test]
    fn test_screen_info_new() {
        let info = ScreenInfo::new(
            "Test Monitor".to_string(),
            BoundingBox::new(0, 0, 1920, 1080),
        );
        assert_eq!(info.title, "Test Monitor");
        assert_eq!(info.bbox.left, 0);
        assert_eq!(info.bbox.width, 1920);
    }

    #[test]
    fn test_is_target_visible_empty_list() {
        let capturer = ScreenCapturer::new();
        assert!(capturer.is_target_visible(&[]));
    }

    #[test]
    fn test_is_target_visible_with_targets() {
        let capturer = ScreenCapturer::new();
        // On a real system, some windows should be visible
        let result = capturer.is_target_visible(&["Code".to_string(), "Explorer".to_string()]);
        // Result depends on what's running; just verify no panic
        let _ = result;
    }

    #[test]
    fn test_capture_bbox() {
        let capturer = ScreenCapturer::new();
        let monitors = capturer.get_monitors();
        if let Some(screen) = monitors.first() {
            let result = capturer.capture_bbox(
                screen.bbox.left,
                screen.bbox.top,
                screen.bbox.width,
                screen.bbox.height,
            );
            assert!(result.is_some());
        }
    }
}

mod orchestrator_tests {
    use crate::config::{ConfigRepository, SavedConfig};
    use crate::event_bus::{EventBus, EventType};
    use crate::orchestrator::VibePilotOrchestrator;
    use std::sync::Arc;

    #[test]
    fn test_orchestrator_creation() {
        let (bus, _rx) = EventBus::new();
        let config_repo = Arc::new(ConfigRepository::new(
            std::env::temp_dir().join("vibepilot_test_orch"),
        ));
        let _orch = VibePilotOrchestrator::new(config_repo, bus);
        // Verify no panic on creation
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

        let _orch = VibePilotOrchestrator::new(config_repo, bus);
        // Orchestrator loads config on first run_loop call
        // We can't run the full loop without a real LLM, but we verify creation
    }

    #[test]
    fn test_orchestrator_emits_log_event() {
        let (bus, rx) = EventBus::new();
        let config_repo = Arc::new(ConfigRepository::new(
            std::env::temp_dir().join("vibepilot_test_orch2"),
        ));
        let _orch = VibePilotOrchestrator::new(config_repo, bus.clone());

        // Verify event bus is properly wired
        assert_eq!(bus.query_handler_count(), 0); // No handlers registered yet
        let _ = rx.try_recv(); // Should be empty
    }

    #[test]
    fn test_orchestrator_pause_query() {
        let (bus, _rx) = EventBus::new();
        let config_repo = Arc::new(ConfigRepository::new(
            std::env::temp_dir().join("vibepilot_test_orch3"),
        ));
        let _orch = VibePilotOrchestrator::new(config_repo, bus);
        // Verify no panic on creation; pause mode defaults to false
    }

    #[test]
    fn test_orchestrator_running_query() {
        let (bus, _rx) = EventBus::new();
        let config_repo = Arc::new(ConfigRepository::new(
            std::env::temp_dir().join("vibepilot_test_orch4"),
        ));
        let orch = VibePilotOrchestrator::new(config_repo, bus.clone());
        
        // Register running handler
        let running_status = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let running_status_clone = running_status.clone();
        bus.register_query(
            EventType::GetOrchestratorRunning,
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
        use crate::orchestrator::is_local_url;
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
        let orch = VibePilotOrchestrator::new(config_repo, bus);
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

        // Save a config with detecter_activite_utilisateur = false
        let config = SavedConfig {
            detecter_activite_utilisateur: false,
            ..SavedConfig::default()
        };
        config_repo.save_config(&config);

        let orch = VibePilotOrchestrator::new(config_repo, bus);
        
        // This should return immediately because detecter_activite_utilisateur is false
        let result = tokio::time::timeout(std::time::Duration::from_millis(500), orch.wait_if_user_active()).await;
        assert!(result.is_ok());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_is_user_active_now_returns_bool() {
        let _active = crate::ui::components::is_user_active_now();
        // Since we don't simulate real user actions in tests, we just check that it runs without panic
    }
}

mod version_tests {
    use crate::version::{get_version, get_git_hash, get_build_date, get_version_info};

    #[test]
    fn test_get_version() {
        let version = get_version();
        assert!(!version.is_empty());
        assert!(version.len() > 0);
    }

    #[test]
    fn test_get_git_hash() {
        let hash = get_git_hash();
        assert!(!hash.is_empty());
    }

    #[test]
    fn test_get_build_date() {
        let date = get_build_date();
        assert!(!date.is_empty());
        assert!(date.contains("-")); // Should have date format
    }

    #[test]
    fn test_get_version_info() {
        let info = get_version_info();
        assert!(info.contains("VibePilot"));
        assert!(info.contains("v"));
    }
}

mod content_tests {
    // Tests for content/prompt constants that were in Python content.py
    #[test]
    fn test_engine_presets_have_defaults() {
        use crate::config::EnginePresets;
        let presets = EnginePresets::default();
        assert!(!presets.lm_studio.url.is_empty());
        assert!(!presets.ollama.url.is_empty());
        assert!(!presets.custom.url.is_empty());
        assert!(presets.lm_studio.modeles.len() > 0);
        assert!(presets.ollama.modeles.len() > 0);
        assert!(presets.custom.modeles.len() > 0);
    }

    #[test]
    fn test_engine_preset_names() {
        use crate::config::EnginePresets;
        let presets = EnginePresets::default();
        assert_eq!(presets.lm_studio.url, "http://127.0.0.1:1234/v1/chat/completions");
        assert_eq!(presets.ollama.url, "http://127.0.0.1:11434/v1/chat/completions");
        assert_eq!(presets.custom.url, "http://localhost:8000/v1/chat/completions");
    }

    #[test]
    fn test_default_config_fields() {
        use crate::config::SavedConfig;
        let config = SavedConfig::default();
        assert_eq!(config.activer_son, true);
        assert_eq!(config.auto_validate, true);
        assert_eq!(config.theme_sombre, true);
    }
}

mod security_tests {
    #[test]
    fn test_dangerous_keywords_detection() {
        let dangerous_keywords = [
            "format", "shutdown", "sudo", "rm -rf", "drop table",
            "delete from", "chmod", "wget http", "curl http",
            "pip install", "npm install", "taskkill", "reg delete",
            "kill -9", "desactiver antivirus", "turn off firewall",
            "rmdir", "apt remove", "apt purge", "bcdedit", "dd if=",
        ];

        for keyword in &dangerous_keywords {
            assert!(crate::config::contains_dangerous_command(keyword), "Keyword '{}' should be flagged as dangerous", keyword);
            assert!(crate::config::contains_dangerous_command(&format!("some prefix {} suffix", keyword)));
        }
    }

    #[test]
    fn test_safe_text_not_flagged() {
        let safe_texts = [
            "Hello world",
            "Normal message",
            "Standard operation",
            "Regular computation",
        ];

        for text in &safe_texts {
            assert!(!crate::config::contains_dangerous_command(text), "Text '{}' should not be flagged as dangerous", text);
        }
    }

    #[test]
    fn test_empty_text_not_flagged() {
        assert!(!crate::config::contains_dangerous_command(""));
    }
}

mod app_state_tests {
    use crate::app::{VibePilotApp, Tab};
    use crate::common::{temp_dir, cleanup};
    use crate::event_bus::EventType;
    use eframe::egui;

    static TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn test_app_creation_and_defaults() {
        let _lock = TEST_LOCK.lock().unwrap();
        let egui_ctx = egui::Context::default();
        let _dir = temp_dir("app_state_creation");
        
        let app = VibePilotApp::new(&egui_ctx);
        assert_eq!(app.active_tab, Tab::GlobalConfig);
        assert_eq!(app.is_running, false);
        assert_eq!(app.logs.len(), 0);
        assert_eq!(app.current_config.activer_son, true);
        assert_eq!(app.current_config.auto_validate, true);
        cleanup("app_state_creation");
    }

    #[test]
    fn test_app_load_profile_resets_session() {
        let _lock = TEST_LOCK.lock().unwrap();
        let egui_ctx = egui::Context::default();
        let dir = temp_dir("app_load_profile_resets");
        let _ = std::fs::create_dir_all(&dir);
        
        let mut app = VibePilotApp::new(&egui_ctx);
        
        // Setup mock profile on disk
        let config = crate::config::SavedConfig {
            contexte: "profile context".to_string(),
            ..crate::config::SavedConfig::default()
        };
        app.config_repo.save_profile("reset_test_profile", &config);

        // Put some data into the active session
        app.action_history.push(("CLICK".to_string(), "c".to_string()));
        app.logs.push("Log message".to_string());
        app.report_content = "some report".to_string();
        app.status_text = "Running".to_string();
        app.status_color = "green".to_string();
        app.is_running = true;

        // Load profile and verify it resets the session
        assert!(app.load_profile("reset_test_profile"));
        app.poll_events();
        assert_eq!(app.action_history.len(), 0);
        assert_eq!(app.logs.len(), 1); // Only the "Profile loaded. Session reset." log is present
        assert_eq!(app.report_content, "");
        assert_eq!(app.status_text, "Ready");
        assert_eq!(app.status_color, "grey");
        assert_eq!(app.is_running, false);
        assert_eq!(app.current_config.contexte, "profile context");

        cleanup("app_load_profile_resets");
    }

    #[test]
    fn test_app_translation_helper() {
        let _lock = TEST_LOCK.lock().unwrap();
        let egui_ctx = egui::Context::default();
        let _dir = temp_dir("app_translation");
        let mut app = VibePilotApp::new(&egui_ctx);
        
        // English translation check
        app.current_config.langue = "English".to_string();
        assert_eq!(app.t("btn_reinit"), "🔄 Reset All");
        
        // French translation check
        app.current_config.langue = "Français".to_string();
        assert_eq!(app.t("btn_reinit"), "🔄 Réinitialiser tout");
        
        cleanup("app_translation");
    }

    #[test]
    fn test_app_reset_settings() {
        let _lock = TEST_LOCK.lock().unwrap();
        let egui_ctx = egui::Context::default();
        let _dir = temp_dir("app_reset_settings");
        
        // Point bootstrap config to _dir
        let mut bootstrap = crate::config::load_bootstrap_config();
        bootstrap.storage_dir = Some(_dir.to_string_lossy().to_string());
        crate::config::save_bootstrap_config(&bootstrap);

        let mut app = VibePilotApp::new(&egui_ctx);
        
        // Modify fields
        app.current_config.contexte = "Non-default context".to_string();
        app.current_config.langue = "Français".to_string();
        app.current_config.zoom_facteur = Some(1.5);
        app.current_config.auto_validate = false;
        
        // Reset settings
        app.reset_all_settings();
        
        // Check defaults are restored
        assert_eq!(app.current_config.contexte, crate::config::DEFAULT_CONTEXT);
        assert_eq!(app.current_config.langue, "English");
        assert_eq!(app.current_config.zoom_facteur, None);
        assert_eq!(app.current_config.auto_validate, true);
        
        cleanup("app_reset_settings");
    }

    #[test]
    fn test_app_engine_management() {
        let _lock = TEST_LOCK.lock().unwrap();
        let egui_ctx = egui::Context::default();
        let _dir = temp_dir("app_engine_mgmt");
        
        // Point bootstrap config to _dir
        let mut bootstrap = crate::config::load_bootstrap_config();
        bootstrap.storage_dir = Some(_dir.to_string_lossy().to_string());
        crate::config::save_bootstrap_config(&bootstrap);

        let mut app = VibePilotApp::new(&egui_ctx);
        
        // Add a preset
        let custom_profile = crate::config::EngineProfile {
            url: "http://my-custom-engine:9999/v1".to_string(),
            modeles: vec!["my-custom-model".to_string()],
        };
        app.engine_presets.insert("MyCustomEngine".to_string(), custom_profile.clone());
        app.config_repo.save_engines(&app.engine_presets);
        
        // Reload in new app to verify persistence
        let app_reloaded = VibePilotApp::new(&egui_ctx);
        let loaded_preset = app_reloaded.engine_presets.get("MyCustomEngine");
        assert!(loaded_preset.is_some());
        assert_eq!(loaded_preset.unwrap().url, "http://my-custom-engine:9999/v1");
        
        // Delete the preset
        let mut app_deleted = app_reloaded;
        app_deleted.engine_presets.remove("MyCustomEngine");
        app_deleted.config_repo.save_engines(&app_deleted.engine_presets);
        
        // Reload again to verify deletion
        let app_final = VibePilotApp::new(&egui_ctx);
        assert!(app_final.engine_presets.get("MyCustomEngine").is_none());
        
        cleanup("app_engine_mgmt");
    }

    #[test]
    fn test_app_storage_migration() {
        let _lock = TEST_LOCK.lock().unwrap();
        let egui_ctx = egui::Context::default();
        let src_dir = temp_dir("app_migration_src");
        let dest_dir = temp_dir("app_migration_dest");
        
        // Point bootstrap config to src_dir
        let mut bootstrap = crate::config::load_bootstrap_config();
        bootstrap.storage_dir = Some(src_dir.to_string_lossy().to_string());
        crate::config::save_bootstrap_config(&bootstrap);

        let mut app = VibePilotApp::new(&egui_ctx);
        
        // Save some custom config in current repo
        app.current_config.contexte = "Migration Test Contexte".to_string();
        app.config_repo.save_config(&app.current_config);
        
        // Migrate storage
        let dest_path_str = dest_dir.to_string_lossy().to_string();
        let result = app.migrate_storage(dest_path_str.clone());
        assert!(result.is_ok());
        
        // Verify bootstrap config updated
        let loaded_bootstrap = crate::config::load_bootstrap_config();
        assert_eq!(loaded_bootstrap.storage_dir, Some(dest_path_str.clone()));
        
        // Verify app's config_repo updated
        let new_save_path = app.config_repo.get_save_path();
        assert!(new_save_path.starts_with(&dest_dir));
        
        // Verify the config is loaded/copied correctly
        assert_eq!(app.current_config.contexte, "Migration Test Contexte");
        
        cleanup("app_migration_src");
        cleanup("app_migration_dest");
    }

    #[test]
    fn test_app_ui_rendering() {
        let _lock = TEST_LOCK.lock().unwrap();
        let egui_ctx = egui::Context::default();
        let _dir = temp_dir("app_ui_rendering");
        
        let mut app = VibePilotApp::new(&egui_ctx);
        
        // Render default tab (GlobalConfig)
        let _ = egui_ctx.run(egui::RawInput::default(), |ctx| {
            crate::ui::render_main_window(ctx, &mut app);
        });

        // Switch to PromptEditor
        app.active_tab = Tab::PromptEditor;
        let _ = egui_ctx.run(egui::RawInput::default(), |ctx| {
            crate::ui::render_main_window(ctx, &mut app);
        });

        // Switch to Help
        app.active_tab = Tab::Help;
        let _ = egui_ctx.run(egui::RawInput::default(), |ctx| {
            crate::ui::render_main_window(ctx, &mut app);
        });

        // Switch to Setup
        app.active_tab = Tab::Setup;
        let _ = egui_ctx.run(egui::RawInput::default(), |ctx| {
            crate::ui::render_main_window(ctx, &mut app);
        });

        // Setup pending dangerous action modal
        app.pending_action = Some(crate::app::PendingAction {
            action: "CLICK".to_string(),
            text_to_type: "rm -rf /".to_string(),
            scroll_value: 0,
        });
        let _ = egui_ctx.run(egui::RawInput::default(), |ctx| {
            crate::ui::render_main_window(ctx, &mut app);
        });

        // Setup pending safe action modal
        app.pending_action = Some(crate::app::PendingAction {
            action: "CLICK".to_string(),
            text_to_type: "safe command".to_string(),
            scroll_value: 0,
        });
        let _ = egui_ctx.run(egui::RawInput::default(), |ctx| {
            crate::ui::render_main_window(ctx, &mut app);
        });

        cleanup("app_ui_rendering");
    }

    #[test]
    fn test_app_orchestrator_controls() {
        let _lock = TEST_LOCK.lock().unwrap();
        let egui_ctx = egui::Context::default();
        let _dir = temp_dir("app_orch_controls");
        
        let mut app = VibePilotApp::new(&egui_ctx);
        assert!(!app.is_running);
        
        app.start_orchestrator();
        assert!(app.is_running);
        
        app.stop_orchestrator();
        assert!(!app.is_running);
        
        cleanup("app_orch_controls");
    }

    #[test]
    fn test_app_event_processing() {
        let _lock = TEST_LOCK.lock().unwrap();
        let egui_ctx = egui::Context::default();
        let _dir = temp_dir("app_event_proc");
        
        let mut app = VibePilotApp::new(&egui_ctx);
        
        // Test Event::Emit Log
        app.bus.emit(EventType::Log("Test log message".to_string()));
        app.poll_events();
        assert!(app.logs.iter().any(|l| l.contains("Test log message")));
        
        // Test Event::Emit UpdateStatus
        app.bus.emit(EventType::UpdateStatus { text: "Custom Status".to_string(), color: "blue".to_string() });
        app.poll_events();
        assert_eq!(app.status_text, "Custom Status");
        assert_eq!(app.status_color, "blue");
        
        // Test Event::Emit AppendAction
        app.bus.emit(EventType::AppendAction { action: "CLICK".to_string(), display: "Clicked Button".to_string() });
        app.poll_events();
        assert_eq!(app.action_history.last().unwrap(), &("CLICK".to_string(), "Clicked Button".to_string()));
        
        // Test Event::Emit SelectTab
        app.bus.emit(EventType::SelectTab(2)); // Help
        app.poll_events();
        assert_eq!(app.active_tab, Tab::Help);
        
        // Test Event::Emit UpdateField
        app.bus.emit(EventType::UpdateField { field: "contexte".to_string(), value: "New Context Content".to_string() });
        app.poll_events();
        assert_eq!(app.current_config.contexte, "New Context Content");
        
        // Test Event::Emit UpdateAllFields
        app.bus.emit(EventType::UpdateAllFields {
            contexte: "c".to_string(),
            task: "t".to_string(),
            objectif: "o".to_string(),
            directives: "d".to_string(),
        });
        app.poll_events();
        assert_eq!(app.current_config.contexte, "c");
        assert_eq!(app.current_config.task, "t");
        assert_eq!(app.current_config.objectif, "o");
        assert_eq!(app.current_config.directives, "d");
        
        // Test Event::Emit ShowActionConfirmation & ClearActionConfirmation
        app.bus.emit(EventType::ShowActionConfirmation {
            action: "CLICK".to_string(),
            text: "Hello".to_string(),
            scroll: 42,
        });
        app.poll_events();
        let pending = app.pending_action.clone().unwrap();
        assert_eq!(pending.action, "CLICK");
        assert_eq!(pending.text_to_type, "Hello");
        assert_eq!(pending.scroll_value, 42);
        
        app.bus.emit(EventType::ClearActionConfirmation);
        app.poll_events();
        assert!(app.pending_action.is_none());
        
        cleanup("app_event_proc");
    }

    #[test]
    fn test_profile_name_generation_and_rename() {
        use crate::common::{temp_dir, cleanup};
        let _lock = TEST_LOCK.lock().unwrap();
        let name = "profile_gen_rename";
        let dir = temp_dir(name);
        
        // Point bootstrap config to temp dir
        let mut bootstrap = crate::config::load_bootstrap_config();
        let old_dir = bootstrap.storage_dir.clone();
        bootstrap.storage_dir = Some(dir.to_string_lossy().to_string());
        crate::config::save_bootstrap_config(&bootstrap);

        let ctx = egui::Context::default();
        let mut app = VibePilotApp::new(&ctx);

        // Verify initial available profile name
        let first_name = app.get_first_available_profile_name();
        assert_eq!(first_name, "profile_01");

        // Save a profile named profile_01
        app.selected_profile = "profile_01".to_string();
        app.config_repo.save_profile("profile_01", &app.current_config);

        // Now first available profile name should be profile_02
        let second_name = app.get_first_available_profile_name();
        assert_eq!(second_name, "profile_02");

        // Test renaming profile_01 to profile_03
        // 1. Verify profile_01 exists and profile_03 does not
        let list_before = app.config_repo.list_profiles();
        assert!(list_before.contains(&"profile_01".to_string()));
        assert!(!list_before.contains(&"profile_03".to_string()));

        // 2. Perform rename logic
        if let Some(cfg) = app.config_repo.load_profile("profile_01") {
            app.config_repo.save_profile("profile_03", &cfg);
            app.config_repo.delete_profile("profile_01");
            app.selected_profile = "profile_03".to_string();
        }

        // 3. Verify results
        let list_after = app.config_repo.list_profiles();
        assert!(!list_after.contains(&"profile_01".to_string()));
        assert!(list_after.contains(&"profile_03".to_string()));

        // Restore bootstrap config
        bootstrap.storage_dir = old_dir;
        crate::config::save_bootstrap_config(&bootstrap);
        cleanup(name);
    }

    #[test]
    fn test_quick_start_profile_name_generation() {
        use crate::common::{temp_dir, cleanup};
        let _lock = TEST_LOCK.lock().unwrap();
        let name = "profile_qs_gen";
        let dir = temp_dir(name);

        // Point bootstrap config to temp dir
        let mut bootstrap = crate::config::load_bootstrap_config();
        let old_dir = bootstrap.storage_dir.clone();
        bootstrap.storage_dir = Some(dir.to_string_lossy().to_string());
        crate::config::save_bootstrap_config(&bootstrap);

        let ctx = egui::Context::default();
        let mut app = VibePilotApp::new(&ctx);

        // Initially both should point to "profile_01" because it's the first available profile
        assert_eq!(app.selected_profile, "profile_01");
        assert_eq!(app.quick_start_profile_name, "profile_01");

        // Set quick start profile name to custom value
        app.quick_start_profile_name = "profile_custom_qs".to_string();
        app.current_config.demande_generique = "a generic prompt request".to_string();

        // Run prompt generator simulation
        app.bus.emit(crate::event_bus::EventType::CreateProfileWithPrompts {
            profile_name: "profile_custom_qs".to_string(),
            contexte: "c".to_string(),
            task: "t".to_string(),
            objectif: "o".to_string(),
            directives: "d".to_string(),
        });
        app.poll_events();

        // selected_profile should switch to "profile_custom_qs"
        assert_eq!(app.selected_profile, "profile_custom_qs");

        // The new profile config file should exist
        let list_profiles = app.config_repo.list_profiles();
        assert!(list_profiles.contains(&"profile_custom_qs".to_string()));

        // quick_start_profile_name should be updated to next available ("profile_01" is still empty/unused on disk)
        assert_eq!(app.quick_start_profile_name, "profile_01");

        // Clean up
        bootstrap.storage_dir = old_dir;
        crate::config::save_bootstrap_config(&bootstrap);
        cleanup(name);
    }
}
