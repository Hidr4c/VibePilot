//! Peripheral controller for VibePilot.
//!
//! Handles physical actions: mouse movement, click, scroll, keyboard input.
//! Uses rdev for input simulation and Windows API for clipboard.

use std::sync::Arc;

use crate::screen_capture::{ScreenCapturer, ScreenInfo};

/// Action payload from the LLM response.
#[derive(Debug, Clone)]
pub struct ActionPayload {
    pub action: String,
    pub relative_click_position: Vec<f64>,
    pub text_to_type: String,
    pub scroll_value: i32,
    pub wait_seconds: i32,
}

/// Screen offset info for coordinate translation.
#[derive(Debug, Clone)]
pub struct ScreenOffset {
    pub titre: String,
    pub left: i32,
    pub top: i32,
    pub width: i32,
    pub height: i32,
    pub y_offset: i32,
}

impl ScreenOffset {
    pub fn from_screen_info(screen: ScreenInfo) -> Self {
        ScreenOffset {
            titre: screen.title,
            left: screen.bbox.left,
            top: screen.bbox.top,
            width: screen.bbox.width,
            height: screen.bbox.height,
            y_offset: screen.bbox.top,
        }
    }
}

/// Mouse/keyboard peripheral controller.
pub struct PeripheralController {
    capturer: Arc<ScreenCapturer>,
    set_clipboard_fn: Option<Box<dyn Fn(&str) + Send + Sync>>,
}

impl PartialEq for PeripheralController {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.capturer, &other.capturer)
    }
}

impl PeripheralController {
    pub fn new(capturer: Arc<ScreenCapturer>) -> Self {
        Self { capturer, set_clipboard_fn: None }
    }

    pub fn with_clipboard<F>(mut self, callback: F) -> Self
    where
        F: Fn(&str) + Send + Sync + 'static,
    {
        self.set_clipboard_fn = Some(Box::new(callback));
        self
    }

    /// Execute a physical action based on the LLM decision.
    pub fn execute_action(&self, payload: &ActionPayload, offsets: &[ScreenOffset], verifier_placement_souris: bool) {
        use rdev::{simulate, EventType, Key};
        #[cfg(not(target_os = "windows"))]
        use windows::Win32::UI::WindowsAndMessaging::SetCursorPos;

        let x_rel = payload.relative_click_position.get(0).copied().unwrap_or(0.5);
        let y_rel = payload.relative_click_position.get(1).copied().unwrap_or(0.5);

        let (abs_x, abs_y) = self.compute_absolute_coordinates(x_rel, y_rel, offsets);

        // Move cursor to position using SendInput on Windows for absolute virtual desktop mapping
        #[cfg(target_os = "windows")]
        {
            win32_move_mouse_absolute(abs_x, abs_y);
        }
        #[cfg(not(target_os = "windows"))]
        unsafe {
            let _ = SetCursorPos(abs_x, abs_y);
        }
        
        if verifier_placement_souris {
            std::thread::sleep(std::time::Duration::from_millis(1500));
        } else {
            std::thread::sleep(std::time::Duration::from_millis(100));
        }

        // Re-align cursor to target coordinates in case user moved the mouse during the delay
        #[cfg(target_os = "windows")]
        {
            win32_move_mouse_absolute(abs_x, abs_y);
        }
        #[cfg(not(target_os = "windows"))]
        unsafe {
            let _ = SetCursorPos(abs_x, abs_y);
        }
        std::thread::sleep(std::time::Duration::from_millis(50));

        match payload.action.as_str() {
            "CLICK_AND_TYPE" => {
                // Simulate left click
                #[cfg(target_os = "windows")]
                {
                    win32_click_current_position();
                }
                #[cfg(not(target_os = "windows"))]
                {
                    let _ = simulate(&EventType::ButtonPress(rdev::Button::Left));
                    std::thread::sleep(std::time::Duration::from_millis(50));
                    let _ = simulate(&EventType::ButtonRelease(rdev::Button::Left));
                }
                std::thread::sleep(std::time::Duration::from_millis(100));

                // Paste text via clipboard + Ctrl+V
                if !payload.text_to_type.is_empty() {
                    self.set_clipboard(&payload.text_to_type);
                    std::thread::sleep(std::time::Duration::from_millis(200));
                    
                    // Ctrl+V using rdev modifiers
                    let _ = simulate(&EventType::KeyPress(Key::ControlLeft));
                    std::thread::sleep(std::time::Duration::from_millis(50));
                    let _ = simulate(&EventType::KeyPress(Key::KeyV));
                    std::thread::sleep(std::time::Duration::from_millis(50));
                    let _ = simulate(&EventType::KeyRelease(Key::KeyV));
                    std::thread::sleep(std::time::Duration::from_millis(50));
                    let _ = simulate(&EventType::KeyRelease(Key::ControlLeft));
                    std::thread::sleep(std::time::Duration::from_millis(300));
                }
            },
            "SCROLL" => {
                let scroll_amount = payload.scroll_value.unsigned_abs();
                for _ in 0..scroll_amount {
                    if payload.scroll_value > 0 {
                        let _ = simulate(&EventType::Wheel { delta_x: 0, delta_y: 1 });
                    } else {
                        let _ = simulate(&EventType::Wheel { delta_x: 0, delta_y: -1 });
                    }
                }
            },
            "WAIT" => {},
            _ => {
                eprintln!("Unknown action type: {}", payload.action);
            }
        }
    }

