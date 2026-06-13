use crate::orchestrator::helpers::{find_executable_for_window, draw_click_marker, is_local_url, parse_key_name};
use crate::config::SavedConfig;
use crate::orchestrator::VibePilotOrchestrator;
use std::sync::Arc;

#[test]
fn test_find_executable_chrome() {
    assert_eq!(find_executable_for_window("Google Chrome - New Tab"), Some("chrome.exe".to_string()));
}

#[test]
fn test_find_executable_firefox() {
    assert_eq!(find_executable_for_window("Mozilla Firefox - Home"), Some("firefox.exe".to_string()));
}

#[test]
fn test_find_executable_vscode() {
    assert_eq!(find_executable_for_window("main.rs - VibePilot - Visual Studio Code"), Some("code.exe".to_string()));
}

#[test]
fn test_find_executable_notepad() {
    assert_eq!(find_executable_for_window("Untitled - Notepad"), Some("notepad.exe".to_string()));
}

#[test]
fn test_find_executable_notepad_plus_plus() {
    assert_eq!(find_executable_for_window("new 1 - Notepad++"), Some("notepad++.exe".to_string()));
}

#[test]
fn test_find_executable_edge() {
    assert_eq!(find_executable_for_window("Microsoft Edge"), Some("msedge.exe".to_string()));
}

#[test]
fn test_find_executable_explorer() {
    assert_eq!(find_executable_for_window("File Explorer"), Some("explorer.exe".to_string()));
}

#[test]
fn test_find_executable_word() {
    assert_eq!(find_executable_for_window("Document1 - Microsoft Word"), Some("winword.exe".to_string()));
}

#[test]
fn test_find_executable_excel() {
    assert_eq!(find_executable_for_window("Budget - Microsoft Excel"), Some("excel.exe".to_string()));
}

#[test]
fn test_find_executable_discord() {
    assert_eq!(find_executable_for_window("Discord"), Some("discord.exe".to_string()));
}

#[test]
fn test_find_executable_terminal() {
    assert_eq!(find_executable_for_window("Windows Terminal"), Some("wt.exe".to_string()));
}

#[test]
fn test_find_executable_unknown() {
    assert_eq!(find_executable_for_window("Some Unknown App v3.2"), None);
}

#[test]
fn test_find_executable_case_insensitive() {
    assert_eq!(find_executable_for_window("GOOGLE CHROME"), Some("chrome.exe".to_string()));
    assert_eq!(find_executable_for_window("google chrome"), Some("chrome.exe".to_string()));
}

#[test]
fn test_find_executable_calculator() {
    assert_eq!(find_executable_for_window("Calculatrice"), Some("calc.exe".to_string()));
    assert_eq!(find_executable_for_window("Calculator"), Some("calc.exe".to_string()));
}

#[test]
fn test_find_executable_vlc() {
    assert_eq!(find_executable_for_window("VLC media player"), Some("vlc.exe".to_string()));
}

#[test]
fn test_find_executable_powershell() {
    assert_eq!(find_executable_for_window("Windows PowerShell"), Some("powershell.exe".to_string()));
}

#[test]
fn test_find_executable_cmd() {
    assert_eq!(find_executable_for_window("Command Prompt"), Some("cmd.exe".to_string()));
}

#[test]
fn test_find_executable_slack() {
    assert_eq!(find_executable_for_window("Slack | general"), Some("slack.exe".to_string()));
}

#[test]
fn test_draw_click_marker_center() {
    let mut img = image::DynamicImage::ImageRgba8(image::ImageBuffer::new(100, 100));
    draw_click_marker(&mut img, 0.5, 0.5, &[image::Rgba([255, 0, 0, 255])]);
    let rgba = img.as_rgba8().unwrap();
    let pixel = rgba.get_pixel(50, 50);
    assert_eq!(pixel, &image::Rgba([255, 0, 0, 255]));
}

