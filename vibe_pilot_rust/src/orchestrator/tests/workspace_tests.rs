use crate::config::SavedConfig;
use crate::orchestrator::workspace::{WorkspaceResolver, AdaptiveWorkspaceResolver, Workspace};
use crate::screen_capture::{ScreenCapturerTrait, BoundingBox, ScreenInfo, WindowInfo, WindowAnchorResult};
use std::sync::Mutex;
use image::DynamicImage;

struct TestCapturer {
    monitors: Vec<ScreenInfo>,
    windows: Vec<WindowInfo>,
    foreground_title: String,
}

impl ScreenCapturerTrait for TestCapturer {
    fn refresh_windows(&self) {}
    fn get_windows(&self) -> Vec<WindowInfo> {
        self.windows.clone()
    }
    fn get_monitors(&self) -> Vec<ScreenInfo> {
        self.monitors.clone()
    }
    fn capture_desktop(&self) -> Option<DynamicImage> { None }
    fn capture_window_by_title(&self, _title: &str) -> Option<DynamicImage> { None }
    fn capture_bbox(&self, _x: i32, _y: i32, _w: i32, _h: i32) -> Option<DynamicImage> { None }
    fn is_target_visible(&self, _target_windows: &[String]) -> bool { true }
    fn ensure_window_foreground(&self, _title: &str) -> WindowAnchorResult { WindowAnchorResult::AlreadyFocused }
    fn get_foreground_window_title(&self) -> Option<String> {
        Some(self.foreground_title.clone())
    }
}

#[test]
fn test_resolve_workspace_disabled_or_escalated() {
    let resolver = AdaptiveWorkspaceResolver::new();
    let capturer = TestCapturer {
        monitors: vec![
            ScreenInfo::new("Screen 1".to_string(), BoundingBox::new(0, 0, 1920, 1080)),
            ScreenInfo::new("Screen 2".to_string(), BoundingBox::new(1920, 0, 1920, 1080)),
        ],
        windows: vec![],
        foreground_title: "Desktop".to_string(),
    };

    let mut config = SavedConfig::default();
    config.activer_recadrage_workspace = false; // Disabled

    let workspace = resolver.resolve_workspace(&config, false, &capturer);
    assert_eq!(workspace.title, "Full Desktop");
    assert_eq!(workspace.bbox.width, 3840);
    assert_eq!(workspace.bbox.left, 0);

    // Escalated
    config.activer_recadrage_workspace = true;
    let workspace_esc = resolver.resolve_workspace(&config, true, &capturer);
    assert_eq!(workspace_esc.title, "Full Desktop");
    assert_eq!(workspace_esc.bbox.width, 3840);
}

#[test]
fn test_resolve_workspace_target_window() {
    let resolver = AdaptiveWorkspaceResolver::new();
    let capturer = TestCapturer {
        monitors: vec![ScreenInfo::new("Screen 1".to_string(), BoundingBox::new(0, 0, 1920, 1080))],
        windows: vec![
            WindowInfo {
                title: "Brave Web Browser".to_string(),
                hwnd: 12345,
                visible: true,
                bbox: BoundingBox::new(100, 100, 800, 600),
            },
            WindowInfo {
                title: "Discord".to_string(),
                hwnd: 67890,
                visible: true,
                bbox: BoundingBox::new(200, 200, 400, 300),
            },
        ],
        foreground_title: "Desktop".to_string(),
    };

    let mut config = SavedConfig::default();
    config.activer_recadrage_workspace = true;
    config.fenetres_surveillees = vec!["Brave".to_string()];

    let workspace = resolver.resolve_workspace(&config, false, &capturer);
    assert!(workspace.is_window);
    assert!(workspace.title.contains("Brave"));
    assert_eq!(workspace.bbox.left, 100);
    assert_eq!(workspace.bbox.width, 800);
}

#[test]
fn test_resolve_workspace_foreground_fallback() {
    let resolver = AdaptiveWorkspaceResolver::new();
    let capturer = TestCapturer {
        monitors: vec![ScreenInfo::new("Screen 1".to_string(), BoundingBox::new(0, 0, 1920, 1080))],
        windows: vec![
            WindowInfo {
                title: "Slack - Work".to_string(),
                hwnd: 1111,
                visible: true,
                bbox: BoundingBox::new(50, 50, 700, 500),
            }
        ],
        foreground_title: "Slack - Work".to_string(), // Foreground matches Slack
    };

    let mut config = SavedConfig::default();
    config.activer_recadrage_workspace = true;
    config.fenetres_surveillees = vec![]; // No targets

    let workspace = resolver.resolve_workspace(&config, false, &capturer);
    assert!(workspace.is_window);
    assert_eq!(workspace.title, "Slack - Work");
    assert_eq!(workspace.bbox.width, 700);
}

#[test]
fn test_resolve_workspace_monitor_cursor_fallback() {
    let resolver = AdaptiveWorkspaceResolver::new();
    let capturer = TestCapturer {
        monitors: vec![
            ScreenInfo::new("Screen 1".to_string(), BoundingBox::new(0, 0, 1920, 1080)),
            ScreenInfo::new("Screen 2".to_string(), BoundingBox::new(1920, 0, 1920, 1080)),
        ],
        windows: vec![],
        foreground_title: "VibePilot".to_string(), // Ignored title
    };

    let mut config = SavedConfig::default();
    config.activer_recadrage_workspace = true;
    config.fenetres_surveillees = vec![];

    // Since we are not on Windows or the cursor is (0,0) during test, it will fall back to Screen 1.
    let workspace = resolver.resolve_workspace(&config, false, &capturer);
    assert_eq!(workspace.bbox.left, 0);
    assert_eq!(workspace.bbox.width, 1920);
}

