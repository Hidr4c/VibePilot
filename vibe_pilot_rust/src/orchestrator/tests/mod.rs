mod loop_breaker_tests;
mod memory_tests;
mod helper_tests;
mod orchestrator_run_tests;
mod orchestrator_run_more_tests;
mod orchestrator_run_extra_tests;
mod workspace_tests;
mod decision_handler_tests;

use crate::config::{ConfigurationRepository, ConfigManagement, ProfileManagement, EngineManagement, StateManagement, ConfigPaths, SavedConfig, EnginePresets};
use crate::llm_client::{LlmVisionProvider, LlmTextProvider, LlmMetadataProvider, LlmProvider, LlmResponse};
use crate::screen_capture::{ScreenCapturerTrait, ScreenInfo, BoundingBox, WindowInfo, WindowAnchorResult};
use crate::peripheral_controller::{PeripheralInput, MouseInput, KeyboardInput, ScrollInput, ClipboardInput, CoordinateMapping, ActionPayload, ScreenOffset};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, Arc};

// Shared Mocks
pub(crate) struct MockConfigRepo {
    pub(crate) config: Mutex<SavedConfig>,
}

impl ConfigManagement for MockConfigRepo {
    fn load_config(&self) -> SavedConfig {
        self.config.lock().unwrap().clone()
    }
    fn save_config(&self, config: &SavedConfig) {
        *self.config.lock().unwrap() = config.clone();
    }
    fn save_now(&self) -> Result<(), String> { Ok(()) }
    fn get_save_path(&self) -> PathBuf {
        std::env::temp_dir().join("save.enc")
    }
    fn get_key_path(&self) -> PathBuf {
        std::env::temp_dir().join("key.enc")
    }
}

impl ProfileManagement for MockConfigRepo {
    fn load_profile(&self, _name: &str) -> Option<SavedConfig> {
        None
    }
    fn save_profile(&self, _name: &str, _config: &SavedConfig) -> bool {
        true
    }
    fn delete_profile(&self, _name: &str) -> bool {
        true
    }
    fn list_profiles(&self) -> Vec<String> {
        vec![]
    }
    fn get_profiles_dir(&self) -> PathBuf {
        std::env::temp_dir().join("profiles")
    }
    fn export_profiles(&self, _path: &Path) -> Result<(), String> { Ok(()) }
    fn import_profiles(&self, _path: &Path) -> Result<usize, String> { Ok(0) }
    fn export_single_profile(&self, _name: &str, _path: &Path) -> Result<(), String> { Ok(()) }
    fn import_single_profile(&self, _path: &Path) -> Result<String, String> { Ok(String::new()) }
}

impl EngineManagement for MockConfigRepo {
    fn load_engines(&self) -> EnginePresets {
        EnginePresets::default()
    }
    fn save_engines(&self, _presets: &EnginePresets) {}
    fn get_engines_path(&self) -> PathBuf {
        std::env::temp_dir().join("engines.enc")
    }
    fn export_engines(&self, _path: &Path) -> Result<(), String> { Ok(()) }
    fn import_engines(&self, _path: &Path) -> Result<(), String> { Ok(()) }
    fn export_single_engine(&self, _name: &str, _path: &Path) -> Result<(), String> { Ok(()) }
    fn import_single_engine(&self, _path: &Path) -> Result<String, String> { Ok(String::new()) }
}

thread_local! {
    pub(crate) static MOCK_TASK_GRAPH: std::cell::RefCell<Option<crate::memory::TaskGraph>> = std::cell::RefCell::new(None);
}

impl StateManagement for MockConfigRepo {
    fn load_task_graph(&self) -> Option<crate::memory::TaskGraph> {
        MOCK_TASK_GRAPH.with(|g| g.borrow().clone())
    }
    fn save_task_graph(&self, graph: Option<crate::memory::TaskGraph>) {
        MOCK_TASK_GRAPH.with(|g| *g.borrow_mut() = graph);
    }
    fn export_config(&self, _path: &Path) -> Result<(), String> { Ok(()) }
    fn import_config(&self, _path: &Path) -> Result<(), String> { Ok(()) }
}

