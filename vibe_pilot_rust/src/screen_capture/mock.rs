use image::DynamicImage;
use super::{WindowInfo, ScreenInfo, BoundingBox, WindowAnchorResult, ScreenCapturerTrait};

pub struct MockScreenCapturer {
    pub windows: std::sync::Mutex<Vec<WindowInfo>>,
    pub monitors: Vec<ScreenInfo>,
    pub foreground_title: std::sync::Mutex<String>,
    pub active_track: std::sync::Mutex<Option<(isize, String)>>,
}

impl MockScreenCapturer {
    pub fn new() -> Self {
        Self {
            windows: std::sync::Mutex::new(Vec::new()),
            monitors: vec![ScreenInfo::new("Screen 1".to_string(), BoundingBox::new(0, 0, 1920, 1080))],
            foreground_title: std::sync::Mutex::new("Desktop".to_string()),
            active_track: std::sync::Mutex::new(None),
        }
    }
}

impl Default for MockScreenCapturer {
    fn default() -> Self {
        Self::new()
    }
}

impl ScreenCapturerTrait for MockScreenCapturer {
    fn refresh_windows(&self) {}
    
    fn get_windows(&self) -> Vec<WindowInfo> {
        self.windows.lock().expect("MockScreenCapturer windows mutex poisoned").clone()
    }
    
    fn get_monitors(&self) -> Vec<ScreenInfo> {
        self.monitors.clone()
    }
    
    fn capture_desktop(&self) -> Option<DynamicImage> {
        Some(DynamicImage::ImageRgba8(image::ImageBuffer::new(100, 100)))
    }
    
    fn capture_window_by_title(&self, _title: &str) -> Option<DynamicImage> {
        Some(DynamicImage::ImageRgba8(image::ImageBuffer::new(100, 100)))
    }
    
    fn capture_bbox(&self, _x: i32, _y: i32, _w: i32, _h: i32) -> Option<DynamicImage> {
        Some(DynamicImage::ImageRgba8(image::ImageBuffer::new(100, 100)))
    }
    
    fn is_target_visible(&self, _target_windows: &[String]) -> bool {
        true
    }
    
    fn ensure_window_foreground(&self, title: &str) -> WindowAnchorResult {
        let windows = self.windows.lock().expect("MockScreenCapturer windows mutex poisoned");
        if windows.iter().any(|w| w.title == title) {
            WindowAnchorResult::AlreadyFocused
        } else {
            WindowAnchorResult::WindowNotFound
        }
    }
    
    fn get_foreground_window_title(&self) -> Option<String> {
        Some(self.foreground_title.lock().expect("MockScreenCapturer foreground_title mutex poisoned").clone())
    }
}