#[test]
fn test_draw_click_marker_corner() {
    let mut img = image::DynamicImage::ImageRgba8(image::ImageBuffer::new(100, 100));
    draw_click_marker(&mut img, 0.0, 0.0, &[image::Rgba([255, 0, 0, 255])]);
    let rgba = img.as_rgba8().unwrap();
    let pixel = rgba.get_pixel(0, 0);
    assert_eq!(pixel, &image::Rgba([255, 0, 0, 255]));
}

#[test]
fn test_draw_click_marker_bottom_right() {
    let mut img = image::DynamicImage::ImageRgba8(image::ImageBuffer::new(200, 200));
    draw_click_marker(&mut img, 0.99, 0.99, &[image::Rgba([255, 0, 0, 255])]);
    // Should not panic even at near-edge coordinates
}

#[test]
fn test_draw_click_marker_crosshair_visible() {
    let mut img = image::DynamicImage::ImageRgba8(image::ImageBuffer::new(100, 100));
    draw_click_marker(&mut img, 0.5, 0.5, &[image::Rgba([255, 0, 0, 255])]);
    let rgba = img.as_rgba8().unwrap();
    // Check horizontal crosshair arm
    let pixel_left = rgba.get_pixel(40, 50);
    assert_eq!(pixel_left, &image::Rgba([255, 0, 0, 255]));
    let pixel_right = rgba.get_pixel(60, 50);
    assert_eq!(pixel_right, &image::Rgba([255, 0, 0, 255]));
    // Check vertical crosshair arm
    let pixel_up = rgba.get_pixel(50, 40);
    assert_eq!(pixel_up, &image::Rgba([255, 0, 0, 255]));
    let pixel_down = rgba.get_pixel(50, 60);
    assert_eq!(pixel_down, &image::Rgba([255, 0, 0, 255]));
}

#[test]
fn test_draw_click_marker_clustered() {
    let mut img = image::DynamicImage::ImageRgba8(image::ImageBuffer::new(100, 100));
    let colors = vec![
        image::Rgba([0, 0, 255, 255]), // Oldest: Blue
        image::Rgba([255, 0, 0, 255]), // Newest: Red
    ];
    draw_click_marker(&mut img, 0.5, 0.5, &colors);
    let rgba = img.as_rgba8().unwrap();
    
    // Innermost pixel at center should be Red (newest color)
    assert_eq!(rgba.get_pixel(50, 50), &image::Rgba([255, 0, 0, 255]));
    
    // Pixel on innermost ring (radius 6) should be Red
    assert_eq!(rgba.get_pixel(50, 56), &image::Rgba([255, 0, 0, 255]));
    
    // Pixel on second ring (radius 10) should be Blue (oldest color)
    assert_eq!(rgba.get_pixel(50, 60), &image::Rgba([0, 0, 255, 255]));
}

#[test]
fn test_is_local_url() {
    assert!(is_local_url("http://localhost:1234/v1/chat"));
    assert!(is_local_url("http://127.0.0.1:8000"));
    assert!(is_local_url("http://[::1]:1234"));
    assert!(!is_local_url("https://api.openai.com/v1"));
    assert!(!is_local_url("https://api.anthropic.com"));
}

#[test]
fn test_parse_key_name_all() {
    assert_eq!(parse_key_name("ctrl"), Some(rdev::Key::ControlLeft));
    assert_eq!(parse_key_name("control"), Some(rdev::Key::ControlLeft));
    assert_eq!(parse_key_name("enter"), Some(rdev::Key::Return));
    assert_eq!(parse_key_name("a"), Some(rdev::Key::KeyA));
    assert_eq!(parse_key_name("unknown_key"), None);
}

