use super::*;
use crate::screen_capture::ScreenCapturer;

#[test]
fn test_is_screen_identical() {
    let img1 = DynamicImage::new_rgb8(100, 100);
    let img2 = DynamicImage::new_rgb8(100, 100);
    let capturer: Arc<dyn ScreenCapturerTrait> = Arc::new(ScreenCapturer::new());
    let router = DecisionRouter::new(capturer);
    assert!(router.is_screen_identical(&img1, &img2));
}

#[test]
fn test_with_text_detector() {
    let capturer: Arc<dyn ScreenCapturerTrait> = Arc::new(ScreenCapturer::new());
    let detector = Arc::new(crate::ocr::NullTextDetector::new());
    let _router = DecisionRouter::new(capturer).with_text_detector(detector);
}

#[test]
fn test_detect_spinner_no_change() {
    let img1 = DynamicImage::new_rgb8(128, 128);
    let img2 = DynamicImage::new_rgb8(128, 128);
    let capturer: Arc<dyn ScreenCapturerTrait> = Arc::new(ScreenCapturer::new());
    let router = DecisionRouter::new(capturer);
    assert!(!router.detect_spinner(&img1, &img2));
}

#[test]
fn test_detect_spinner_localized_animation() {
    let img1 = DynamicImage::new_rgb8(128, 128);
    let mut img2 = DynamicImage::new_rgb8(128, 128);
    // Change a small 5x5 square in the center (simulating local spinner/loading activity)
    if let Some(rgb) = img2.as_mut_rgb8() {
        for x in 60..65 {
            for y in 60..65 {
                rgb.put_pixel(x, y, image::Rgb([255, 255, 255]));
            }
        }
    }
    let capturer: Arc<dyn ScreenCapturerTrait> = Arc::new(ScreenCapturer::new());
    let router = DecisionRouter::new(capturer);
    assert!(router.detect_spinner(&img1, &img2));
}

#[test]
fn test_detect_spinner_massive_change() {
    let img1 = DynamicImage::new_rgb8(128, 128);
    let mut img2 = DynamicImage::new_rgb8(128, 128);
    // Change the whole image (not a localized spinner animation)
    if let Some(rgb) = img2.as_mut_rgb8() {
        for x in 0..128 {
            for y in 0..128 {
                rgb.put_pixel(x, y, image::Rgb([255, 255, 255]));
            }
        }
    }
    let capturer: Arc<dyn ScreenCapturerTrait> = Arc::new(ScreenCapturer::new());
    let router = DecisionRouter::new(capturer);
    assert!(!router.detect_spinner(&img1, &img2));
}

#[test]
fn test_route_spinner_wait() {
    let capturer: Arc<dyn ScreenCapturerTrait> = Arc::new(ScreenCapturer::new());
    let router = DecisionRouter::new(capturer);
    let img1 = DynamicImage::new_rgb8(128, 128);
    let mut img2 = DynamicImage::new_rgb8(128, 128);
    if let Some(rgb) = img2.as_mut_rgb8() {
        for x in 60..65 {
            for y in 60..65 {
                rgb.put_pixel(x, y, image::Rgb([255, 255, 255]));
            }
        }
    }
    let ctx = DecisionContext {
        screenshot: &img2,
        last_screenshot: Some(&img1),
        last_action: None,
        current_subtask_description: None,
        target_window_title: None,
        langue: "English",
    };
    let route = router.route(&ctx);
    assert!(matches!(route, DecisionRoute::Fast(resp) if resp.action == "WAIT"));
}

#[test]
fn test_route_identical_screen_wait_reuse() {
    let capturer: Arc<dyn ScreenCapturerTrait> = Arc::new(ScreenCapturer::new());
    let router = DecisionRouter::new(capturer);
    let img1 = DynamicImage::new_rgb8(128, 128);
    let img2 = DynamicImage::new_rgb8(128, 128);
    let step = ActionStep {
        timestamp: "12:00:00".to_string(),
        action_type: "WAIT".to_string(),
        coordinates: None,
        text_typed: None,
        llm_report: "Wait report".to_string(),
        was_repeated: false,
    };
    let ctx = DecisionContext {
        screenshot: &img2,
        last_screenshot: Some(&img1),
        last_action: Some(&step),
        current_subtask_description: None,
        target_window_title: None,
        langue: "English",
    };
    let route = router.route(&ctx);
    assert!(matches!(route, DecisionRoute::Fast(resp) if resp.action == "WAIT"));
}

#[test]
fn test_route_modal_dialog_french() {
    let capturer = Arc::new(crate::screen_capture::MockScreenCapturer::new());
    *capturer.foreground_title.lock().unwrap() = "Error Alert".to_string();
    capturer.windows.lock().unwrap().push(crate::screen_capture::WindowInfo {
        title: "Error Alert".to_string(),
        hwnd: 9999,
        visible: true,
        bbox: crate::screen_capture::BoundingBox::new(200, 200, 400, 300),
    });

    let router = DecisionRouter::new(capturer);
    let img = DynamicImage::new_rgb8(128, 128);
    let ctx = DecisionContext {
        screenshot: &img,
        last_screenshot: None,
        last_action: None,
        current_subtask_description: None,
        target_window_title: Some("My Main Application"),
        langue: "Français",
    };
    let route = router.route(&ctx);
    if let DecisionRoute::Fast(resp) = route {
        assert_eq!(resp.action, "CLICK_AND_TYPE");
        assert!(resp.status_display.contains("Fast : Clic sur bouton OK"));
    } else {
        panic!("Expected DecisionRoute::Fast");
    }
}