    /// Compute absolute screen coordinates from relative position.
    pub fn compute_absolute_coordinates(
        &self,
        x_rel: f64,
        y_rel: f64,
        offsets: &[ScreenOffset],
    ) -> (i32, i32) {
        if offsets.is_empty() {
            return (0, 0);
        }

        let mut x_rel = x_rel;
        let mut y_rel = y_rel;

        // Auto-detect coordinate space (0..1 vs 0..1000 vs pixel space)
        if x_rel > 1.0 || y_rel > 1.0 {
            if x_rel <= 1000.0 && y_rel <= 1000.0 && x_rel > 1.0 && y_rel > 1.0 {
                x_rel /= 1000.0;
                y_rel /= 1000.0;
            } else {
                if x_rel > 1.0 {
                    let target_w = offsets.get(0).map(|o| o.width).unwrap_or(1920) as f64;
                    x_rel /= target_w;
                }
                if y_rel > 1.0 {
                    let target_h = offsets.get(0).map(|o| o.height).unwrap_or(1080) as f64;
                    y_rel /= target_h;
                }
            }
        }

        if offsets.len() == 1 {
            let target = &offsets[0];
            let abs_x = target.left + (target.width as f64 * x_rel) as i32;
            let abs_y = target.top + (target.height as f64 * y_rel) as i32;
            return (abs_x, abs_y);
        }

        let is_desktop = offsets.len() == 1
            && self.is_desktop_title(&offsets[0].titre);

        if is_desktop {
            let monitors = self.capturer.get_monitors();
            if monitors.is_empty() {
                return (0, 0);
            }

            let min_x = monitors.iter().map(|m| m.bbox.left).min().unwrap_or(0);
            let min_y = monitors.iter().map(|m| m.bbox.top).min().unwrap_or(0);
            let max_x = monitors.iter().map(|m| m.bbox.left + m.bbox.width).max().unwrap_or(0);
            let max_y = monitors.iter().map(|m| m.bbox.top + m.bbox.height).max().unwrap_or(0);

            let virtual_width = max_x - min_x;
            let virtual_height = max_y - min_y;

            if virtual_width <= 0 || virtual_height <= 0 {
                return (0, 0);
            }

            let abs_x = min_x + (virtual_width as f64 * x_rel) as i32;
            let abs_y = min_y + (virtual_height as f64 * y_rel) as i32;
            (abs_x, abs_y)
        } else {
            let total_height: i32 = offsets.iter().map(|o| o.height + 25).sum();
            let y_phys = (y_rel * total_height as f64) as i32;

            let mut target = &offsets[0];
            for offset in offsets {
                if offset.y_offset <= y_phys && y_phys < offset.y_offset + offset.height {
                    target = offset;
                    break;
                }
            }

            let y_local = y_phys - target.y_offset;
            let y_local_rel = y_local as f64 / target.height as f64;

            let abs_x = target.left + (target.width as f64 * x_rel) as i32;
            let abs_y = target.top + (target.height as f64 * y_local_rel) as i32;
            (abs_x, abs_y)
        }
    }

