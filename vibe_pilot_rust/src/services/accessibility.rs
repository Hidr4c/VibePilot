use std::sync::Arc;

/// Trait defining system-level accessibility tree layout extraction.
pub trait AccessibilityParser: Send + Sync {
    /// Parses the active foreground application window controls layout.
    /// Returns a Markdown representation of buttons, labels, and text fields.
    fn parse_active_window(&self) -> Result<String, String>;
}

/// No-op implementation of AccessibilityParser for non-Windows environments.
pub struct NullAccessibilityParser;

impl NullAccessibilityParser {
    /// Creates a new NullAccessibilityParser.
    pub fn new() -> Self {
        Self
    }
}

impl AccessibilityParser for NullAccessibilityParser {
    fn parse_active_window(&self) -> Result<String, String> {
        Ok("Accessibility parsing is not available on this platform.".to_string())
    }
}

#[cfg(target_os = "windows")]
struct ControlInfo {
    class_name: String,
    text: String,
    rect: windows::Win32::Foundation::RECT,
}

/// Windows Win32-based window traversal layout parser.
#[cfg(target_os = "windows")]
pub struct Win32AccessibilityParser;

#[cfg(target_os = "windows")]
impl Win32AccessibilityParser {
    /// Creates a new Win32AccessibilityParser.
    pub fn new() -> Self {
        Self
    }
}

#[cfg(target_os = "windows")]
impl AccessibilityParser for Win32AccessibilityParser {
    fn parse_active_window(&self) -> Result<String, String> {
        use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, EnumChildWindows, GetClassNameW, GetWindowTextW, GetWindowRect};
        use windows::Win32::Foundation::{HWND, LPARAM};

        let hwnd = unsafe { GetForegroundWindow() };
        if hwnd.0.is_null() {
            return Err("No active window found".to_string());
        }

        let mut controls: Vec<ControlInfo> = Vec::new();

        unsafe {
            let _ = EnumChildWindows(hwnd, Some(enum_child_proc), LPARAM(&mut controls as *mut _ as isize));
        }

        unsafe extern "system" fn enum_child_proc(child_hwnd: HWND, lparam: LPARAM) -> windows::Win32::Foundation::BOOL {
            let list = &mut *(lparam.0 as *mut Vec<ControlInfo>);

            let mut class_buf = [0u16; 256];
            let class_len = GetClassNameW(child_hwnd, &mut class_buf);
            let class_name = String::from_utf16_lossy(&class_buf[..class_len as usize]);

            let mut text_buf = [0u16; 512];
            let text_len = GetWindowTextW(child_hwnd, &mut text_buf);
            let text = String::from_utf16_lossy(&text_buf[..text_len as usize]);

            let mut rect = windows::Win32::Foundation::RECT::default();
            let _ = GetWindowRect(child_hwnd, &mut rect);

            list.push(ControlInfo {
                class_name,
                text,
                rect,
            });

            if list.len() >= 50 {
                windows::Win32::Foundation::FALSE
            } else {
                windows::Win32::Foundation::TRUE
            }
        }

        let mut output = String::new();
        output.push_str("### Monitored Window Accessibility Layout Tree:\n");
        if controls.is_empty() {
            output.push_str("- No child control elements detected.\n");
        } else {
            for ctrl in controls {
                let clean_text = ctrl.text.trim();
                let clean_class = ctrl.class_name.trim();
                if clean_text.is_empty() && clean_class.is_empty() {
                    continue;
                }
                output.push_str(&format!(
                    "- [Class: {}] Text: \"{}\" | BBox: (left={}, top={}, width={}, height={})\n",
                    clean_class,
                    clean_text,
                    ctrl.rect.left,
                    ctrl.rect.top,
                    ctrl.rect.right - ctrl.rect.left,
                    ctrl.rect.bottom - ctrl.rect.top
                ));
            }
        }

        Ok(output)
    }
}

/// Factory to construct AccessibilityParser implementations following the IoC pattern.
pub struct AccessibilityParserFactory;

impl AccessibilityParserFactory {
    /// Creates the default AccessibilityParser implementation.
    pub fn create() -> Arc<dyn AccessibilityParser> {
        #[cfg(target_os = "windows")]
        {
            Arc::new(Win32AccessibilityParser::new())
        }
        #[cfg(not(target_os = "windows"))]
        {
            Arc::new(NullAccessibilityParser::new())
        }
    }
}