#[test]
fn test_resolve_workspace_desktop_keyword() {
    let resolver = AdaptiveWorkspaceResolver::new();
    let capturer = TestCapturer {
        monitors: vec![ScreenInfo::new("Screen 1".to_string(), BoundingBox::new(0, 0, 1920, 1080))],
        windows: vec![],
        foreground_title: "".to_string(),
    };

    let mut config = SavedConfig::default();
    config.activer_recadrage_workspace = true;
    config.fenetres_surveillees = vec!["Tous les ecrans".to_string()];

    let workspace = resolver.resolve_workspace(&config, false, &capturer);
    assert_eq!(workspace.title, "Full Desktop");
}

#[test]
fn test_resolve_workspace_empty_monitors_fallback() {
    let resolver = AdaptiveWorkspaceResolver::new();
    let capturer = TestCapturer {
        monitors: vec![],
        windows: vec![],
        foreground_title: "".to_string(),
    };

    let mut config = SavedConfig::default();
    config.activer_recadrage_workspace = true;
    config.fenetres_surveillees = vec![];

    let workspace = resolver.resolve_workspace(&config, false, &capturer);
    assert_eq!(workspace.title, "Fallback Desktop");
}

#[test]
fn test_workspace_resolver_additional_branches() {
    let resolver = AdaptiveWorkspaceResolver::new();

    // 1. Test is_desktop_title branches indirectly
    let capturer_desktop = TestCapturer {
        monitors: vec![ScreenInfo::new("Screen 1".to_string(), BoundingBox::new(0, 0, 1920, 1080))],
        windows: vec![],
        foreground_title: "".to_string(),
    };
    let mut config = SavedConfig::default();
    config.activer_recadrage_workspace = true;

    config.fenetres_surveillees = vec!["Bureau".to_string()];
    let ws = resolver.resolve_workspace(&config, false, &capturer_desktop);
    assert_eq!(ws.title, "Full Desktop");

    config.fenetres_surveillees = vec!["écrans".to_string()];
    let ws = resolver.resolve_workspace(&config, false, &capturer_desktop);
    assert_eq!(ws.title, "Full Desktop");

    config.fenetres_surveillees = vec!["All Screens".to_string()];
    let ws = resolver.resolve_workspace(&config, false, &capturer_desktop);
    assert_eq!(ws.title, "Full Desktop");

    // 2. Test window match with small bounding box (should be skipped)
    let capturer_small_box = TestCapturer {
        monitors: vec![ScreenInfo::new("Screen 1".to_string(), BoundingBox::new(0, 0, 1920, 1080))],
        windows: vec![WindowInfo {
            title: "Slack".to_string(),
            hwnd: 1111,
            visible: true,
            bbox: BoundingBox::new(0, 0, 50, 50), // small
        }],
        foreground_title: "".to_string(),
    };
    let mut config = SavedConfig::default();
    config.activer_recadrage_workspace = true;
    config.fenetres_surveillees = vec!["Slack".to_string()];
    let workspace = resolver.resolve_workspace(&config, false, &capturer_small_box);
    // Should fall back to the first monitor since Slack has a small box
    assert_eq!(workspace.title, "Screen 1");

    // 3. Test foreground title is empty
    let capturer_empty_fg = TestCapturer {
        monitors: vec![ScreenInfo::new("Screen 1".to_string(), BoundingBox::new(0, 0, 1920, 1080))],
        windows: vec![],
        foreground_title: "".to_string(),
    };
    config.fenetres_surveillees = vec![];
    let workspace_empty = resolver.resolve_workspace(&config, false, &capturer_empty_fg);
    assert_eq!(workspace_empty.title, "Screen 1");

    // 4. Test foreground title contains "vibepilot" (should be skipped)
    let capturer_vibepilot_fg = TestCapturer {
        monitors: vec![ScreenInfo::new("Screen 1".to_string(), BoundingBox::new(0, 0, 1920, 1080))],
        windows: vec![],
        foreground_title: "VibePilot Application".to_string(),
    };
    let workspace_vibepilot = resolver.resolve_workspace(&config, false, &capturer_vibepilot_fg);
    assert_eq!(workspace_vibepilot.title, "Screen 1");

    // 5. Test foreground window matching but has small box
    let capturer_fg_small = TestCapturer {
        monitors: vec![ScreenInfo::new("Screen 1".to_string(), BoundingBox::new(0, 0, 1920, 1080))],
        windows: vec![WindowInfo {
            title: "Foreground App".to_string(),
            hwnd: 2222,
            visible: true,
            bbox: BoundingBox::new(0, 0, 50, 50),
        }],
        foreground_title: "Foreground App".to_string(),
    };
    let workspace_fg_small = resolver.resolve_workspace(&config, false, &capturer_fg_small);
    assert_eq!(workspace_fg_small.title, "Screen 1");
}
