use std::sync::Arc;
use super::{ScreenCapturerTrait, gdi::ScreenCapturer, mock::MockScreenCapturer};

/// Factory to construct screen capture implementations following the IoC pattern.
pub struct ScreenCapturerFactory;

impl ScreenCapturerFactory {
    /// Creates the appropriate ScreenCapturerTrait implementation.
    pub fn create() -> Arc<dyn ScreenCapturerTrait> {
        #[cfg(target_os = "windows")]
        {
            Arc::new(ScreenCapturer::new())
        }
        #[cfg(not(target_os = "windows"))]
        {
            Arc::new(MockScreenCapturer::new())
        }
    }
}
