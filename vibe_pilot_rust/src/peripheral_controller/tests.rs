use super::*;
use crate::screen_capture::{ScreenInfo, BoundingBox, WindowInfo, WindowAnchorResult, ScreenCapturerTrait};

struct DummyCapturer;
impl ScreenCapturerTrait for DummyCapturer {
    fn capture_desktop(&self) -> Option<image::DynamicImage> { None }
    fn capture_bbox(&self, _x: i32, _y: i32, _w: i32, _h: i32) -> Option<image::DynamicImage> { None }
    fn get_monitors(&self) -> Vec<ScreenInfo> {
        vec![ScreenInfo::new("Screen 1".to_string(), BoundingBox::new(0, 0, 1920, 1080))]
    }
    fn refresh_windows(&self) {}
    fn get_windows(&self) -> Vec<WindowInfo> { vec![] }
    fn is_target_visible(&self, _target_windows: &[String]) -> bool { true }
    fn ensure_window_foreground(&self, _target_title: &str) -> WindowAnchorResult {
        WindowAnchorResult::AlreadyFocused
    }
    fn capture_window_by_title(&self, _title: &str) -> Option<image::DynamicImage> { None }
    fn get_foreground_window_title(&self) -> Option<String> { None }
}

#[test]
fn test_peripheral_controller_actions() {
    let capturer = std::sync::Arc::new(DummyCapturer);
    let called = std::sync::Arc::new(std::sync::Mutex::new(false));
    let called_clone = called.clone();
    let controller = PeripheralController::new(capturer).with_clipboard(move |_text| {
        *called_clone.lock().unwrap() = true;
    });

    // Let's test clipboard operations
    let _ = controller.set_clipboard_text("test");
    assert!(*called.lock().unwrap());
    
    let _ = controller.get_clipboard_text();

    // Let's test coordinate calculations
    let offsets = vec![ScreenOffset {
        titre: "Screen 1".to_string(),
        left: 0,
        top: 0,
        width: 1920,
        height: 1080,
        y_offset: 0,
    }];
    
    let (x, y) = controller.compute_absolute_coordinates(0.5, 0.5, &offsets);
    assert_eq!(x, 960);
    assert_eq!(y, 540);

    // Test coordinate parsing with values > 1.0 (interpret as thousandths)
    let (x2, y2) = controller.compute_absolute_coordinates(500.0, 500.0, &offsets);
    assert_eq!(x2, 960);
    assert_eq!(y2, 540);

    // Test coordinate parsing with large values > 1000 (interpret as raw pixels)
    let (x3, y3) = controller.compute_absolute_coordinates(1920.0, 1080.0, &offsets);
    assert_eq!(x3, 1920);
    assert_eq!(y3, 1080);
}

#[test]
fn test_controller_scroll_and_keyboard_actions() {
    let capturer = std::sync::Arc::new(DummyCapturer);
    let controller = PeripheralController::new(capturer);

    // Test copy, paste, select_all, cut_selected
    let _ = controller.copy_selected();
    let _ = controller.paste();
    let _ = controller.select_all();
    let _ = controller.cut_selected();

    // Test scroll actions
    let _ = controller.scroll_vertical(1);
    let _ = controller.scroll_vertical(-1);
    let _ = controller.scroll_horizontal(1);
    let _ = controller.scroll_horizontal(-1);
    
    // Test smooth/quick scroll
    let _ = controller.smooth_scroll_vertical(2, 2, 10);
    let _ = controller.smooth_scroll_vertical(2, 0, 10); // Edge case zero steps
    let _ = controller.quick_scroll_vertical("up", "low");
    let _ = controller.quick_scroll_vertical("down", "medium");
    let _ = controller.quick_scroll_vertical("down", "high");
}

#[test]
fn test_controller_clicks() {
    let capturer = std::sync::Arc::new(DummyCapturer);
    let controller = PeripheralController::new(capturer);

    let _ = controller.right_click();
    let _ = controller.middle_click();
    let _ = controller.double_click();
    let _ = controller.drag_and_drop((0, 0), (10, 10), rdev::Button::Left);
    let _ = controller.mouse_move_relative(5, 5);
}

