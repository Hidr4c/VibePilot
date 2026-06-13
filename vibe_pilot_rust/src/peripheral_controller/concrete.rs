use std::sync::Arc;
use super::payload::{ActionPayload, ScreenOffset};
use crate::screen_capture::ScreenCapturerTrait;
use super::{
    MouseInput, KeyboardInput, ScrollInput, ClipboardInput, CoordinateMapping, PeripheralInput
};

#[cfg(target_os = "windows")]
use super::native_win::{
    win32_move_mouse_absolute,
    win32_click_current_position,
    win32_right_click_current_position,
    win32_middle_click_current_position,
};

/// Concrete implementation of PeripheralController.
pub struct PeripheralController {
    pub(crate) capturer: Arc<dyn ScreenCapturerTrait>,
    #[allow(clippy::type_complexity)]
    set_clipboard_fn: Option<Box<dyn Fn(&str) + Send + Sync>>,
}

impl std::fmt::Debug for PeripheralController {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PeripheralController").finish()
    }
}

impl PartialEq for PeripheralController {
    fn eq(&self, other: &Self) -> bool {
        Arc::as_ptr(&self.capturer) as *const () == Arc::as_ptr(&other.capturer) as *const ()
    }
}

impl PeripheralController {
    pub fn new(capturer: Arc<dyn ScreenCapturerTrait>) -> Self {
        Self { capturer, set_clipboard_fn: None }
    }

    pub fn with_clipboard<F>(mut self, callback: F) -> Self
    where
        F: Fn(&str) + Send + Sync + 'static,
    {
        self.set_clipboard_fn = Some(Box::new(callback));
        self
    }
}

