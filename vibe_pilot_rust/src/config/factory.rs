use std::sync::Arc;
use std::path::PathBuf;
use super::{ConfigurationRepository, repository::ConfigRepository};

/// Factory to construct ConfigurationRepository implementations following the IoC pattern.
pub struct ConfigRepositoryFactory;

impl ConfigRepositoryFactory {
    /// Creates a ConfigurationRepository implementation with the specified base directory.
    pub fn create(base_dir: PathBuf) -> Arc<dyn ConfigurationRepository> {
        Arc::new(ConfigRepository::new(base_dir))
    }
}