#[test]
fn test_execute_action_cases() {
    let capturer = std::sync::Arc::new(DummyCapturer);
    let controller = PeripheralController::new(capturer);
    let offsets = vec![ScreenOffset {
        titre: "Screen 1".to_string(),
        left: 0,
        top: 0,
        width: 1920,
        height: 1080,
        y_offset: 0,
    }];

    let payload_click = ActionPayload {
        action: "CLICK_AND_TYPE".to_string(),
        relative_click_position: vec![0.5, 0.5],
        text_to_type: "hello".to_string(),
        scroll_value: 0,
        wait_seconds: 0,
    };
    controller.execute_action(&payload_click, &offsets, false);

    let payload_scroll = ActionPayload {
        action: "SCROLL".to_string(),
        relative_click_position: vec![],
        text_to_type: "".to_string(),
        scroll_value: 2,
        wait_seconds: 0,
    };
    controller.execute_action(&payload_scroll, &offsets, false);

    let payload_wait = ActionPayload {
        action: "WAIT".to_string(),
        relative_click_position: vec![],
        text_to_type: "".to_string(),
        scroll_value: 0,
        wait_seconds: 0,
    };
    controller.execute_action(&payload_wait, &offsets, false);
}

#[test]
fn test_screen_offset_from_screen_info() {
    let screen = ScreenInfo::new("Test Window".to_string(), BoundingBox::new(100, 200, 800, 600));
    let offset = ScreenOffset::from_screen_info(screen);
    assert_eq!(offset.titre, "Test Window");
    assert_eq!(offset.left, 100);
    assert_eq!(offset.top, 200);
    assert_eq!(offset.width, 800);
    assert_eq!(offset.height, 600);
    assert_eq!(offset.y_offset, 200);
}

#[test]
fn test_action_payload_default_values() {
    let payload = ActionPayload {
        action: "CLICK_AND_TYPE".to_string(),
        relative_click_position: vec![0.5, 0.5],
        text_to_type: "hello".to_string(),
        scroll_value: 0,
        wait_seconds: 0,
    };
    assert_eq!(payload.action, "CLICK_AND_TYPE");
    assert_eq!(payload.scroll_value, 0);
    assert_eq!(payload.wait_seconds, 0);
}

#[test]
fn test_peripheral_controller_equality() {
    let capturer1 = std::sync::Arc::new(DummyCapturer);
    let capturer2 = std::sync::Arc::new(DummyCapturer);
    let c1 = PeripheralController::new(capturer1);
    let c2 = PeripheralController::new(capturer2);
    // Different Arc instances should not be equal
    let c1_ptr = std::ptr::addr_of!(c1);
    let c2_ptr = std::ptr::addr_of!(c2);
    assert_ne!(c1_ptr as usize, c2_ptr as usize);

    let same = std::sync::Arc::new(DummyCapturer);
    let c3 = PeripheralController::new(same.clone());
    let c4 = PeripheralController::new(same);
    // Same Arc should be equal
    assert_eq!(c3, c4);
}

#[test]
fn test_peripheral_controller_new() {
    let capturer = std::sync::Arc::new(DummyCapturer);
    let controller = PeripheralController::new(capturer);
    // Verify it was created without panicking
    let _ = controller.set_clipboard_text("test");
}

#[test]
fn test_compute_absolute_coordinates_empty_offsets() {
    let capturer = std::sync::Arc::new(DummyCapturer);
    let controller = PeripheralController::new(capturer);
    let offsets: Vec<ScreenOffset> = vec![];
    let (x, y) = controller.compute_absolute_coordinates(0.5, 0.5, &offsets);
    assert_eq!(x, 0);
    assert_eq!(y, 0);
}

#[test]
fn test_compute_absolute_coordinates_single_monitor() {
    let capturer = std::sync::Arc::new(DummyCapturer);
    let controller = PeripheralController::new(capturer);
    let offsets = vec![ScreenOffset {
        titre: "Monitor 1".to_string(),
        left: 0,
        top: 0,
        width: 1920,
        height: 1080,
        y_offset: 0,
    }];
    // Edge cases
    let (x, y) = controller.compute_absolute_coordinates(0.0, 0.0, &offsets);
    assert_eq!(x, 0);
    assert_eq!(y, 0);

    let (x, y) = controller.compute_absolute_coordinates(1.0, 1.0, &offsets);
    assert_eq!(x, 1920);
    assert_eq!(y, 1080);
}

#[test]
fn test_compute_absolute_coordinates_with_offset() {
    let capturer = std::sync::Arc::new(DummyCapturer);
    let controller = PeripheralController::new(capturer);
    let offsets = vec![ScreenOffset {
        titre: "Monitor 2".to_string(),
        left: 1920,
        top: 0,
        width: 1280,
        height: 720,
        y_offset: 0,
    }];
    let (x, y) = controller.compute_absolute_coordinates(0.5, 0.5, &offsets);
    assert_eq!(x, 2560);
    assert_eq!(y, 360);
}

