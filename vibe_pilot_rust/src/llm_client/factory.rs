use std::sync::Arc;
use super::{LlmProvider, LlmClient};

/// Factory to construct LLM client provider implementations following the IoC pattern.
pub struct LlmClientFactory;

impl LlmClientFactory {
    /// Creates the default LlmProvider implementation.
    pub fn create() -> Arc<dyn LlmProvider> {
        Arc::new(LlmClient::new())
    }
}