impl ConfigPaths for MockConfigRepo {
    fn get_base_dir(&self) -> PathBuf {
        let thread_name = std::thread::current().name().unwrap_or("default").to_string();
        let safe_name = thread_name.chars()
            .filter(|c| c.is_alphanumeric() || *c == '_' || *c == ':')
            .map(|c| if c == ':' { '_' } else { c })
            .collect::<String>();
        let path = std::env::temp_dir().join(format!("vibepilot_test_{}", safe_name));
        let _ = std::fs::create_dir_all(&path);
        path
    }
    fn get_store_path(&self) -> PathBuf {
        self.get_base_dir().join("store.enc")
    }
}

pub(crate) struct MockLlmClient {
    pub(crate) next_response: Mutex<LlmResponse>,
    pub(crate) decompose_response: Mutex<String>,
    pub(crate) call_count: Mutex<usize>,
}

impl LlmVisionProvider for MockLlmClient {
    fn execute_decision<'a>(
        &'a self,
        _image: &'a image::DynamicImage,
        _contexte: &'a str,
        _objectif: &'a str,
        _task: &'a str,
        _directives: &'a str,
        _user_feedback: &'a str,
        _url: &'a str,
        _model: &'a str,
        _auth_mode: &'a str,
        _auth_api_key: &'a str,
        _auth_login: &'a str,
        _auth_password: &'a str,
        _timeout_secs: u64,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<LlmResponse, String>> + Send + 'a>> {
        let mut count = self.call_count.lock().unwrap();
        *count += 1;
        let resp = if *count == 1 {
            LlmResponse {
                status_display: "Clicking...".to_string(),
                action: "CLICK_AND_TYPE".to_string(),
                relative_click_position: vec![0.5, 0.5],
                text_to_type: "hello".to_string(),
                report: Some("Clicking button".to_string()),
                ..LlmResponse::default()
            }
        } else {
            self.next_response.lock().unwrap().clone()
        };
        Box::pin(async move { Ok(resp) })
    }

    fn identify_roi<'a>(
        &'a self,
        _image: &'a image::DynamicImage,
        _contexte: &'a str,
        _objectif: &'a str,
        _task: &'a str,
        _url: &'a str,
        _model: &'a str,
        _auth_mode: &'a str,
        _auth_api_key: &'a str,
        _auth_login: &'a str,
        _auth_password: &'a str,
        _timeout_secs: u64,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<String, String>> + Send + 'a>> {
        Box::pin(async { Ok("{\"x\": 0.5, \"y\": 0.5, \"width\": 0.2, \"height\": 0.2, \"label\": \"roi\"}".to_string()) })
    }
}