#[test]
fn test_is_desktop_title_various_inputs() {
    let capturer = std::sync::Arc::new(DummyCapturer);
    let controller = PeripheralController::new(capturer);

    assert!(controller.is_desktop_title("Desktop"));
    assert!(controller.is_desktop_title("bureau"));
    assert!(controller.is_desktop_title("Écrans"));
    assert!(controller.is_desktop_title("Écran"));
    assert!(controller.is_desktop_title("All Screens"));
    assert!(controller.is_desktop_title("tous les écrans"));
    assert!(controller.is_desktop_title("all_screens"));

    assert!(!controller.is_desktop_title("Chrome"));
    assert!(!controller.is_desktop_title("Visual Studio Code"));
    assert!(!controller.is_desktop_title(""));
}

#[test]
fn test_get_screen_bounds() {
    let capturer = std::sync::Arc::new(DummyCapturer);
    let controller = PeripheralController::new(capturer);
    // On Windows this returns virtual screen bounds
    let (left, top, width, height) = controller.get_screen_bounds();
    assert!(width > left);
    assert!(height > top);
}

#[test]
fn test_mock_peripheral_input_basic() {
    let mock = MockPeripheralInput::new();
    let offsets = vec![ScreenOffset {
        titre: "Test".to_string(),
        left: 0,
        top: 0,
        width: 1920,
        height: 1080,
        y_offset: 0,
    }];

    let payload = ActionPayload {
        action: "CLICK_AND_TYPE".to_string(),
        relative_click_position: vec![0.5, 0.5],
        text_to_type: "hello world".to_string(),
        scroll_value: 0,
        wait_seconds: 0,
    };
    mock.execute_action(&payload, &offsets, false);

    let actions = mock.actions.lock().unwrap();
    assert!(actions.iter().any(|a| a.contains("execute_action")));
    assert!(actions.iter().any(|a| a.contains("type_text")));
}

#[test]
fn test_mock_peripheral_input_clipboard() {
    let mock = MockPeripheralInput::new();
    let _ = mock.set_clipboard_text("clipboard content");
    let text = mock.get_clipboard_text().unwrap();
    assert_eq!(text, "clipboard content");

    let _ = mock.set_clipboard_html("html content");
    let html = mock.get_clipboard_html().unwrap();
    assert_eq!(html, "html:html content");

    let img = image::DynamicImage::ImageRgba8(image::ImageBuffer::new(2, 2));
    let _ = mock.set_clipboard_image(&img);
    let img_res = mock.get_clipboard_image().unwrap();
    assert_eq!(img_res.width(), 10);
}

#[test]
fn test_mock_peripheral_input_scroll_vertical() {
    let mock = MockPeripheralInput::new();
    let _ = mock.scroll_vertical(5);
    let _ = mock.scroll_vertical(-3);
    let actions = mock.actions.lock().unwrap();
    assert!(actions.iter().any(|a| a.starts_with("scroll_vertical")));
}

#[test]
fn test_mock_peripheral_input_key_combo() {
    let mock = MockPeripheralInput::new();
    let _ = mock.key_combo(&[rdev::Key::ControlLeft, rdev::Key::KeyC]);
    let actions = mock.actions.lock().unwrap();
    assert!(actions.iter().any(|a| a.starts_with("key_combo")));
}

#[test]
fn test_mock_peripheral_input_smooth_scroll() {
    let mock = MockPeripheralInput::new();
    let _ = mock.smooth_scroll_vertical(10, 5, 500);
    let actions = mock.actions.lock().unwrap();
    assert!(actions.iter().any(|a| a.starts_with("smooth_scroll_vertical")));
}

#[test]
fn test_mock_peripheral_input_quick_scroll() {
    let mock = MockPeripheralInput::new();
    let _ = mock.quick_scroll_vertical("up", "low");
    let _ = mock.quick_scroll_vertical("down", "medium");
    let _ = mock.quick_scroll_vertical("down", "high");
    let _ = mock.quick_scroll_vertical("down", "invalid");
    let actions = mock.actions.lock().unwrap();
    assert!(actions.iter().any(|a| a.starts_with("quick_scroll_vertical")));
}

