use crate::peripheral_controller::{ActionPayload, PeripheralController, ScreenOffset, PeripheralInput, MouseInput, KeyboardInput, ScrollInput, ClipboardInput, CoordinateMapping};
use crate::screen_capture::{ScreenCapturer, ScreenInfo, BoundingBox, ScreenCapturerTrait};

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
        .map(ScreenOffset::from_screen_info)
        .collect();
    let controller = PeripheralController::new(std::sync::Arc::new(capturer));

    let (x, y) = controller.compute_absolute_coordinates(0.5, 0.5, &offsets);
    assert!(x >= 0);
    assert!(y >= 0);
}

#[test]
fn test_coordinate_clamping() {
    let capturer = ScreenCapturer::new();
    let offsets: Vec<ScreenOffset> = capturer.get_monitors()
        .into_iter()
        .map(ScreenOffset::from_screen_info)
        .collect();
    let controller = PeripheralController::new(std::sync::Arc::new(capturer));

    let (x1, y1) = controller.compute_absolute_coordinates(0.0, 0.0, &offsets);
    let min_x = offsets.iter().map(|o| o.left).min().unwrap_or(0);
    let min_y = offsets.iter().map(|o| o.top).min().unwrap_or(0);
    assert_eq!(x1, min_x);
    assert_eq!(y1, min_y);

    let (x2, y2) = controller.compute_absolute_coordinates(1.0, 1.0, &offsets);
    assert!(x2 > min_x);
    assert!(y2 > min_y);
}

#[test]
fn test_is_desktop_title() {
    let capturer = ScreenCapturer::new();
    let controller = PeripheralController::new(std::sync::Arc::new(capturer));

    assert!(controller.is_desktop_title("Desktop"));
    assert!(controller.is_desktop_title("Bureau"));
    assert!(controller.is_desktop_title("ECRAN"));
    assert!(controller.is_desktop_title("ecran"));
    assert!(controller.is_desktop_title("all_screens"));
    assert!(controller.is_desktop_title("tous les ecrans"));
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
    assert!(x >= 1920);
    assert!(y >= 0);
}

#[test]
fn test_sub_traits_mouse_input() {
    let capturer = ScreenCapturer::new();
    let controller = PeripheralController::new(std::sync::Arc::new(capturer));
    assert!(controller.right_click().is_ok());
}

#[test]
fn test_sub_traits_keyboard_input() {
    let capturer = ScreenCapturer::new();
    let controller = PeripheralController::new(std::sync::Arc::new(capturer));
    assert!(controller.key_combo(&[]).is_ok());
}

#[test]
fn test_sub_traits_scroll_input() {
    let capturer = ScreenCapturer::new();
    let controller = PeripheralController::new(std::sync::Arc::new(capturer));
    assert!(controller.scroll_vertical(1).is_ok());
    assert!(controller.scroll_horizontal(1).is_ok());
}

#[test]
fn test_sub_traits_clipboard_input() {
    let capturer = ScreenCapturer::new();
    let controller = PeripheralController::new(std::sync::Arc::new(capturer));
    assert!(controller.copy_selected().is_ok());
    assert!(controller.paste().is_ok());
}

#[test]
fn test_sub_traits_coordinate_mapping() {
    let capturer = ScreenCapturer::new();
    let controller = PeripheralController::new(std::sync::Arc::new(capturer));
    let bounds = controller.get_screen_bounds();
    assert!(bounds.2 > bounds.0);
    assert!(bounds.3 > bounds.1);
}

use crate::screen_capture::MockScreenCapturer;
use rand::Rng;

#[test]
fn test_mocked_single_monitor_window_click() {
    let mut capturer = MockScreenCapturer::new();
    capturer.monitors = vec![
        ScreenInfo::new("Screen 1".to_string(), BoundingBox::new(0, 0, 1920, 1080))
    ];
    let controller = PeripheralController::new(std::sync::Arc::new(capturer));

    // Target window "Chrome" at (100, 150, 800, 600)
    let offsets = vec![ScreenOffset {
        titre: "Chrome".to_string(),
        left: 100,
        top: 150,
        width: 800,
        height: 600,
        y_offset: 150,
    }];

    // Generate random relative coordinates
    let mut rng = rand::thread_rng();
    for _ in 0..100 {
        let rx: f64 = rng.gen_range(0.0..1.0);
        let ry: f64 = rng.gen_range(0.0..1.0);

        let (abs_x, abs_y) = controller.compute_absolute_coordinates(rx, ry, &offsets);
        let expected_x = 100 + (800.0 * rx) as i32;
        let expected_y = 150 + (600.0 * ry) as i32;
        assert_eq!(abs_x, expected_x);
        assert_eq!(abs_y, expected_y);
    }
}