impl LlmTextProvider for MockLlmClient {
    fn optimize_field<'a>(
        &'a self,
        _text: &'a str,
        _field_type: &'a str,
        _url: &'a str,
        _model: &'a str,
        _auth_mode: &'a str,
        _auth_api_key: &'a str,
        _auth_login: &'a str,
        _auth_password: &'a str,
        _timeout_secs: u64,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<String, String>> + Send + 'a>> {
        Box::pin(async { Ok("optimized".to_string()) })
    }

    fn generate_config<'a>(
        &'a self,
        _user_request: &'a str,
        _url: &'a str,
        _model: &'a str,
        _auth_mode: &'a str,
        _auth_api_key: &'a str,
        _auth_login: &'a str,
        _auth_password: &'a str,
        _timeout_secs: u64,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<serde_json::Value, String>> + Send + 'a>> {
        Box::pin(async { Ok(serde_json::json!({})) })
    }

    fn compile_dag<'a>(
        &'a self,
        _prompt: &'a str,
        _url: &'a str,
        _model: &'a str,
        _auth_mode: &'a str,
        _auth_api_key: &'a str,
        _auth_login: &'a str,
        _auth_password: &'a str,
        _timeout_secs: u64,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<serde_json::Value, String>> + Send + 'a>> {
        Box::pin(async {
            Ok(serde_json::json!([
                {
                    "id": "node_1",
                    "name": "1. Test Node",
                    "profile_name": "Test Profile",
                    "worker_url": "http://127.0.0.1:4040",
                    "duration_secs": 5.0,
                    "dependencies": []
                }
            ]))
        })
    }

    fn decompose_objective<'a>(
        &'a self,
        _objectif: &'a str,
        _contexte: &'a str,
        _task: &'a str,
        _url: &'a str,
        _model: &'a str,
        _auth_mode: &'a str,
        _auth_api_key: &'a str,
        _auth_login: &'a str,
        _auth_password: &'a str,
        _timeout_secs: u64,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<String, String>> + Send + 'a>> {
        let resp = self.decompose_response.lock().unwrap().clone();
        Box::pin(async move { Ok(resp) })
    }

    fn compress_history<'a>(
        &'a self,
        _old_steps_text: &'a str,
        _previous_summary: &'a str,
        _langue: &'a str,
        _url: &'a str,
        _model: &'a str,
        _auth_mode: &'a str,
        _auth_api_key: &'a str,
        _auth_login: &'a str,
        _auth_password: &'a str,
        _timeout_secs: u64,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<String, String>> + Send + 'a>> {
        Box::pin(async { Ok("summary".to_string()) })
    }
}

impl LlmMetadataProvider for MockLlmClient {
    fn is_engine_busy<'a>(&'a self, _url: &'a str, _model: &'a str) -> std::pin::Pin<Box<dyn std::future::Future<Output = bool> + Send + 'a>> {
        Box::pin(async { false })
    }

    fn fetch_models<'a>(
        &'a self,
        _url: &'a str,
        _auth_mode: &'a str,
        _auth_api_key: &'a str,
        _auth_login: &'a str,
        _auth_password: &'a str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<String>, String>> + Send + 'a>> {
        Box::pin(async { Ok(vec!["mock-model".to_string()]) })
    }
}

pub(crate) struct MockScreenCapturer {
    pub(crate) window_foreground_result: Mutex<WindowAnchorResult>,
}

impl ScreenCapturerTrait for MockScreenCapturer {
    fn capture_desktop(&self) -> Option<image::DynamicImage> {
        Some(image::DynamicImage::ImageRgba8(image::ImageBuffer::new(100, 100)))
    }
    fn capture_bbox(&self, _x: i32, _y: i32, _w: i32, _h: i32) -> Option<image::DynamicImage> {
        Some(image::DynamicImage::ImageRgba8(image::ImageBuffer::new(10, 10)))
    }
    fn get_monitors(&self) -> Vec<ScreenInfo> {
        vec![ScreenInfo::new(
            "Screen 1".to_string(),
            BoundingBox::new(0, 0, 1920, 1080)
        )]
    }
    fn refresh_windows(&self) {}
    fn get_windows(&self) -> Vec<WindowInfo> {
        vec![WindowInfo {
            title: "Mock Window".to_string(),
            hwnd: 1234,
            visible: true,
            bbox: BoundingBox::new(100, 100, 500, 400),
        }]
    }
    fn is_target_visible(&self, _target_windows: &[String]) -> bool {
        true
    }
    fn ensure_window_foreground(&self, _target_title: &str) -> WindowAnchorResult {
        self.window_foreground_result.lock().unwrap().clone()
    }
    fn capture_window_by_title(&self, _title: &str) -> Option<image::DynamicImage> {
        Some(image::DynamicImage::ImageRgba8(image::ImageBuffer::new(100, 100)))
    }
    fn get_foreground_window_title(&self) -> Option<String> {
        Some("Mock Window".to_string())
    }
}

pub(crate) struct MockPeripheralController {
    pub(crate) actions: Mutex<Vec<String>>,
}

