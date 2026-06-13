use super::*;

#[test]
fn test_new_capturer() {
    let c = ScreenCapturer::new();
    assert!(!c.get_monitors().is_empty());
    c.refresh_windows();
    let _wins = c.get_windows();
    // verify no panic and we can read them
    let _monitors = c.get_monitors();
}

#[test]
fn test_bounding_box_contains() {
    let bbox = BoundingBox::new(100, 100, 200, 200);
    assert!(bbox.contains(150, 150));
    assert!(!bbox.contains(50, 50));
}

#[test]
fn test_bounding_box_new() {
    let bbox = BoundingBox::new(0, 0, 1920, 1080);
    assert_eq!(bbox.left, 0); assert_eq!(bbox.width, 1920);
}

#[test]
fn test_window_anchor_result_equality() {
    assert_eq!(WindowAnchorResult::AlreadyFocused, WindowAnchorResult::AlreadyFocused);
    assert_ne!(WindowAnchorResult::AlreadyFocused, WindowAnchorResult::Refocused);
    assert_ne!(WindowAnchorResult::Refocused, WindowAnchorResult::WindowNotFound);
}

#[test]
fn test_get_foreground_window_title() {
    let c = ScreenCapturer::new();
    // On a desktop with windows, there should be a foreground window
    // This may return None in headless/CI environments
    let _title = c.get_foreground_window_title();
}

#[test]
fn test_is_target_visible() {
    let c = ScreenCapturer::new();
    assert!(c.is_target_visible(&[]));
    assert!(c.is_target_visible(&["all screens".to_string()]));
    assert!(c.is_target_visible(&["tous les ecrans".to_string()]));
    // Add a mock window to the list to test
    if let Ok(mut wins) = c.windows.lock() {
        wins.push(WindowInfo {
            title: "MyTestWindowTitle".to_string(),
            hwnd: 12345,
            visible: true,
            bbox: BoundingBox::new(10, 10, 100, 100),
        });
    }
    assert!(c.is_target_visible(&["MyTestWindowTitle".to_string()]));
    assert!(c.is_target_visible(&["mytestwindow".to_string()]));
    assert!(!c.is_target_visible(&["NonexistentTarget".to_string()]));
}

#[test]
fn test_ensure_window_foreground_not_found() {
    let c = ScreenCapturer::new();
    // Non-existent window should return WindowNotFound
    let res = c.ensure_window_foreground("NonexistentWindowTitleXYZ");
    assert_eq!(res, WindowAnchorResult::WindowNotFound);
}

#[test]
fn test_capture_window_by_title_not_found() {
    let c = ScreenCapturer::new();
    let _img = c.capture_window_by_title("NonexistentWindowTitleXYZ");
}

#[test]
fn test_mock_capturer_impl() {
    let mc = MockScreenCapturer::new();
    assert!(mc.is_target_visible(&["any".to_string()]));
    {
        let mut wins = mc.windows.lock().unwrap();
        wins.push(WindowInfo {
            title: "any".to_string(),
            hwnd: 123,
            visible: true,
            bbox: BoundingBox::new(0, 0, 100, 100),
        });
    }
    assert_eq!(mc.ensure_window_foreground("any"), WindowAnchorResult::AlreadyFocused);
    assert_eq!(mc.get_foreground_window_title(), Some("Desktop".to_string()));
    let wins = mc.get_windows();
    assert!(!wins.is_empty());
    assert!(mc.capture_desktop().is_some());
    assert!(mc.capture_window_by_title("title").is_some());
    assert!(mc.capture_bbox(0, 0, 10, 10).is_some());
}

#[cfg(target_os = "windows")]
#[test]
fn test_win32_helpers() {
    use windows::Win32::Foundation::{HWND, LPARAM, BOOL};
    use crate::screen_capture::win32_helpers::{is_same_or_child_process, enum_windows_proc};

    // Test with null HWNDs
    assert!(!is_same_or_child_process(HWND::default(), HWND::default()));
    
    // Test enum_windows_proc with null pointer
    let result = enum_windows_proc(HWND::default(), LPARAM(0));
    assert_eq!(result, BOOL(1));

    // Test enum_windows_proc with valid collector pointer but invisible/invalid HWND
    let mut collector = Vec::<WindowInfo>::new();
    let data_ptr = &mut collector as *mut Vec<WindowInfo>;
    let result2 = enum_windows_proc(HWND::default(), LPARAM(data_ptr as isize));
    assert_eq!(result2, BOOL(1));

    // Test with actual desktop and shell windows to trigger process name reading
    unsafe {
        use windows::Win32::UI::WindowsAndMessaging::{GetDesktopWindow, GetShellWindow};
        let desktop = GetDesktopWindow();
        let shell = GetShellWindow();
        if !desktop.0.is_null() {
            assert!(is_same_or_child_process(desktop, desktop));
        }
        if !desktop.0.is_null() && !shell.0.is_null() {
            let _ = is_same_or_child_process(desktop, shell);
        }
    }
}

#[test]
fn test_null_pii_masker() {
    use crate::screen_capture::pii_masker::{NullPiiMasker, PiiMasker};
    let masker = NullPiiMasker::new();
    let mut img = image::DynamicImage::new_rgba8(100, 100);
    masker.mask_pii(&mut img);
}

#[test]
fn test_ocr_pii_masker_helpers() {
    use crate::screen_capture::pii_masker::{OcrPiiMasker, PiiMasker};
    let masker = OcrPiiMasker::new("eng".to_string());
    
    assert!(masker.is_credit_card("1234-5678-9012-3456"));
    assert!(masker.is_credit_card("1234567890123"));
    assert!(!masker.is_credit_card("123"));

    assert!(masker.is_password_bullets("••••••••"));
    assert!(masker.is_password_bullets("******"));
    assert!(masker.is_password_bullets("xxxxxx"));
    assert!(!masker.is_password_bullets("hello"));
    assert!(!masker.is_password_bullets(""));

    assert!(masker.is_sensitive_key_pattern("sk-Proj12345678901234567890"));
    assert!(masker.is_sensitive_key_pattern("api-key-12345"));
    assert!(masker.is_sensitive_key_pattern("Ab1Cd2Ef3Gh4Ij5Kl6Mn7Op8Qr9St"));
    assert!(!masker.is_sensitive_key_pattern("just a normal sentence text"));

    let mut img = image::DynamicImage::new_rgba8(10, 10);
    // Initialize image with non-zero pixels
    if let Some(rgba) = img.as_mut_rgba8() {
        for y in 0..10 {
            for x in 0..10 {
                rgba.put_pixel(x, y, image::Rgba([255, 255, 255, 255]));
            }
        }
    }
    
    masker.mask_rect(&mut img, 2, 2, 6, 6);
    if let Some(rgba) = img.as_rgba8() {
        assert_eq!(rgba.get_pixel(0, 0)[0], 255);
        assert_eq!(rgba.get_pixel(4, 4)[0], 0);
        assert_eq!(rgba.get_pixel(4, 4)[3], 255);
    }

    // Call mask_pii with an image - since tesseract is not found/running, it should safely return early
    let mut test_img = image::DynamicImage::new_rgba8(10, 10);
    masker.mask_pii(&mut test_img);
}

