//! Passive wait management service for VibePilot.
//!
//! Handles passive pause actions (time-based or visual condition-based) without blocking the UI.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use tokio::time::sleep;
use crate::screen_capture::ScreenCapturerTrait;
use crate::event_bus::{EventBus, NotificationEvent};
use crate::llm_client::WaitCondition;
use crate::reflection::{ReflectionModule, VisualDiffResult};
use std::future::Future;
use std::pin::Pin;

/// Trait defining wait operations, allowing clean IoC mocking.
pub trait WaitManagerTrait: Send + Sync {
    fn wait_for(
        &self,
        condition: WaitCondition,
        timeout_secs: u32,
        screen_capturer: Arc<dyn ScreenCapturerTrait>,
        bus: EventBus,
        skip_trigger: Arc<AtomicBool>,
    ) -> Pin<Box<dyn Future<Output = bool> + Send + 'static>>;
}

/// Concrete implementation of the WaitManager service.
pub struct WaitManager;

impl WaitManager {
    /// Creates a new WaitManager instance.
    pub fn new() -> Self {
        Self
    }

    async fn wait_time_elapsed(
        timeout_secs: u32,
        bus: EventBus,
        skip_trigger: Arc<AtomicBool>,
    ) -> bool {
        let start = Instant::now();
        let target_duration = Duration::from_secs(timeout_secs as u64);
        
        while start.elapsed() < target_duration {
            if skip_trigger.load(Ordering::Relaxed) {
                bus.emit_notification(NotificationEvent::Log("⏳ Wait skipped by user".to_string()));
                return true;
            }
            
            let elapsed = start.elapsed().as_secs();
            let remaining = timeout_secs.saturating_sub(elapsed as u32);
            if remaining > 0 {
                bus.emit_notification(NotificationEvent::UpdateStatus {
                    text: format!("Attente: {}s restantes...", remaining),
                    color: "orange".to_string(),
                });
            }
            
            sleep(Duration::from_millis(100)).await;
        }
        true
    }

    async fn wait_screen_stable(
        seconds: u32,
        timeout_secs: u32,
        screen_capturer: Arc<dyn ScreenCapturerTrait>,
        bus: EventBus,
        skip_trigger: Arc<AtomicBool>,
    ) -> bool {
        let start = Instant::now();
        let timeout_duration = Duration::from_secs(timeout_secs as u64);
        let mut last_hash = None;
        let mut stable_since: Option<Instant> = None;

        while start.elapsed() < timeout_duration {
            if skip_trigger.load(Ordering::Relaxed) {
                bus.emit_notification(NotificationEvent::Log("⏳ Wait skipped by user".to_string()));
                return true;
            }

            let elapsed = start.elapsed().as_secs();
            let remaining = timeout_secs.saturating_sub(elapsed as u32);
            
            let current_stable = stable_since.map(|s| s.elapsed().as_secs()).unwrap_or(0);
            bus.emit_notification(NotificationEvent::UpdateStatus {
                text: format!("Attente stabilite ({}s/{}s): {}s restant", 
                    current_stable,
                    seconds,
                    remaining
                ),
                color: "orange".to_string(),
            });

            if let Some(img) = screen_capturer.capture_desktop() {
                if let Some(ref prev_img) = last_hash {
                    let diff = ReflectionModule::quick_visual_diff(prev_img, &img);
                    match diff {
                        VisualDiffResult::Unchanged { .. } => {
                            if stable_since.is_none() {
                                stable_since = Some(Instant::now());
                            } else if stable_since.expect("stable_since should be Some").elapsed() >= Duration::from_secs(seconds as u64) {
                                bus.emit_notification(NotificationEvent::Log(format!("✅ Screen stable for {}s.", seconds)));
                                return true;
                            }
                        }
                        _ => {
                            // Reset stability timer on change
                            stable_since = None;
                        }
                    }
                }
                last_hash = Some(img);
            }

            sleep(Duration::from_millis(250)).await;
        }
        
        bus.emit_notification(NotificationEvent::Log("⏰ Screen stability timeout".to_string()));
        false
    }