    /// Check if title refers to the desktop.
    pub fn is_desktop_title(&self, title: &str) -> bool {
        let t = title.to_lowercase();
        let t = t
            .replace('é', "e")
            .replace('è', "e")
            .replace('ê', "e")
            .replace('ë', "e");
        t.contains("bureau")
            || t.contains("desktop")
            || t.contains("ecrans")
            || t.contains("screens")
            || t.contains("ecran")
            || t.contains("all_screens")
            || t.contains("tous les")
            || t.contains("all screens")
    }

    #[allow(dead_code)]
    pub(crate) fn get_screen_bounds(&self) -> (i32, i32, i32, i32) {
        #[cfg(target_os = "windows")]
        {
            use windows::Win32::UI::WindowsAndMessaging::{
                GetSystemMetrics, SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN,
                SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_CXSCREEN, SM_CYSCREEN
            };
            unsafe {
                let left = GetSystemMetrics(SM_XVIRTUALSCREEN);
                let top = GetSystemMetrics(SM_YVIRTUALSCREEN);
                let width = GetSystemMetrics(SM_CXVIRTUALSCREEN);
                let height = GetSystemMetrics(SM_CYVIRTUALSCREEN);
                if width == 0 || height == 0 {
                    let cx = GetSystemMetrics(SM_CXSCREEN);
                    let cy = GetSystemMetrics(SM_CYSCREEN);
                    (0, 0, cx, cy)
                } else {
                    (left, top, left + width, top + height)
                }
            }
        }
        #[cfg(not(target_os = "windows"))]
        {
            (0, 0, 1920, 1080)
        }
    }

    fn set_clipboard(&self, text: &str) {
        // Use thread-safe callback if provided (Tkinter clipboard)
        if let Some(ref callback) = self.set_clipboard_fn {
            callback(text);
            return;
        }

        // Fallback: use arboard clipboard
        if let Ok(mut clip) = arboard::Clipboard::new() {
            let _ = clip.set_text(text);
        }
    }
}

#[cfg(target_os = "windows")]
fn win32_move_mouse_absolute(x: i32, y: i32) {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        SendInput, INPUT, INPUT_MOUSE, MOUSEEVENTF_ABSOLUTE, MOUSEEVENTF_MOVE, MOUSEEVENTF_VIRTUALDESK, MOUSEINPUT,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        GetSystemMetrics, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN
    };

    unsafe {
        let v_left = GetSystemMetrics(SM_XVIRTUALSCREEN);
        let v_top = GetSystemMetrics(SM_YVIRTUALSCREEN);
        let v_width = GetSystemMetrics(SM_CXVIRTUALSCREEN);
        let v_height = GetSystemMetrics(SM_CYVIRTUALSCREEN);

        if v_width <= 1 || v_height <= 1 {
            return;
        }

        let norm_x = (((x - v_left) as f64 * 65535.0) / (v_width - 1) as f64) as i32;
        let norm_y = (((y - v_top) as f64 * 65535.0) / (v_height - 1) as f64) as i32;

        let mut input = INPUT::default();
        input.r#type = INPUT_MOUSE;
        input.Anonymous.mi = MOUSEINPUT {
            dx: norm_x,
            dy: norm_y,
            mouseData: 0,
            dwFlags: MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_VIRTUALDESK,
            time: 0,
            dwExtraInfo: 0,
        };

        SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
    }
}

#[cfg(target_os = "windows")]
fn win32_click_current_position() {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        SendInput, INPUT, INPUT_MOUSE, MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP, MOUSEINPUT,
    };

    unsafe {
        let mut inputs = [INPUT::default(), INPUT::default()];

        inputs[0].r#type = INPUT_MOUSE;
        inputs[0].Anonymous.mi = MOUSEINPUT {
            dx: 0,
            dy: 0,
            mouseData: 0,
            dwFlags: MOUSEEVENTF_LEFTDOWN,
            time: 0,
            dwExtraInfo: 0,
        };

        inputs[1].r#type = INPUT_MOUSE;
        inputs[1].Anonymous.mi = MOUSEINPUT {
            dx: 0,
            dy: 0,
            mouseData: 0,
            dwFlags: MOUSEEVENTF_LEFTUP,
            time: 0,
            dwExtraInfo: 0,
        };

        SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
    }
}
