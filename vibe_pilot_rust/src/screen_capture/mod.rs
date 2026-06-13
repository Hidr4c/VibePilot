//! Screen capture for VibePilot.

use image::DynamicImage;
use serde::{Deserialize, Serialize};

pub mod gdi;
pub mod mock;
pub mod dpi;
pub mod win32_helpers;
pub mod factory;
pub mod pii_masker;
pub mod replay;

#[cfg(test)]
mod tests;

pub use gdi::ScreenCapturer;
pub use factory::ScreenCapturerFactory;
pub use pii_masker::{PiiMasker, OcrPiiMasker, NullPiiMasker, PiiMaskerFactory};
pub use replay::{SessionReplayManager, FileSessionReplayManager, SessionReplayFactory};

#[cfg(target_os = "windows")]
pub use dpi::DpiAwarenessScope;

pub use mock::MockScreenCapturer;

/// Result of a window re-anchoring attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WindowAnchorResult {
    /// Target window was already in the foreground.
    AlreadyFocused,
    /// Target window was found and brought to the foreground.
    Refocused,
    /// Target window could not be found in the window enumeration.
    WindowNotFound,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BoundingBox {
    pub left: i32,
    pub top: i32,
    pub width: i32,
    pub height: i32,
}

impl BoundingBox {
    pub fn new(left: i32, top: i32, width: i32, height: i32) -> Self {
        BoundingBox { left, top, width, height }
    }
    pub fn contains(&self, x: i32, y: i32) -> bool {
        x >= self.left && x < self.left + self.width && y >= self.top && y < self.top + self.height
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenInfo {
    pub title: String,
    pub bbox: BoundingBox,
}

impl ScreenInfo {
    pub fn new(title: String, bbox: BoundingBox) -> Self {
        ScreenInfo { title, bbox }
    }
}

#[derive(Debug, Clone)]
pub struct WindowInfo {
    pub title: String,
    pub hwnd: isize,
    pub visible: bool,
    pub bbox: BoundingBox,
}

/// Checks if the window title matches the target title, supporting flexible application-suffix mapping.
pub fn is_window_title_match(win_title: &str, target_title: &str) -> bool {
    let w_lower = win_title.to_lowercase();
    let t_lower = target_title.to_lowercase();

    // 1. Direct or partial match
    if w_lower.contains(&t_lower) || t_lower.contains(&w_lower) {
        return true;
    }

    // 2. Application suffix matching (e.g. "Google - Brave" vs "Anime-Sama - Brave")
    let separators = &[" - ", " – ", " — ", " | "];
    
    let get_suffix = |t: &str| -> Option<String> {
        for sep in separators {
            if let Some(pos) = t.rfind(sep) {
                let suffix = t[pos + sep.len()..].trim().to_lowercase();
                if !suffix.is_empty() && suffix.len() < 30 {
                    return Some(suffix);
                }
            }
        }
        None
    };

    if let (Some(t_suffix), Some(w_suffix)) = (get_suffix(&t_lower), get_suffix(&w_lower)) {
        if t_suffix == w_suffix {
            return true;
        }
    }

    false
}

pub trait ScreenCapturerTrait: Send + Sync {
    fn refresh_windows(&self);
    fn get_windows(&self) -> Vec<WindowInfo>;
    fn get_monitors(&self) -> Vec<ScreenInfo>;
    fn capture_desktop(&self) -> Option<DynamicImage>;
    fn capture_window_by_title(&self, title: &str) -> Option<DynamicImage>;
    fn capture_bbox(&self, x: i32, y: i32, w: i32, h: i32) -> Option<DynamicImage>;
    fn is_target_visible(&self, target_windows: &[String]) -> bool;
    fn ensure_window_foreground(&self, title: &str) -> WindowAnchorResult;
    fn get_foreground_window_title(&self) -> Option<String>;
}
