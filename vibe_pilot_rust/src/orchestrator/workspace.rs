use crate::config::SavedConfig;
use crate::screen_capture::{ScreenCapturerTrait, BoundingBox, ScreenInfo};

/// A resolved region representing the active screen workspace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Workspace {
    pub title: String,
    pub bbox: BoundingBox,
    pub is_window: bool,
}

/// A strategy trait to dynamically resolve the current workspace coordinates.
pub trait WorkspaceResolver: Send + Sync {
    /// Resolves the current active workspace bounding box and title based on the system state.
    fn resolve_workspace(
        &self,
        config: &SavedConfig,
        escalate_to_desktop: bool,
        capturer: &dyn ScreenCapturerTrait,
    ) -> Workspace;
}

/// Concrete adaptive workspace cropping strategy.
pub struct AdaptiveWorkspaceResolver;

impl AdaptiveWorkspaceResolver {
    pub fn new() -> Self {
        AdaptiveWorkspaceResolver
    }

    fn resolve_full_desktop(&self, capturer: &dyn ScreenCapturerTrait) -> Workspace {
        let monitors = capturer.get_monitors();
        if monitors.is_empty() {
            return Workspace {
                title: "Default Desktop".to_string(),
                bbox: BoundingBox::new(0, 0, 1920, 1080),
                is_window: false,
            };
        }
        let min_x = monitors.iter().map(|m| m.bbox.left).min().unwrap_or(0);
        let min_y = monitors.iter().map(|m| m.bbox.top).min().unwrap_or(0);
        let max_x = monitors.iter().map(|m| m.bbox.left + m.bbox.width).max().unwrap_or(0);
        let max_y = monitors.iter().map(|m| m.bbox.top + m.bbox.height).max().unwrap_or(0);
        let width = max_x - min_x;
        let height = max_y - min_y;

        Workspace {
            title: "Full Desktop".to_string(),
            bbox: BoundingBox::new(min_x, min_y, width, height),
            is_window: false,
        }
    }

    fn is_desktop_title(&self, title: &str) -> bool {
        let t = title.to_lowercase();
        let t = t.replace(['é', 'è', 'ê', 'ë'], "e");
        t.contains("bureau")
            || t.contains("desktop")
            || t.contains("ecrans")
            || t.contains("screens")
            || t.contains("ecran")
            || t.contains("screen")
            || t.contains("all_screens")
            || t.contains("tous les")
            || t.contains("all screens")
    }

    #[cfg(test)]
    fn get_cursor_position(&self) -> (i32, i32) {
        (0, 0)
    }

    #[cfg(not(test))]
    #[cfg(target_os = "windows")]
    fn get_cursor_position(&self) -> (i32, i32) {
        use windows::Win32::Foundation::POINT;
        use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;
        use crate::screen_capture::DpiAwarenessScope;
        let mut pt = POINT::default();
        unsafe {
            let _scope = DpiAwarenessScope::enter_per_monitor_v2();
            let _ = GetCursorPos(&mut pt);
        }
        (pt.x, pt.y)
    }

    #[cfg(not(test))]
    #[cfg(not(target_os = "windows"))]
    fn get_cursor_position(&self) -> (i32, i32) {
        (0, 0)
    }
}

impl Default for AdaptiveWorkspaceResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl WorkspaceResolver for AdaptiveWorkspaceResolver {
    fn resolve_workspace(
        &self,
        config: &SavedConfig,
        escalate_to_desktop: bool,
        capturer: &dyn ScreenCapturerTrait,
    ) -> Workspace {
        // 1. Check if we should escalate to the full desktop
        if escalate_to_desktop || !config.activer_recadrage_workspace {
            return self.resolve_full_desktop(capturer);
        }

        // 2. If a specific window is targeted in the config, try that first
        if !config.fenetres_surveillees.is_empty() {
            let first_target = &config.fenetres_surveillees[0];
            // If it's a desktop keyword, return the full desktop
            if self.is_desktop_title(first_target) {
                return self.resolve_full_desktop(capturer);
            }

            // Look for the target window in the open windows list
            let wins = capturer.get_windows();
            for win in &wins {
                if crate::screen_capture::is_window_title_match(&win.title, first_target) {
                    if win.bbox.width > 100 && win.bbox.height > 100 {
                        return Workspace {
                            title: win.title.clone(),
                            bbox: win.bbox.clone(),
                            is_window: true,
                        };
                    }
                }
            }
        }

        // 3. Fallback: Check the active foreground window
        if let Some(fg_title) = capturer.get_foreground_window_title() {
            let fg_lower = fg_title.to_lowercase();
            // Don't capture VibePilot window itself or empty window titles as the workspace
            if !fg_lower.is_empty() && !fg_lower.contains("vibepilot") {
                let wins = capturer.get_windows();
                for win in &wins {
                    if crate::screen_capture::is_window_title_match(&win.title, &fg_title) {
                        if win.bbox.width > 100 && win.bbox.height > 100 {
                            return Workspace {
                                title: win.title.clone(),
                                bbox: win.bbox.clone(),
                                is_window: true,
                            };
                        }
                    }
                }
            }
        }

        // 4. Fallback: Target the monitor where the mouse cursor is located
        let (cx, cy) = self.get_cursor_position();
        let monitors = capturer.get_monitors();
        for monitor in &monitors {
            if monitor.bbox.contains(cx, cy) {
                return Workspace {
                    title: monitor.title.clone(),
                    bbox: monitor.bbox.clone(),
                    is_window: false,
                };
            }
        }

        // 5. Ultimate fallback: First monitor
        if let Some(first_monitor) = monitors.first() {
            Workspace {
                title: first_monitor.title.clone(),
                bbox: first_monitor.bbox.clone(),
                is_window: false,
            }
        } else {
            // Safe fallback
            Workspace {
                title: "Fallback Desktop".to_string(),
                bbox: BoundingBox::new(0, 0, 1920, 1080),
                is_window: false,
            }
        }
    }
}