#[test]
fn test_orchestrator_getters_and_offsets() {
    let (bus, _rx) = crate::event_bus::EventBus::new();
    
    // Default config
    let config = SavedConfig::default();
    let config_repo = std::sync::Arc::new(super::MockConfigRepo {
        config: std::sync::Mutex::new(config),
    });

    let llm_client = std::sync::Arc::new(super::MockLlmClient {
        next_response: std::sync::Mutex::new(crate::llm_client::LlmResponse::default()),
        decompose_response: std::sync::Mutex::new(String::new()),
        call_count: std::sync::Mutex::new(0),
    });

    let capturer = std::sync::Arc::new(super::MockScreenCapturer {
        window_foreground_result: std::sync::Mutex::new(crate::screen_capture::WindowAnchorResult::AlreadyFocused),
    });

    let controller = std::sync::Arc::new(super::MockPeripheralController {
        actions: std::sync::Mutex::new(vec![]),
    });

    let orchestrator = VibePilotOrchestrator::new(
        config_repo.clone(),
        llm_client,
        capturer,
        controller,
        Arc::new(super::MockWaitManager),
        bus,
    );

    // Test refresh_windows
    orchestrator.refresh_windows();

    // Test get_available_windows
    let wins = orchestrator.get_available_windows();
    assert!(wins.contains(&"Mock Window".to_string()));
    assert!(wins.contains(&crate::config::ALL_SCREENS_KEY.to_string()));

    // Test get_target_offsets:
    // Case 1: targets is empty
    let mut cfg = SavedConfig::default();
    cfg.fenetres_surveillees = vec![];
    let offsets1 = orchestrator.get_target_offsets(&cfg);
    assert_eq!(offsets1.len(), 1);
    assert_eq!(offsets1[0].width, 1920);

    // Case 2: target is Desktop
    cfg.fenetres_surveillees = vec!["Desktop".to_string()];
    let offsets2 = orchestrator.get_target_offsets(&cfg);
    assert_eq!(offsets2.len(), 1);
    assert_eq!(offsets2[0].width, 1920);

    // Case 3: target is Mock Window
    cfg.fenetres_surveillees = vec!["Mock Window".to_string()];
    let offsets3 = orchestrator.get_target_offsets(&cfg);
    assert_eq!(offsets3.len(), 1);
    assert_eq!(offsets3[0].titre, "Mock Window");
    assert_eq!(offsets3[0].width, 500);

    // Case 4: target window is not found (falls back to monitors)
    cfg.fenetres_surveillees = vec!["Nonexistent Window".to_string()];
    let offsets4 = orchestrator.get_target_offsets(&cfg);
    assert_eq!(offsets4.len(), 1);
    assert_eq!(offsets4[0].width, 1920);
}

#[tokio::test]
async fn test_get_gpu_utilization() {
    let _ = crate::orchestrator::helpers::get_gpu_utilization().await;
}

#[test]
fn test_draw_past_clicks_basic() {
    let (bus, _rx) = crate::event_bus::EventBus::new();
    let config = SavedConfig::default();
    let config_repo = std::sync::Arc::new(super::MockConfigRepo {
        config: std::sync::Mutex::new(config),
    });

    let llm_client = std::sync::Arc::new(super::MockLlmClient {
        next_response: std::sync::Mutex::new(crate::llm_client::LlmResponse::default()),
        decompose_response: std::sync::Mutex::new(String::new()),
        call_count: std::sync::Mutex::new(0),
    });

    let capturer = std::sync::Arc::new(super::MockScreenCapturer {
        window_foreground_result: std::sync::Mutex::new(crate::screen_capture::WindowAnchorResult::AlreadyFocused),
    });

    let controller = std::sync::Arc::new(super::MockPeripheralController {
        actions: std::sync::Mutex::new(vec![]),
    });

    let orchestrator = VibePilotOrchestrator::new(
        config_repo.clone(),
        llm_client,
        capturer,
        controller,
        Arc::new(super::MockWaitManager),
        bus,
    );

    let mut img = image::DynamicImage::ImageRgba8(image::ImageBuffer::new(200, 200));
    let mut session_memory = crate::orchestrator::session_memory::SessionMemory::new();
    session_memory.record(crate::orchestrator::session_memory::ActionStep {
        timestamp: "12:00:00".to_string(),
        action_type: "CLICK".to_string(),
        coordinates: Some((0.5, 0.5)),
        text_typed: None,
        llm_report: "Test Click".to_string(),
        was_repeated: false,
    });

    orchestrator.draw_past_clicks(&mut img, &None, &session_memory, Some((0.6, 0.6)));
}

