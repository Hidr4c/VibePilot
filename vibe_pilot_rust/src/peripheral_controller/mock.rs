use std::sync::{Arc, Mutex};
use super::payload::{ActionPayload, ScreenOffset};
use crate::peripheral_controller::{
    MouseInput, KeyboardInput, ScrollInput, ClipboardInput, CoordinateMapping, PeripheralInput
};

/// Mock implementation of PeripheralInput for unit testing.
pub struct MockPeripheralInput {
    pub actions: Arc<Mutex<Vec<String>>>,
    pub clipboard: Arc<Mutex<String>>,
}

impl MockPeripheralInput {
    pub fn new() -> Self {
        Self {
            actions: Arc::new(Mutex::new(Vec::new())),
            clipboard: Arc::new(Mutex::new(String::new())),
        }
    }
}

impl Default for MockPeripheralInput {
    fn default() -> Self {
        Self::new()
    }
}

impl MouseInput for MockPeripheralInput {
    fn right_click(&self) -> Result<(), String> {
        self.actions.lock().unwrap().push("right_click".to_string());
        Ok(())
    }
    fn middle_click(&self) -> Result<(), String> {
        self.actions.lock().unwrap().push("middle_click".to_string());
        Ok(())
    }
    fn double_click(&self) -> Result<(), String> {
        self.actions.lock().unwrap().push("double_click".to_string());
        Ok(())
    }
    fn drag_and_drop(&self, from: (i32, i32), to: (i32, i32), button: rdev::Button) -> Result<(), String> {
        self.actions.lock().unwrap().push(format!("drag_and_drop: from {:?} to {:?} btn {:?}", from, to, button));
        Ok(())
    }
    fn mouse_move_relative(&self, dx: i32, dy: i32) -> Result<(), String> {
        self.actions.lock().unwrap().push(format!("mouse_move_relative: dx: {} dy: {}", dx, dy));
        Ok(())
    }
}

impl KeyboardInput for MockPeripheralInput {
    fn key_combo(&self, keys: &[rdev::Key]) -> Result<(), String> {
        self.actions.lock().unwrap().push(format!("key_combo: {:?}", keys));
        Ok(())
    }
}

impl ScrollInput for MockPeripheralInput {
    fn scroll_vertical(&self, amount: i32) -> Result<(), String> {
        self.actions.lock().unwrap().push(format!("scroll_vertical: {}", amount));
        Ok(())
    }
    fn scroll_horizontal(&self, amount: i32) -> Result<(), String> {
        self.actions.lock().unwrap().push(format!("scroll_horizontal: {}", amount));
        Ok(())
    }
    fn smooth_scroll_vertical(&self, amount: i32, steps: u32, duration_ms: u64) -> Result<(), String> {
        self.actions.lock().unwrap().push(format!("smooth_scroll_vertical: {} steps: {} duration: {}ms", amount, steps, duration_ms));
        Ok(())
    }
    fn quick_scroll_vertical(&self, direction: &str, intensity: &str) -> Result<(), String> {
        self.actions.lock().unwrap().push(format!("quick_scroll_vertical: {} intensity: {}", direction, intensity));
        Ok(())
    }
}

impl ClipboardInput for MockPeripheralInput {
    fn set_clipboard_text(&self, text: &str) -> Result<(), String> {
        let mut clip = self.clipboard.lock().unwrap();
        *clip = text.to_string();
        self.actions.lock().unwrap().push(format!("set_clipboard: {}", text));
        Ok(())
    }
    fn get_clipboard_text(&self) -> Result<String, String> {
        Ok(self.clipboard.lock().unwrap().clone())
    }
    fn copy_selected(&self) -> Result<(), String> {
        self.actions.lock().unwrap().push("copy_selected".to_string());
        Ok(())
    }
    fn paste(&self) -> Result<(), String> {
        self.actions.lock().unwrap().push("paste".to_string());
        Ok(())
    }
    fn cut_selected(&self) -> Result<(), String> {
        self.actions.lock().unwrap().push("cut_selected".to_string());
        Ok(())
    }
    fn select_all(&self) -> Result<(), String> {
        self.actions.lock().unwrap().push("select_all".to_string());
        Ok(())
    }
    fn set_clipboard_html(&self, html: &str) -> Result<(), String> {
        let mut clip = self.clipboard.lock().unwrap();
        *clip = format!("html:{}", html);
        self.actions.lock().unwrap().push(format!("set_clipboard_html: {}", html));
        Ok(())
    }
    fn get_clipboard_html(&self) -> Result<String, String> {
        Ok(self.clipboard.lock().unwrap().clone())
    }
    fn set_clipboard_image(&self, _img: &image::DynamicImage) -> Result<(), String> {
        self.actions.lock().unwrap().push("set_clipboard_image".to_string());
        Ok(())
    }
    fn get_clipboard_image(&self) -> Result<image::DynamicImage, String> {
        Ok(image::DynamicImage::ImageRgba8(image::ImageBuffer::new(10, 10)))
    }
}

impl CoordinateMapping for MockPeripheralInput {
    fn compute_absolute_coordinates(&self, x_rel: f64, y_rel: f64, _offsets: &[ScreenOffset]) -> (i32, i32) {
        ((x_rel * 1920.0) as i32, (y_rel * 1080.0) as i32)
    }
    fn is_desktop_title(&self, title: &str) -> bool {
        title.contains("Desktop")
    }
    fn get_screen_bounds(&self) -> (i32, i32, i32, i32) {
        (0, 0, 1920, 1080)
    }
}

impl PeripheralInput for MockPeripheralInput {
    fn execute_action(&self, payload: &ActionPayload, _offsets: &[ScreenOffset], _verifier_placement_souris: bool) {
        let mut act = self.actions.lock().unwrap();
        act.push(format!("execute_action: {}", payload.action));
        if !payload.text_to_type.is_empty() {
            let mut clip = self.clipboard.lock().unwrap();
            *clip = payload.text_to_type.clone();
            act.push(format!("type_text: {}", payload.text_to_type));
        }
        if payload.scroll_value != 0 {
            act.push(format!("scroll: {}", payload.scroll_value));
        }
    }
}
