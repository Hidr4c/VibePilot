//! Peripheral controller for VibePilot.
//!
//! Handles physical actions: mouse movement, click, scroll, keyboard input.
//! Uses rdev for input simulation and Windows API for clipboard.

pub mod payload;
pub mod concrete;
pub mod mock;
pub mod native_win;
pub mod coordinate_mapping;
pub mod factory;


#[cfg(test)]
mod tests;

pub use payload::{ActionPayload, ScreenOffset};
pub use concrete::PeripheralController;
pub use mock::MockPeripheralInput;
pub use factory::PeripheralControllerFactory;

#[cfg(target_os = "windows")]
pub use native_win::{
    win32_move_mouse_absolute,
    win32_click_current_position,
    win32_right_click_current_position,
    win32_middle_click_current_position,
};

// === Interface Segregation: Focused Sub-Traits ===

/// Mouse-related actions: clicks, dragging, relative movement.
pub trait MouseInput: Send + Sync {
    fn right_click(&self) -> Result<(), String>;
    fn middle_click(&self) -> Result<(), String>;
    fn double_click(&self) -> Result<(), String>;
    fn drag_and_drop(&self, from: (i32, i32), to: (i32, i32), button: rdev::Button) -> Result<(), String>;
    fn mouse_move_relative(&self, dx: i32, dy: i32) -> Result<(), String>;
}

/// Keyboard-related actions: key combinations.
pub trait KeyboardInput: Send + Sync {
    fn key_combo(&self, keys: &[rdev::Key]) -> Result<(), String>;
}

/// Scroll-related actions: vertical, horizontal, smooth, quick scroll.
pub trait ScrollInput: Send + Sync {
    fn scroll_vertical(&self, amount: i32) -> Result<(), String>;
    fn scroll_horizontal(&self, amount: i32) -> Result<(), String>;
    fn smooth_scroll_vertical(&self, amount: i32, steps: u32, duration_ms: u64) -> Result<(), String>;
    fn quick_scroll_vertical(&self, direction: &str, intensity: &str) -> Result<(), String>;
}

/// Clipboard-related actions: set, get, copy, paste, cut, select all.
pub trait ClipboardInput: Send + Sync {
    fn set_clipboard_text(&self, text: &str) -> Result<(), String>;
    fn get_clipboard_text(&self) -> Result<String, String>;
    fn set_clipboard_html(&self, html: &str) -> Result<(), String>;
    fn get_clipboard_html(&self) -> Result<String, String>;
    fn copy_selected(&self) -> Result<(), String>;
    fn paste(&self) -> Result<(), String>;
    fn cut_selected(&self) -> Result<(), String>;
    fn select_all(&self) -> Result<(), String>;
    fn set_clipboard_image(&self, img: &image::DynamicImage) -> Result<(), String>;
    fn get_clipboard_image(&self) -> Result<image::DynamicImage, String>;
}

/// Coordinate mapping and screen detection utilities.
pub trait CoordinateMapping: Send + Sync {
    fn compute_absolute_coordinates(&self, x_rel: f64, y_rel: f64, offsets: &[ScreenOffset]) -> (i32, i32);
    fn is_desktop_title(&self, title: &str) -> bool;
    fn get_screen_bounds(&self) -> (i32, i32, i32, i32);
}

/// Composite trait that extends all 5 sub-traits plus `execute_action`.
pub trait PeripheralInput: MouseInput + KeyboardInput + ScrollInput + ClipboardInput + CoordinateMapping + Send + Sync {
    fn execute_action(&self, payload: &ActionPayload, offsets: &[ScreenOffset], verifier_placement_souris: bool);
}
