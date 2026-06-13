use std::sync::Arc;
use super::wait_manager::{WaitManagerTrait, WaitManager};

/// Factory to construct WaitManager implementations following the IoC pattern.
pub struct WaitManagerFactory;

impl WaitManagerFactory {
    /// Creates the default WaitManagerTrait implementation.
    pub fn create() -> Arc<dyn WaitManagerTrait> {
        Arc::new(WaitManager::new())
    }
}