#[test]
fn test_apply_coordinate_calibration_shifting() {
    let (bus, _rx) = crate::event_bus::EventBus::new();
    let config = SavedConfig::default();
    let config_repo = std::sync::Arc::new(super::MockConfigRepo {
        config: std::sync::Mutex::new(config),
    });

    let llm_client = std::sync::Arc::new(super::MockLlmClient {
        next_response: std::sync::Mutex::new(crate::llm_client::LlmResponse::default()),
        decompose_response: std::sync::Mutex::new(String::new()),
        call_count: std::sync::Mutex::new(0),
    });

    let capturer = std::sync::Arc::new(super::MockScreenCapturer {
        window_foreground_result: std::sync::Mutex::new(crate::screen_capture::WindowAnchorResult::AlreadyFocused),
    });

    let controller = std::sync::Arc::new(super::MockPeripheralController {
        actions: std::sync::Mutex::new(vec![]),
    });

    let orchestrator = VibePilotOrchestrator::new(
        config_repo.clone(),
        llm_client,
        capturer,
        controller,
        Arc::new(super::MockWaitManager),
        bus,
    );

    let mut rx = 0.5;
    let mut ry = 0.5;
    let mut base_click_coordinates = None;
    let mut coordinate_calibration_retries = 0;

    let cfg = SavedConfig::default();
    orchestrator.apply_coordinate_calibration(&cfg, &mut rx, &mut ry, &mut base_click_coordinates, &mut coordinate_calibration_retries);
    assert_eq!(base_click_coordinates, Some((0.5, 0.5)));
    assert_eq!(coordinate_calibration_retries, 0);

    orchestrator.apply_coordinate_calibration(&cfg, &mut rx, &mut ry, &mut base_click_coordinates, &mut coordinate_calibration_retries);
    assert_eq!(coordinate_calibration_retries, 1);
    assert!(rx < 0.5);
}

#[tokio::test]
async fn test_compress_session_history_basic() {
    let (bus, _rx) = crate::event_bus::EventBus::new();
    let config = SavedConfig::default();
    let config_repo = std::sync::Arc::new(super::MockConfigRepo {
        config: std::sync::Mutex::new(config),
    });

    let llm_client = std::sync::Arc::new(super::MockLlmClient {
        next_response: std::sync::Mutex::new(crate::llm_client::LlmResponse::default()),
        decompose_response: std::sync::Mutex::new(String::new()),
        call_count: std::sync::Mutex::new(0),
    });

    let capturer = std::sync::Arc::new(super::MockScreenCapturer {
        window_foreground_result: std::sync::Mutex::new(crate::screen_capture::WindowAnchorResult::AlreadyFocused),
    });

    let controller = std::sync::Arc::new(super::MockPeripheralController {
        actions: std::sync::Mutex::new(vec![]),
    });

    let orchestrator = VibePilotOrchestrator::new(
        config_repo.clone(),
        llm_client,
        capturer,
        controller,
        Arc::new(super::MockWaitManager),
        bus,
    );

    let mut session_memory = crate::orchestrator::session_memory::SessionMemory::new();
    for i in 0..11 {
        session_memory.record(crate::orchestrator::session_memory::ActionStep {
            timestamp: format!("12:00:{:02}", i),
            action_type: "CLICK".to_string(),
            coordinates: Some((0.5, 0.5)),
            text_typed: None,
            llm_report: format!("Click {}", i),
            was_repeated: false,
        });
    }

    let cfg = SavedConfig::default();
    orchestrator.compress_session_history(&cfg, &mut session_memory).await;
    assert_eq!(session_memory.compressed_history, "summary");
    assert_eq!(session_memory.steps.len(), 2);
}