#[test]
fn test_mocked_multi_monitor_desktop_click() {
    let mut capturer = MockScreenCapturer::new();
    capturer.monitors = vec![
        ScreenInfo::new("Screen 1 (1080p)".to_string(), BoundingBox::new(0, 0, 1920, 1080)),
        ScreenInfo::new("Screen 2 (2K)".to_string(), BoundingBox::new(1920, 0, 2560, 1440)),
        ScreenInfo::new("Screen 3 (4K)".to_string(), BoundingBox::new(4480, 0, 3840, 2160)),
    ];
    let controller = PeripheralController::new(std::sync::Arc::new(capturer));

    // When targeting the desktop, get_target_offsets will return all monitors.
    let offsets = vec![
        ScreenOffset { titre: "Screen 1".to_string(), left: 0, top: 0, width: 1920, height: 1080, y_offset: 0 },
        ScreenOffset { titre: "Screen 2".to_string(), left: 1920, top: 0, width: 2560, height: 1440, y_offset: 0 },
        ScreenOffset { titre: "Screen 3".to_string(), left: 4480, top: 0, width: 3840, height: 2160, y_offset: 0 },
    ];

    // Combined bounds cover (0, 0) to (8320, 2160).
    // Test random coordinate mapping on the virtual desktop
    let mut rng = rand::thread_rng();
    for _ in 0..100 {
        let rx: f64 = rng.gen_range(0.0..1.0);
        let ry: f64 = rng.gen_range(0.0..1.0);

        let (abs_x, abs_y) = controller.compute_absolute_coordinates(rx, ry, &offsets);
        let expected_x = (8320.0 * rx) as i32;
        let expected_y = (2160.0 * ry) as i32;
        assert_eq!(abs_x, expected_x);
        assert_eq!(abs_y, expected_y);
    }
}

#[test]
fn test_mocked_multi_monitor_window_click() {
    let mut capturer = MockScreenCapturer::new();
    capturer.monitors = vec![
        ScreenInfo::new("Screen 1 (1080p)".to_string(), BoundingBox::new(0, 0, 1920, 1080)),
        ScreenInfo::new("Screen 2 (2K)".to_string(), BoundingBox::new(1920, 0, 2560, 1440)),
        ScreenInfo::new("Screen 3 (4K)".to_string(), BoundingBox::new(4480, 0, 3840, 2160)),
    ];
    let controller = PeripheralController::new(std::sync::Arc::new(capturer));

    // Target window "Notepad" at (4500, 100, 800, 600) on Screen 3
    let offsets = vec![ScreenOffset {
        titre: "Notepad".to_string(),
        left: 4500,
        top: 100,
        width: 800,
        height: 600,
        y_offset: 100,
    }];

    // Test random coordinate mapping within the target window
    let mut rng = rand::thread_rng();
    for _ in 0..100 {
        let rx: f64 = rng.gen_range(0.0..1.0);
        let ry: f64 = rng.gen_range(0.0..1.0);

        let (abs_x, abs_y) = controller.compute_absolute_coordinates(rx, ry, &offsets);
        let expected_x = 4500 + (800.0 * rx) as i32;
        let expected_y = 100 + (600.0 * ry) as i32;
        assert_eq!(abs_x, expected_x);
        assert_eq!(abs_y, expected_y);
    }
}

#[test]
fn test_noise_coordinate_out_of_bounds_bug() {
    let mut capturer = MockScreenCapturer::new();
    capturer.monitors = vec![
        ScreenInfo::new("Screen 1".to_string(), BoundingBox::new(0, 0, 1920, 1080))
    ];
    let controller = PeripheralController::new(std::sync::Arc::new(capturer));

    let offsets = vec![ScreenOffset {
        titre: "Chrome".to_string(),
        left: 100,
        top: 150,
        width: 800,
        height: 600,
        y_offset: 150,
    }];

    // If coordinates are slightly out of bounds, say (1.01, 0.5), it is a relative coordinate with model noise.
    // It should map to absolute coordinates without being scaled by width division.
    // Expected: 100 + 800 * 1.01 = 908
    let (abs_x, abs_y) = controller.compute_absolute_coordinates(1.01, 0.5, &offsets);
    
    assert_eq!(abs_x, 908, "Bug: relative coordinate slightly > 1.0 was snapped to the left edge!");
    assert_eq!(abs_y, 450);
}