impl MouseInput for MockPeripheralController {
    fn right_click(&self) -> Result<(), String> { Ok(()) }
    fn middle_click(&self) -> Result<(), String> { Ok(()) }
    fn double_click(&self) -> Result<(), String> { Ok(()) }
    fn drag_and_drop(&self, _from: (i32, i32), _to: (i32, i32), _button: rdev::Button) -> Result<(), String> { Ok(()) }
    fn mouse_move_relative(&self, dx: i32, dy: i32) -> Result<(), String> {
        self.actions.lock().unwrap().push(format!("mouse_move_relative: {}, {}", dx, dy));
        Ok(())
    }
}

impl KeyboardInput for MockPeripheralController {
    fn key_combo(&self, keys: &[rdev::Key]) -> Result<(), String> {
        self.actions.lock().unwrap().push(format!("key_combo: {:?}", keys));
        Ok(())
    }
}

impl ScrollInput for MockPeripheralController {
    fn scroll_vertical(&self, amount: i32) -> Result<(), String> {
        self.actions.lock().unwrap().push(format!("scroll_vertical: {}", amount));
        Ok(())
    }
    fn scroll_horizontal(&self, _amount: i32) -> Result<(), String> { Ok(()) }
    fn smooth_scroll_vertical(&self, _amount: i32, _steps: u32, _duration_ms: u64) -> Result<(), String> { Ok(()) }
    fn quick_scroll_vertical(&self, _direction: &str, _intensity: &str) -> Result<(), String> { Ok(()) }
}

impl ClipboardInput for MockPeripheralController {
    fn set_clipboard_text(&self, text: &str) -> Result<(), String> {
        self.actions.lock().unwrap().push(format!("set_clipboard: {}", text));
        Ok(())
    }
    fn get_clipboard_text(&self) -> Result<String, String> {
        Ok(String::new())
    }
    fn set_clipboard_html(&self, html: &str) -> Result<(), String> {
        self.actions.lock().unwrap().push(format!("set_clipboard_html: {}", html));
        Ok(())
    }
    fn get_clipboard_html(&self) -> Result<String, String> {
        Ok(String::new())
    }
    fn copy_selected(&self) -> Result<(), String> { Ok(()) }
    fn paste(&self) -> Result<(), String> { Ok(()) }
    fn cut_selected(&self) -> Result<(), String> { Ok(()) }
    fn select_all(&self) -> Result<(), String> { Ok(()) }
    fn set_clipboard_image(&self, _img: &image::DynamicImage) -> Result<(), String> { Ok(()) }
    fn get_clipboard_image(&self) -> Result<image::DynamicImage, String> {
        Ok(image::DynamicImage::ImageRgba8(image::ImageBuffer::new(10, 10)))
    }
}

impl CoordinateMapping for MockPeripheralController {
    fn compute_absolute_coordinates(&self, _x_rel: f64, _y_rel: f64, _offsets: &[ScreenOffset]) -> (i32, i32) {
        (0, 0)
    }
    fn is_desktop_title(&self, title: &str) -> bool {
        title == "Desktop" || title == "Bureau"
    }
    fn get_screen_bounds(&self) -> (i32, i32, i32, i32) {
        (0, 0, 1920, 1080)
    }
}

impl PeripheralInput for MockPeripheralController {
    fn execute_action(&self, payload: &ActionPayload, _offsets: &[ScreenOffset], _verifier: bool) {
        let coords_str = if payload.relative_click_position.len() >= 2 {
            format!(" at {:.3}, {:.3}", payload.relative_click_position[0], payload.relative_click_position[1])
        } else {
            String::new()
        };
        self.actions.lock().unwrap().push(format!("execute: {}{}", payload.action, coords_str));
    }
}

pub(crate) struct MockWaitManager;
impl crate::services::wait_manager::WaitManagerTrait for MockWaitManager {
    fn wait_for(
        &self,
        _condition: crate::llm_client::WaitCondition,
        _timeout_secs: u32,
        _screen_capturer: Arc<dyn crate::screen_capture::ScreenCapturerTrait>,
        _bus: crate::event_bus::EventBus,
        _skip_trigger: Arc<std::sync::atomic::AtomicBool>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = bool> + Send + 'static>> {
        Box::pin(async { true })
    }
}
