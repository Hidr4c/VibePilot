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

#[test]
fn test_file_session_replay_manager() {
    let temp_dir = std::env::temp_dir().join("vibepilot_test_replay");
    let _ = std::fs::remove_dir_all(&temp_dir);
    let _ = std::fs::create_dir_all(&temp_dir);

    let manager = FileSessionReplayManager::new(temp_dir.clone());

    // Record 12 mock frames to cover capacity limit and pop_front eviction
    for _ in 0..12 {
        let img = image::DynamicImage::new_rgba8(10, 10);
        manager.record_frame(&img);
    }

    // Test save_replay
    let save_res = manager.save_replay("Test Error");
    assert!(save_res.is_ok());
    let report_dir = save_res.unwrap();
    assert!(report_dir.exists());
    assert!(report_dir.join("info.txt").exists());
    assert!(report_dir.join("frame_01.png").exists());
    assert!(report_dir.join("replay.gif").exists());

    // Test export_gif
    let gif_path = temp_dir.join("test_export.gif");
    let export_res = manager.export_gif(&gif_path);
    assert!(export_res.is_ok());
    assert!(gif_path.exists());

    // Test export_full_session_gif
    let full_gif_path = temp_dir.join("test_full_export.gif");
    let full_export_res = manager.export_full_session_gif(&full_gif_path);
    assert!(full_export_res.is_ok());
    assert!(full_gif_path.exists());

    // Test export_full_session_archive
    let archive_dir = temp_dir.join("archive");
    let logs = vec!["log line 1".to_string(), "log line 2".to_string()];
    let report = "# Final Report\nSuccess.";
    let archive_res = manager.export_full_session_archive(&archive_dir, &logs, report);
    assert!(archive_res.is_ok());
    assert!(archive_dir.join("logs.txt").exists());
    assert!(archive_dir.join("report.md").exists());
    assert!(archive_dir.join("replay.gif").exists());
    assert!(archive_dir.join("frames/frame_001.png").exists());

    // Test clear_full_session
    manager.clear_full_session();
    let empty_gif_path = temp_dir.join("empty.gif");
    let empty_export_res = manager.export_full_session_gif(&empty_gif_path);
    assert!(empty_export_res.is_err()); // Should error since buffer is empty

    // Test export_full_session_archive when buffer is empty (should not write gif but write other files)
    let empty_archive_dir = temp_dir.join("empty_archive");
    let archive_res_empty = manager.export_full_session_archive(&empty_archive_dir, &logs, report);
    assert!(archive_res_empty.is_ok());
    assert!(empty_archive_dir.join("logs.txt").exists());
    assert!(empty_archive_dir.join("report.md").exists());
    assert!(!empty_archive_dir.join("replay.gif").exists());

    // Test export errors by passing invalid paths
    // 1. Passing a directory path as the gif file path (should fail on File::create)
    let invalid_gif_path = temp_dir.clone();
    let err_gif = manager.export_gif(&invalid_gif_path);
    assert!(err_gif.is_err());

    // 2. Passing a directory path as the full session gif file path (should fail on File::create)
    // We need to record a frame first so it doesn't fail on empty check
    let img2 = image::DynamicImage::new_rgba8(10, 10);
    manager.record_frame(&img2);
    let err_full_gif = manager.export_full_session_gif(&invalid_gif_path);
    assert!(err_full_gif.is_err());

    // 3. Passing an existing file path as the archive directory (should fail on create_dir_all)
    let invalid_archive_dir = temp_dir.join("test_export.gif"); // existing file
    let err_archive = manager.export_full_session_archive(&invalid_archive_dir, &logs, report);
    assert!(err_archive.is_err());

    // 4. Trigger GIF creation error in export_full_session_archive when buffer is not empty
    let err_archive_dir = temp_dir.join("err_archive");
    std::fs::create_dir_all(&err_archive_dir).unwrap();
    let bad_gif_dir = err_archive_dir.join("replay.gif");
    std::fs::create_dir_all(&bad_gif_dir).unwrap();
    let err_archive_gif = manager.export_full_session_archive(&err_archive_dir, &logs, report);
    assert!(err_archive_gif.is_err());

    // 5. Trigger directory creation error in save_replay by pointing it to a file path
    let file_path = temp_dir.join("save_replay_file.txt");
    std::fs::write(&file_path, "hello").unwrap();
    let bad_manager = FileSessionReplayManager::new(file_path);
    let save_err = bad_manager.save_replay("Test Error");
    assert!(save_err.is_err());

    // 6. Test PiiMaskerFactory::create
    let factory_masker = crate::screen_capture::pii_masker::PiiMaskerFactory::create("eng".to_string());
    let mut test_img = image::DynamicImage::new_rgba8(10, 10);
    factory_masker.mask_pii(&mut test_img);

    let _ = std::fs::remove_dir_all(&temp_dir);
}