#[test]
fn test_route_identical_screen_french_click() {
    let capturer = Arc::new(crate::screen_capture::MockScreenCapturer::new());
    let router = DecisionRouter::new(capturer);
    let img = DynamicImage::new_rgb8(128, 128);
    let step = ActionStep {
        timestamp: "12:00:00".to_string(),
        action_type: "CLICK_AND_TYPE".to_string(),
        coordinates: Some((0.5, 0.5)),
        text_typed: Some("hello".to_string()),
        llm_report: "Click report".to_string(),
        was_repeated: false,
    };
    let ctx = DecisionContext {
        screenshot: &img,
        last_screenshot: Some(&img),
        last_action: Some(&step),
        current_subtask_description: None,
        target_window_title: None,
        langue: "Français",
    };
    let route = router.route(&ctx);
    assert_eq!(route, DecisionRoute::Slow);
}

#[test]
fn test_route_identical_screen_scroll_english() {
    let capturer = Arc::new(crate::screen_capture::MockScreenCapturer::new());
    let router = DecisionRouter::new(capturer);
    let img = DynamicImage::new_rgb8(128, 128);
    let step = ActionStep {
        timestamp: "12:00:00".to_string(),
        action_type: "SCROLL".to_string(),
        coordinates: None,
        text_typed: None,
        llm_report: "Scroll report".to_string(),
        was_repeated: false,
    };
    let ctx = DecisionContext {
        screenshot: &img,
        last_screenshot: Some(&img),
        last_action: Some(&step),
        current_subtask_description: None,
        target_window_title: None,
        langue: "English",
    };
    let route = router.route(&ctx);
    assert!(matches!(route, DecisionRoute::Fast(resp) if resp.action == "SCROLL"));
}

struct MockTextDetector {
    pos: Option<(u32, u32)>,
}

impl crate::ocr::TextDetector for MockTextDetector {
    fn find_text_center(&self, _image: &DynamicImage, _target_words: &[&str]) -> Option<(u32, u32)> {
        self.pos
    }
}

#[test]
fn test_route_modal_dialog_with_custom_text_detector() {
    let capturer = Arc::new(crate::screen_capture::MockScreenCapturer::new());
    *capturer.foreground_title.lock().unwrap() = "Alert Window".to_string();
    capturer.windows.lock().unwrap().push(crate::screen_capture::WindowInfo {
        title: "Alert Window".to_string(),
        hwnd: 1234,
        visible: true,
        bbox: crate::screen_capture::BoundingBox::new(100, 100, 300, 200),
    });

    let detector = Arc::new(MockTextDetector { pos: Some((150, 120)) });
    let router = DecisionRouter::new(capturer).with_text_detector(detector);
    let img = DynamicImage::new_rgb8(100, 100);
    let ctx = DecisionContext {
        screenshot: &img,
        last_screenshot: None,
        last_action: None,
        current_subtask_description: None,
        target_window_title: Some("My Main Application"),
        langue: "English",
    };
    let route = router.route(&ctx);
    if let DecisionRoute::Fast(resp) = route {
        assert_eq!(resp.action, "CLICK_AND_TYPE");
        assert!(resp.relative_click_position[0] > 0.0);
    } else {
        panic!("Expected DecisionRoute::Fast");
    }
}

#[test]
fn test_route_modal_dialog_target_title_match() {
    let capturer = Arc::new(crate::screen_capture::MockScreenCapturer::new());
    *capturer.foreground_title.lock().unwrap() = "My Main Application".to_string();
    capturer.windows.lock().unwrap().push(crate::screen_capture::WindowInfo {
        title: "My Main Application".to_string(),
        hwnd: 1234,
        visible: true,
        bbox: crate::screen_capture::BoundingBox::new(100, 100, 300, 200),
    });

    let router = DecisionRouter::new(capturer);
    let img = DynamicImage::new_rgb8(100, 100);
    let ctx = DecisionContext {
        screenshot: &img,
        last_screenshot: None,
        last_action: None,
        current_subtask_description: None,
        target_window_title: Some("My Main Application"),
        langue: "English",
    };
    let route = router.route(&ctx);
    assert_eq!(route, DecisionRoute::Slow);
}

#[test]
fn test_is_screen_identical_different() {
    let img1 = DynamicImage::new_rgb8(10, 10);
    let mut img2 = DynamicImage::new_rgb8(10, 10);
    if let Some(rgb) = img2.as_mut_rgb8() {
        for x in 0..10 {
            for y in 0..10 {
                rgb.put_pixel(x, y, image::Rgb([255, 255, 255]));
            }
        }
    }
    let capturer: Arc<dyn ScreenCapturerTrait> = Arc::new(crate::screen_capture::ScreenCapturer::new());
    let router = DecisionRouter::new(capturer);
    assert!(!router.is_screen_identical(&img1, &img2));
}