impl MouseInput for PeripheralController {
    fn right_click(&self) -> Result<(), String> {
        if cfg!(test) {
            return Ok(());
        }
        #[cfg(target_os = "windows")]
        {
            win32_right_click_current_position();
        }
        #[cfg(not(target_os = "windows"))]
        {
            use rdev::{simulate, EventType, Button};
            let _ = simulate(&EventType::ButtonPress(Button::Right));
            std::thread::sleep(std::time::Duration::from_millis(50));
            let _ = simulate(&EventType::ButtonRelease(Button::Right));
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
        Ok(())
    }

    fn middle_click(&self) -> Result<(), String> {
        if cfg!(test) {
            return Ok(());
        }
        #[cfg(target_os = "windows")]
        {
            win32_middle_click_current_position();
        }
        #[cfg(not(target_os = "windows"))]
        {
            use rdev::{simulate, EventType, Button};
            let _ = simulate(&EventType::ButtonPress(Button::Middle));
            std::thread::sleep(std::time::Duration::from_millis(50));
            let _ = simulate(&EventType::ButtonRelease(Button::Middle));
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
        Ok(())
    }

    fn double_click(&self) -> Result<(), String> {
        if cfg!(test) {
            return Ok(());
        }
        #[cfg(target_os = "windows")]
        {
            win32_click_current_position();
            std::thread::sleep(std::time::Duration::from_millis(80));
            win32_click_current_position();
        }
        #[cfg(not(target_os = "windows"))]
        {
            use rdev::{simulate, EventType, Button};
            let _ = simulate(&EventType::ButtonPress(Button::Left));
            std::thread::sleep(std::time::Duration::from_millis(30));
            let _ = simulate(&EventType::ButtonRelease(Button::Left));
            std::thread::sleep(std::time::Duration::from_millis(80));
            let _ = simulate(&EventType::ButtonPress(Button::Left));
            std::thread::sleep(std::time::Duration::from_millis(30));
            let _ = simulate(&EventType::ButtonRelease(Button::Left));
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
        Ok(())
    }

    fn drag_and_drop(&self, from: (i32, i32), to: (i32, i32), button: rdev::Button) -> Result<(), String> {
        if cfg!(test) {
            return Ok(());
        }
        #[cfg(target_os = "windows")]
        {
            win32_move_mouse_absolute(from.0, from.1);
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = (from, to, button);
        }
        std::thread::sleep(std::time::Duration::from_millis(100));

        let rdev_btn = button;
        
        use rdev::{simulate, EventType};
        let _ = simulate(&EventType::ButtonPress(rdev_btn));
        std::thread::sleep(std::time::Duration::from_millis(100));

        #[cfg(target_os = "windows")]
        {
            let steps = 10;
            for i in 1..=steps {
                let curr_x = from.0 + ((to.0 - from.0) * i / steps);
                let curr_y = from.1 + ((to.1 - from.1) * i / steps);
                win32_move_mouse_absolute(curr_x, curr_y);
                std::thread::sleep(std::time::Duration::from_millis(15));
            }
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = (to);
        }
        std::thread::sleep(std::time::Duration::from_millis(100));

        let _ = simulate(&EventType::ButtonRelease(rdev_btn));
        std::thread::sleep(std::time::Duration::from_millis(100));
        Ok(())
    }

    fn mouse_move_relative(&self, dx: i32, dy: i32) -> Result<(), String> {
        if cfg!(test) {
            return Ok(());
        }
        #[cfg(target_os = "windows")]
        {
            use windows::Win32::Foundation::POINT;
            use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;
            use crate::screen_capture::DpiAwarenessScope;
            let mut pt = POINT::default();
            unsafe {
                let _scope = DpiAwarenessScope::enter_per_monitor_v2();
                let _ = GetCursorPos(&mut pt);
            }
            win32_move_mouse_absolute(pt.x + dx, pt.y + dy);
        }
        #[cfg(not(target_os = "windows"))]
        {
            use rdev::{simulate, EventType};
            let _ = simulate(&EventType::MouseMove { x: dx as f64, y: dy as f64 });
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
        Ok(())
    }
}

impl KeyboardInput for PeripheralController {
    fn key_combo(&self, keys: &[rdev::Key]) -> Result<(), String> {
        if cfg!(test) {
            return Ok(());
        }
        use rdev::{simulate, EventType};
        for key in keys {
            let _ = simulate(&EventType::KeyPress(*key));
            std::thread::sleep(std::time::Duration::from_millis(30));
        }
        std::thread::sleep(std::time::Duration::from_millis(30));
        for key in keys.iter().rev() {
            let _ = simulate(&EventType::KeyRelease(*key));
            std::thread::sleep(std::time::Duration::from_millis(30));
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
        Ok(())
    }
}

impl ScrollInput for PeripheralController {
    fn scroll_vertical(&self, amount: i32) -> Result<(), String> {
        if cfg!(test) {
            return Ok(());
        }
        use rdev::{simulate, EventType};
        let scroll_amount = amount.unsigned_abs();
        for _ in 0..scroll_amount {
            if amount > 0 {
                let _ = simulate(&EventType::Wheel { delta_x: 0, delta_y: 1 });
            } else {
                let _ = simulate(&EventType::Wheel { delta_x: 0, delta_y: -1 });
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        Ok(())
    }

    fn scroll_horizontal(&self, amount: i32) -> Result<(), String> {
        if cfg!(test) {
            return Ok(());
        }
        use rdev::{simulate, EventType};
        let scroll_amount = amount.unsigned_abs();
        for _ in 0..scroll_amount {
            if amount > 0 {
                let _ = simulate(&EventType::Wheel { delta_x: 1, delta_y: 0 });
            } else {
                let _ = simulate(&EventType::Wheel { delta_x: -1, delta_y: 0 });
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        Ok(())
    }

    fn smooth_scroll_vertical(&self, amount: i32, steps: u32, duration_ms: u64) -> Result<(), String> {
        if cfg!(test) {
            return Ok(());
        }
        if steps == 0 { return Ok(()); }
        let step_amount = amount as f64 / steps as f64;
        let delay = std::time::Duration::from_millis(duration_ms / steps as u64);
        let mut accumulated = 0.0f64;
        for _ in 0..steps {
            accumulated += step_amount;
            let delta = accumulated.round() as i32;
            if delta != 0 {
                self.scroll_vertical(delta)?;
                accumulated -= delta as f64;
            }
            std::thread::sleep(delay);
        }
        Ok(())
    }

    fn quick_scroll_vertical(&self, direction: &str, intensity: &str) -> Result<(), String> {
        if cfg!(test) {
            return Ok(());
        }
        let ticks = match intensity.to_lowercase().as_str() {
            "low" => 5,
            "medium" => 15,
            "high" => 30,
            _ => 15,
        };
        let val = if direction.to_lowercase() == "up" { 1 } else { -1 };
        for _ in 0..ticks {
            self.scroll_vertical(val)?;
            std::thread::sleep(std::time::Duration::from_millis(15));
        }
        Ok(())
    }
}

impl ClipboardInput for PeripheralController {
    fn set_clipboard_text(&self, text: &str) -> Result<(), String> {
        if let Some(ref callback) = self.set_clipboard_fn {
            callback(text);
            return Ok(());
        }
        if cfg!(test) {
            return Ok(());
        }
        let mut clip = arboard::Clipboard::new().map_err(|e| format!("Clipboard error: {}", e))?;
        clip.set_text(text).map_err(|e| format!("Clipboard write error: {}", e))
    }

    fn get_clipboard_text(&self) -> Result<String, String> {
        if cfg!(test) {
            return Ok("mock_clipboard".to_string());
        }
        let mut clip = arboard::Clipboard::new().map_err(|e| format!("Clipboard error: {}", e))?;
        clip.get_text().map_err(|e| format!("Clipboard read error: {}", e))
    }

    fn set_clipboard_html(&self, html: &str) -> Result<(), String> {
        if cfg!(test) {
            return Ok(());
        }
        let mut clip = arboard::Clipboard::new().map_err(|e| format!("Clipboard error: {}", e))?;
        clip.set_html(html, Some(html)).map_err(|e| format!("Clipboard HTML write error: {}", e))
    }

    fn get_clipboard_html(&self) -> Result<String, String> {
        if cfg!(test) {
            return Ok("<html><body>mock_clipboard</body></html>".to_string());
        }
        Err("HTML clipboard read not available".to_string())
    }

    fn set_clipboard_image(&self, img: &image::DynamicImage) -> Result<(), String> {
        if cfg!(test) {
            return Ok(());
        }
        let mut clip = arboard::Clipboard::new().map_err(|e| format!("Clipboard error: {}", e))?;
        let rgba = img.to_rgba8();
        let (w, h) = rgba.dimensions();
        let img_data = arboard::ImageData {
            width: w as usize,
            height: h as usize,
            bytes: std::borrow::Cow::Borrowed(rgba.as_raw()),
        };
        clip.set_image(img_data).map_err(|e| format!("Clipboard image write error: {}", e))
    }

    fn get_clipboard_image(&self) -> Result<image::DynamicImage, String> {
        if cfg!(test) {
            return Ok(image::DynamicImage::ImageRgba8(image::ImageBuffer::new(10, 10)));
        }
        let mut clip = arboard::Clipboard::new().map_err(|e| format!("Clipboard error: {}", e))?;
        let img_data = clip.get_image().map_err(|e| format!("Clipboard image read error: {}", e))?;
        let width = img_data.width as u32;
        let height = img_data.height as u32;
        let raw_bytes = img_data.bytes.into_owned();
        let buffer = image::ImageBuffer::from_raw(width, height, raw_bytes)
            .ok_or_else(|| "Failed to construct ImageBuffer from raw clipboard bytes".to_string())?;
        Ok(image::DynamicImage::ImageRgba8(buffer))
    }

    fn copy_selected(&self) -> Result<(), String> {
        if cfg!(test) {
            return Ok(());
        }
        use rdev::{simulate, EventType, Key};
        let _ = simulate(&EventType::KeyPress(Key::ControlLeft));
        std::thread::sleep(std::time::Duration::from_millis(50));
        let _ = simulate(&EventType::KeyPress(Key::KeyC));
        std::thread::sleep(std::time::Duration::from_millis(50));
        let _ = simulate(&EventType::KeyRelease(Key::KeyC));
        std::thread::sleep(std::time::Duration::from_millis(50));
        let _ = simulate(&EventType::KeyRelease(Key::ControlLeft));
        std::thread::sleep(std::time::Duration::from_millis(150));
        Ok(())
    }

    fn paste(&self) -> Result<(), String> {
        if cfg!(test) {
            return Ok(());
        }
        use rdev::{simulate, EventType, Key};
        let _ = simulate(&EventType::KeyPress(Key::ControlLeft));
        std::thread::sleep(std::time::Duration::from_millis(50));
        let _ = simulate(&EventType::KeyPress(Key::KeyV));
        std::thread::sleep(std::time::Duration::from_millis(50));
        let _ = simulate(&EventType::KeyRelease(Key::KeyV));
        std::thread::sleep(std::time::Duration::from_millis(50));
        let _ = simulate(&EventType::KeyRelease(Key::ControlLeft));
        std::thread::sleep(std::time::Duration::from_millis(150));
        Ok(())
    }

    fn cut_selected(&self) -> Result<(), String> {
        if cfg!(test) {
            return Ok(());
        }
        use rdev::{simulate, EventType, Key};
        let _ = simulate(&EventType::KeyPress(Key::ControlLeft));
        std::thread::sleep(std::time::Duration::from_millis(50));
        let _ = simulate(&EventType::KeyPress(Key::KeyX));
        std::thread::sleep(std::time::Duration::from_millis(50));
        let _ = simulate(&EventType::KeyRelease(Key::KeyX));
        std::thread::sleep(std::time::Duration::from_millis(50));
        let _ = simulate(&EventType::KeyRelease(Key::ControlLeft));
        std::thread::sleep(std::time::Duration::from_millis(150));
        Ok(())
    }

    fn select_all(&self) -> Result<(), String> {
        if cfg!(test) {
            return Ok(());
        }
        use rdev::{simulate, EventType, Key};
        let _ = simulate(&EventType::KeyPress(Key::ControlLeft));
        std::thread::sleep(std::time::Duration::from_millis(50));
        let _ = simulate(&EventType::KeyPress(Key::KeyA));
        std::thread::sleep(std::time::Duration::from_millis(50));
        let _ = simulate(&EventType::KeyRelease(Key::KeyA));
        std::thread::sleep(std::time::Duration::from_millis(50));
        let _ = simulate(&EventType::KeyRelease(Key::ControlLeft));
        std::thread::sleep(std::time::Duration::from_millis(150));
        Ok(())
    }
}



impl PeripheralInput for PeripheralController {
    fn execute_action(&self, payload: &ActionPayload, offsets: &[ScreenOffset], verifier_placement_souris: bool) {
        if cfg!(test) {
            return;
        }
        use rdev::{simulate, EventType, Key};

        let x_rel = payload.relative_click_position.first().copied().unwrap_or(0.5);
        let y_rel = payload.relative_click_position.get(1).copied().unwrap_or(0.5);

        let (abs_x, abs_y) = self.compute_absolute_coordinates(x_rel, y_rel, offsets);

        #[cfg(target_os = "windows")]
        {
            win32_move_mouse_absolute(abs_x, abs_y);
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = (abs_x, abs_y);
        }
        
        if verifier_placement_souris {
            std::thread::sleep(std::time::Duration::from_millis(1500));
        } else {
            std::thread::sleep(std::time::Duration::from_millis(100));
        }

        #[cfg(target_os = "windows")]
        {
            win32_move_mouse_absolute(abs_x, abs_y);
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = (abs_x, abs_y);
        }
        std::thread::sleep(std::time::Duration::from_millis(50));

        match payload.action.as_str() {
            "CLICK_AND_TYPE" => {
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

                if !payload.text_to_type.is_empty() {
                    let _ = self.set_clipboard_text(&payload.text_to_type);
                    std::thread::sleep(std::time::Duration::from_millis(200));
                    
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
}