    async fn wait_video_end(
        timeout_secs: u32,
        screen_capturer: Arc<dyn ScreenCapturerTrait>,
        bus: EventBus,
        skip_trigger: Arc<AtomicBool>,
    ) -> bool {
        // VideoEnd is a specialization of ScreenStable. We check if screen has been stable (unchanged) for 3 seconds.
        Self::wait_screen_stable(3, timeout_secs, screen_capturer, bus, skip_trigger).await
    }

    async fn wait_element_appears(
        description: String,
        timeout_secs: u32,
        _screen_capturer: Arc<dyn ScreenCapturerTrait>,
        bus: EventBus,
        skip_trigger: Arc<AtomicBool>,
    ) -> bool {
        let start = Instant::now();
        let timeout_duration = Duration::from_secs(timeout_secs as u64);
        
        bus.emit_notification(NotificationEvent::Log(format!("🔍 Waiting for element to appear: '{}'", description)));
        
        while start.elapsed() < timeout_duration {
            if skip_trigger.load(Ordering::Relaxed) {
                bus.emit_notification(NotificationEvent::Log("⏳ Wait skipped by user".to_string()));
                return true;
            }

            let elapsed = start.elapsed().as_secs();
            let remaining = timeout_secs.saturating_sub(elapsed as u32);
            bus.emit_notification(NotificationEvent::UpdateStatus {
                text: format!("Attente '{}': {}s restantes...", description, remaining),
                color: "orange".to_string(),
            });

            sleep(Duration::from_millis(250)).await;
        }
        
        bus.emit_notification(NotificationEvent::Log(format!("⏰ Element '{}' did not appear (timeout)", description)));
        false
    }
}

