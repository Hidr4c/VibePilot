use crate::screen_capture::{ScreenCapturer, ScreenInfo, BoundingBox, ScreenCapturerTrait};

#[test]
fn test_new_capturer() {
    let capturer = ScreenCapturer::new();
    let monitors = capturer.get_monitors();
    assert!(!monitors.is_empty());
}

#[test]
fn test_capture_desktop() {
    let capturer = ScreenCapturer::new();
    let result = capturer.capture_desktop();
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
    assert!(bbox.contains(100, 100));
    assert!(!bbox.contains(350, 350));
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
    let result = capturer.is_target_visible(&["Code".to_string(), "Explorer".to_string()]);
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