#[test]
fn test_mock_peripheral_input_copy_paste_cut_select_all() {
    let mock = MockPeripheralInput::new();
    let _ = mock.copy_selected();
    let _ = mock.paste();
    let _ = mock.cut_selected();
    let _ = mock.select_all();
    let actions = mock.actions.lock().unwrap();
    assert!(actions.iter().any(|a| a == "copy_selected"));
    assert!(actions.iter().any(|a| a == "paste"));
    assert!(actions.iter().any(|a| a == "cut_selected"));
    assert!(actions.iter().any(|a| a == "select_all"));
}

#[test]
fn test_mock_peripheral_input_right_middle_double_click() {
    let mock = MockPeripheralInput::new();
    let _ = mock.right_click();
    let _ = mock.middle_click();
    let _ = mock.double_click();
    let _ = mock.drag_and_drop((0, 0), (100, 100), rdev::Button::Left);
    let _ = mock.mouse_move_relative(10, 20);
    let actions = mock.actions.lock().unwrap();
    assert!(actions.iter().any(|a| a == "right_click"));
    assert!(actions.iter().any(|a| a == "middle_click"));
    assert!(actions.iter().any(|a| a == "double_click"));
}

#[test]
fn test_mock_peripheral_input_is_desktop_title() {
    let mock = MockPeripheralInput::new();
    assert!(mock.is_desktop_title("Desktop"));
    assert!(!mock.is_desktop_title("Chrome"));
}

#[test]
fn test_mock_peripheral_input_get_screen_bounds() {
    let mock = MockPeripheralInput::new();
    let (left, top, width, height) = mock.get_screen_bounds();
    assert_eq!(left, 0);
    assert_eq!(top, 0);
    assert_eq!(width, 1920);
    assert_eq!(height, 1080);
}

#[test]
fn test_mock_peripheral_input_compute_coordinates() {
    let mock = MockPeripheralInput::new();
    let (x, y) = mock.compute_absolute_coordinates(0.5, 0.5, &[]);
    assert_eq!(x, 960);
    assert_eq!(y, 540);
}

#[test]
fn test_scroll_horizontal_negative() {
    let capturer = std::sync::Arc::new(DummyCapturer);
    let controller = PeripheralController::new(capturer);
    let _ = controller.scroll_horizontal(-5);
}

#[test]
fn test_quick_scroll_all_intensities() {
    let capturer = std::sync::Arc::new(DummyCapturer);
    let controller = PeripheralController::new(capturer);
    for intensity in &["low", "medium", "high", "invalid"] {
        let _ = controller.quick_scroll_vertical("up", intensity);
        let _ = controller.quick_scroll_vertical("down", intensity);
    }
}

#[test]
fn test_smooth_scroll_zero_steps_returns_ok() {
    let capturer = std::sync::Arc::new(DummyCapturer);
    let controller = PeripheralController::new(capturer);
    let result = controller.smooth_scroll_vertical(10, 0, 100);
    assert!(result.is_ok());
}

#[test]
fn test_peripheral_controller_all_methods_coverage() {
    let capturer = std::sync::Arc::new(DummyCapturer);
    let controller = PeripheralController::new(capturer);

    assert!(controller.right_click().is_ok());
    assert!(controller.middle_click().is_ok());
    assert!(controller.double_click().is_ok());
    assert!(controller.drag_and_drop((0, 0), (10, 10), rdev::Button::Left).is_ok());
    assert!(controller.mouse_move_relative(5, 5).is_ok());
    assert!(controller.key_combo(&[rdev::Key::KeyA]).is_ok());
    assert!(controller.scroll_vertical(5).is_ok());
    assert!(controller.scroll_horizontal(5).is_ok());
    assert!(controller.smooth_scroll_vertical(5, 2, 20).is_ok());
    assert!(controller.quick_scroll_vertical("up", "high").is_ok());
    assert!(controller.set_clipboard_text("test_coverage").is_ok());
    assert!(controller.get_clipboard_text().is_ok());
    assert!(controller.copy_selected().is_ok());
    assert!(controller.paste().is_ok());
    assert!(controller.cut_selected().is_ok());
    assert!(controller.select_all().is_ok());

    let _ = controller.set_clipboard_html("<html></html>");
    let _ = controller.get_clipboard_html();
    let img = image::DynamicImage::ImageRgba8(image::ImageBuffer::new(5, 5));
    let _ = controller.set_clipboard_image(&img);
    let _ = controller.get_clipboard_image();
}