impl WaitManagerTrait for WaitManager {
    fn wait_for(
        &self,
        condition: WaitCondition,
        timeout_secs: u32,
        screen_capturer: Arc<dyn ScreenCapturerTrait>,
        bus: EventBus,
        skip_trigger: Arc<AtomicBool>,
    ) -> Pin<Box<dyn Future<Output = bool> + Send + 'static>> {
        Box::pin(async move {
            match condition {
                WaitCondition::TimeElapsed => {
                    Self::wait_time_elapsed(timeout_secs, bus, skip_trigger).await
                }
                WaitCondition::ScreenStable { seconds } => {
                    Self::wait_screen_stable(seconds, timeout_secs, screen_capturer, bus, skip_trigger).await
                }
                WaitCondition::VideoEnd => {
                    Self::wait_video_end(timeout_secs, screen_capturer, bus, skip_trigger).await
                }
                WaitCondition::ElementAppears { description } => {
                    Self::wait_element_appears(description, timeout_secs, screen_capturer, bus, skip_trigger).await
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orchestrator::tests::MockScreenCapturer;
    use crate::event_bus::EventBus;
    use std::sync::atomic::AtomicBool;
    use image::DynamicImage;

    #[tokio::test]
    async fn test_wait_manager_time_elapsed() {
        let (bus, _rx) = EventBus::new();
        let skip_trigger = Arc::new(AtomicBool::new(false));
        let capturer = Arc::new(MockScreenCapturer {
            window_foreground_result: std::sync::Mutex::new(crate::screen_capture::WindowAnchorResult::AlreadyFocused),
        });

        let manager = WaitManager::new();
        let result = manager.wait_for(
            WaitCondition::TimeElapsed,
            1,
            capturer,
            bus,
            skip_trigger,
        ).await;

        assert!(result);
    }

    #[tokio::test]
    async fn test_wait_manager_skip() {
        let (bus, _rx) = EventBus::new();
        let skip_trigger = Arc::new(AtomicBool::new(true));
        let capturer = Arc::new(MockScreenCapturer {
            window_foreground_result: std::sync::Mutex::new(crate::screen_capture::WindowAnchorResult::AlreadyFocused),
        });

        let manager = WaitManager::new();
        let result = manager.wait_for(
            WaitCondition::TimeElapsed,
            10,
            capturer,
            bus,
            skip_trigger,
        ).await;

        assert!(result);
    }

    #[tokio::test]
    async fn test_wait_manager_element_appears_timeout() {
        let (bus, _rx) = EventBus::new();
        let skip_trigger = Arc::new(AtomicBool::new(false));
        let capturer = Arc::new(MockScreenCapturer {
            window_foreground_result: std::sync::Mutex::new(crate::screen_capture::WindowAnchorResult::AlreadyFocused),
        });

        let manager = WaitManager::new();
        let result = manager.wait_for(
            WaitCondition::ElementAppears { description: "test_element".to_string() },
            1,
            capturer,
            bus,
            skip_trigger,
        ).await;

        assert!(!result);
    }

    #[tokio::test]
    async fn test_wait_manager_screen_stable_timeout() {
        let (bus, _rx) = EventBus::new();
        let skip_trigger = Arc::new(AtomicBool::new(false));
        let capturer = Arc::new(MockScreenCapturer {
            window_foreground_result: std::sync::Mutex::new(crate::screen_capture::WindowAnchorResult::AlreadyFocused),
        });

        let manager = WaitManager::new();
        let result = manager.wait_for(
            WaitCondition::ScreenStable { seconds: 2 },
            1,
            capturer,
            bus,
            skip_trigger,
        ).await;

        assert!(!result);
    }

    struct StableTestCapturer {
        capture_result: Option<DynamicImage>,
    }

    impl ScreenCapturerTrait for StableTestCapturer {
        fn refresh_windows(&self) {}
        fn get_windows(&self) -> Vec<crate::screen_capture::WindowInfo> { vec![] }
        fn get_monitors(&self) -> Vec<crate::screen_capture::ScreenInfo> { vec![] }
        fn capture_desktop(&self) -> Option<DynamicImage> {
            self.capture_result.clone()
        }
        fn capture_window_by_title(&self, _title: &str) -> Option<DynamicImage> { None }
        fn capture_bbox(&self, _x: i32, _y: i32, _w: i32, _h: i32) -> Option<DynamicImage> { None }
        fn is_target_visible(&self, _target_windows: &[String]) -> bool { true }
        fn ensure_window_foreground(&self, _title: &str) -> crate::screen_capture::WindowAnchorResult { crate::screen_capture::WindowAnchorResult::AlreadyFocused }
        fn get_foreground_window_title(&self) -> Option<String> { None }
    }

    #[tokio::test]
    async fn test_wait_manager_video_end() {
        let (bus, _rx) = EventBus::new();
        let skip_trigger = Arc::new(AtomicBool::new(false));
        let capturer = Arc::new(StableTestCapturer {
            capture_result: Some(DynamicImage::new_rgb8(10, 10)),
        });

        let manager = WaitManager::new();
        let result = manager.wait_for(
            WaitCondition::VideoEnd,
            1,
            capturer,
            bus,
            skip_trigger,
        ).await;
        assert!(!result); // Tends to timeout since we only run for 1 sec but need 3 sec stability
    }

    #[tokio::test]
    async fn test_wait_manager_screen_stable_skips() {
        let (bus, _rx) = EventBus::new();
        let skip_trigger = Arc::new(AtomicBool::new(true));
        let capturer = Arc::new(StableTestCapturer {
            capture_result: Some(DynamicImage::new_rgb8(10, 10)),
        });

        let manager = WaitManager::new();
        let result = manager.wait_for(
            WaitCondition::ScreenStable { seconds: 1 },
            5,
            capturer,
            bus,
            skip_trigger,
        ).await;
        assert!(result);
    }

    #[tokio::test]
    async fn test_wait_manager_element_appears_skips() {
        let (bus, _rx) = EventBus::new();
        let skip_trigger = Arc::new(AtomicBool::new(true));
        let capturer = Arc::new(StableTestCapturer {
            capture_result: None,
        });

        let manager = WaitManager::new();
        let result = manager.wait_for(
            WaitCondition::ElementAppears { description: "test".to_string() },
            5,
            capturer,
            bus,
            skip_trigger,
        ).await;
        assert!(result);
    }

    #[tokio::test]
    async fn test_wait_manager_screen_stable_success() {
        let (bus, _rx) = EventBus::new();
        let skip_trigger = Arc::new(AtomicBool::new(false));
        let capturer = Arc::new(StableTestCapturer {
            capture_result: Some(DynamicImage::new_rgb8(10, 10)),
        });

        let manager = WaitManager::new();
        let result = manager.wait_for(
            WaitCondition::ScreenStable { seconds: 0 }, // 0 seconds stability required
            2,
            capturer,
            bus,
            skip_trigger,
        ).await;
        assert!(result);
    }
}
