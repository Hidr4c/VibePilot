use std::future::Future;
use std::pin::Pin;
use image::DynamicImage;

pub mod types;
pub mod vision;
pub mod text;
pub mod metadata;
pub mod factory;

#[cfg(test)]
pub mod tests;

pub use types::{LlmResponse, WaitCondition};
pub use factory::LlmClientFactory;

// ─── Sub-traits (Interface Segregation) ───────────────────────────────────────

/// Vision-related operations: analyze screenshots and identify regions of interest.
#[allow(clippy::too_many_arguments)]
pub trait LlmVisionProvider: Send + Sync {
    fn execute_decision<'a>(
        &'a self,
        image: &'a DynamicImage,
        contexte: &'a str,
        objectif: &'a str,
        task: &'a str,
        directives: &'a str,
        user_feedback: &'a str,
        url: &'a str,
        model: &'a str,
        auth_mode: &'a str,
        auth_api_key: &'a str,
        auth_login: &'a str,
        auth_password: &'a str,
        timeout_secs: u64,
    ) -> Pin<Box<dyn Future<Output = Result<LlmResponse, String>> + Send + 'a>>;

    fn identify_roi<'a>(
        &'a self,
        image: &'a DynamicImage,
        contexte: &'a str,
        objectif: &'a str,
        task: &'a str,
        url: &'a str,
        model: &'a str,
        auth_mode: &'a str,
        auth_api_key: &'a str,
        auth_login: &'a str,
        auth_password: &'a str,
        timeout_secs: u64,
    ) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send + 'a>>;
}

/// Text processing operations: optimize prompts, generate configs, decompose objectives, compress history.
#[allow(clippy::too_many_arguments)]
pub trait LlmTextProvider: Send + Sync {
    fn optimize_field<'a>(
        &'a self,
        text: &'a str,
        field_type: &'a str,
        url: &'a str,
        model: &'a str,
        auth_mode: &'a str,
        auth_api_key: &'a str,
        auth_login: &'a str,
        auth_password: &'a str,
        timeout_secs: u64,
    ) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send + 'a>>;

    fn generate_config<'a>(
        &'a self,
        user_request: &'a str,
        url: &'a str,
        model: &'a str,
        auth_mode: &'a str,
        auth_api_key: &'a str,
        auth_login: &'a str,
        auth_password: &'a str,
        timeout_secs: u64,
    ) -> Pin<Box<dyn Future<Output = Result<serde_json::Value, String>> + Send + 'a>>;

    fn compile_dag<'a>(
        &'a self,
        prompt: &'a str,
        url: &'a str,
        model: &'a str,
        auth_mode: &'a str,
        auth_api_key: &'a str,
        auth_login: &'a str,
        auth_password: &'a str,
        timeout_secs: u64,
    ) -> Pin<Box<dyn Future<Output = Result<serde_json::Value, String>> + Send + 'a>>;

    fn decompose_objective<'a>(
        &'a self,
        objectif: &'a str,
        contexte: &'a str,
        task: &'a str,
        url: &'a str,
        model: &'a str,
        auth_mode: &'a str,
        auth_api_key: &'a str,
        auth_login: &'a str,
        auth_password: &'a str,
        timeout_secs: u64,
    ) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send + 'a>>;

    fn compress_history<'a>(
        &'a self,
        old_steps_text: &'a str,
        previous_summary: &'a str,
        langue: &'a str,
        url: &'a str,
        model: &'a str,
        auth_mode: &'a str,
        auth_api_key: &'a str,
        auth_login: &'a str,
        auth_password: &'a str,
        timeout_secs: u64,
    ) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send + 'a>>;
}

/// Metadata operations: health checks and model discovery.
pub trait LlmMetadataProvider: Send + Sync {
    fn is_engine_busy<'a>(&'a self, url: &'a str, model: &'a str) -> Pin<Box<dyn Future<Output = bool> + Send + 'a>>;

    fn fetch_models<'a>(
        &'a self,
        url: &'a str,
        auth_mode: &'a str,
        auth_api_key: &'a str,
        auth_login: &'a str,
        auth_password: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<String>, String>> + Send + 'a>>;
}

/// Composite trait extending all sub-traits.
pub trait LlmProvider: LlmVisionProvider + LlmTextProvider + LlmMetadataProvider + Send + Sync {}

// Blanket impl: anything that implements all 3 sub-traits auto-implements LlmProvider
impl<T: LlmVisionProvider + LlmTextProvider + LlmMetadataProvider> LlmProvider for T {}

pub struct LlmClient {
    client: reqwest::Client,
}

impl Default for LlmClient {
    fn default() -> Self { Self::new() }
}

impl LlmClient {
    pub fn new() -> Self {
        LlmClient {
            client: reqwest::Client::new(),
        }
    }

    pub fn apply_auth_and_timeout(
        &self,
        mut builder: reqwest::RequestBuilder,
        auth_mode: &str,
        auth_api_key: &str,
        auth_login: &str,
        auth_password: &str,
        timeout_secs: u64,
    ) -> reqwest::RequestBuilder {
        builder = builder.timeout(std::time::Duration::from_secs(timeout_secs));
        match auth_mode {
            "api_key" if !auth_api_key.is_empty() => {
                builder = builder.header("Authorization", format!("Bearer {}", auth_api_key));
            }
            "basic_auth" if !auth_login.is_empty() || !auth_password.is_empty() => {
                builder = builder.basic_auth(auth_login, Some(auth_password));
            }
            _ => {}
        }
        builder
    }
}

pub fn strip_markdown_code_blocks(s: &str) -> String {
    let mut s = s.trim().to_string();
    if s.starts_with("```") {
        if let Some(first_newline) = s.find('\n') {
            s = s[first_newline..].trim().to_string();
        }
        if s.ends_with("```") {
            s = s[..s.len() - 3].trim().to_string();
        }
    }
    s
}

/// Determines whether a zoomed crop is needed for the given LLM response.
///
/// Zoom is triggered when the LLM's confidence is low, the target region
/// is very small, or the local image entropy is high.
pub fn should_zoom(confidence: f32, target_size_px: u32, entropy: Option<f64>) -> bool {
    let low_confidence = confidence < 0.6;
    let small_target = target_size_px < 200;
    let high_entropy = entropy.map_or(false, |e| e > 4.0);
    low_confidence || small_target || high_entropy
}
