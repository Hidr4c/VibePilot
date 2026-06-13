use std::sync::Arc;
use crate::screen_capture::ScreenCapturerTrait;
use super::{PeripheralInput, concrete::PeripheralController};

/// Factory to construct peripheral controller implementations following the IoC pattern.
pub struct PeripheralControllerFactory;

impl PeripheralControllerFactory {
    /// Creates the appropriate PeripheralInput implementation.
    pub fn create(capturer: Arc<dyn ScreenCapturerTrait>) -> Arc<dyn PeripheralInput> {
        // Concrete is cross-platform since it internally wraps rdev for macOS/Linux,
        // and uses native win32 functions on Windows. In the future, platform-specific
        // subclasses (e.g. MacOsPeripheralController, LinuxPeripheralController) can be
        // bound here.
        Arc::new(PeripheralController::new(capturer))
    }
}
