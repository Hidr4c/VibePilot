//! LLM API client for VibePilot.
//!
//! Handles communication with local LLM servers (LM Studio, Ollama, etc.)
//! for vision-based decision making.
//!
//! # Features
//!
//! - Vision-based decision protocol: sends screenshots + prompts to LLM
//! - Prompt field optimization: rewrites individual fields in technical English
//! - Full configuration generation: creates complete orchestrator config from user request
//! - Health check: verifies engine availability via HTTP
//!
//! # Error Handling
//!
//! All public methods return `Result<T, String>` with descriptive error messages.
//! HTTP errors, JSON parse errors, and missing response fields are all caught.

use serde::{Deserialize, Serialize};
use base64::Engine;
use image::DynamicImage;
use std::io::Cursor;

/// API response structure from the LLM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmResponse {
    pub status_display: String,
    pub action: String,
    #[serde(default)]
    pub relative_click_position: Vec<f64>,
    #[serde(default)]
    pub text_to_type: String,
    #[serde(default)]
    pub scroll_value: i32,
    #[serde(default)]
    pub wait_seconds: i32,
    #[serde(default)]
    pub report: Option<String>,
}

/// Request payload for the LLM API.
#[derive(Debug, Serialize)]
struct LlmRequest {
    model: String,
    messages: Vec<Message>,
    temperature: f64,
}

#[derive(Debug, Serialize)]
struct Message {
    role: String,
    content: Vec<MessageContent>,
}

#[derive(Debug, Serialize)]
struct MessageContent {
    #[serde(rename = "type")]
    content_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    image_url: Option<ImageUrl>,
}

#[derive(Debug, Serialize)]
struct ImageUrl {
    url: String,
}

/// LLM API client.
pub struct LlmClient {
    client: reqwest::Client,
}

impl LlmClient {
    pub fn new() -> Self {
        LlmClient {
            client: reqwest::Client::new(),
        }
    }

    fn apply_auth_and_timeout(
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
            "api_key" => {
                if !auth_api_key.is_empty() {
                    builder = builder.header("Authorization", format!("Bearer {}", auth_api_key));
                }
            }
            "basic_auth" => {
                if !auth_login.is_empty() || !auth_password.is_empty() {
                    builder = builder.basic_auth(auth_login, Some(auth_password));
                }
            }
            _ => {}
        }
        builder
    }

    /// Check if the engine is busy (health check).
    pub async fn is_engine_busy(&self, url: &str, _model: &str) -> bool {
        let test_url = if url.ends_with("/chat/completions") {
            url.trim_end_matches("/chat/completions").to_owned() + "/models"
        } else {
            url.to_string()
        };

        match self.client.get(test_url).timeout(std::time::Duration::from_secs(2)).send().await {
            Ok(resp) => !resp.status().is_success(),
            Err(_) => true, // busy or unreachable
        }
    }

    /// Fetch list of available models from the engine endpoint.
    pub async fn fetch_models(
        &self,
        url: &str,
        auth_mode: &str,
        auth_api_key: &str,
        auth_login: &str,
        auth_password: &str,
    ) -> Result<Vec<String>, String> {
        let test_url = if url.ends_with("/chat/completions") {
            url.trim_end_matches("/chat/completions").to_owned() + "/models"
        } else if url.ends_with("/v1") {
            url.to_string() + "/models"
        } else {
            if url.ends_with('/') {
                url.to_string() + "v1/models"
            } else {
                url.to_string() + "/v1/models"
            }
        };

        let req = self.client.get(test_url);
        let req = self.apply_auth_and_timeout(req, auth_mode, auth_api_key, auth_login, auth_password, 10);
        let resp = req.send()
            .await
            .map_err(|e| format!("Connection error: {}", e))?;

        if !resp.status().is_success() {
            return Err(format!("HTTP error: {}", resp.status()));
        }

        let json: serde_json::Value = resp.json()
            .await
            .map_err(|e| format!("JSON parse error: {}", e))?;

        let mut models = Vec::new();
        if let Some(data) = json["data"].as_array() {
            for m in data {
                if let Some(id) = m["id"].as_str() {
                    models.push(id.to_string());
                }
            }
        }

        if models.is_empty() {
            return Err("No models found in response".to_string());
        }

        Ok(models)
    }

    /// Execute a decision protocol: send screenshot + prompts to the LLM.
    pub async fn execute_decision(
        &self,
        image: &DynamicImage,
        contexte: &str,
        objectif: &str,
        task: &str,
        directives: &str,
        user_feedback: &str,
        url: &str,
        model: &str,
        auth_mode: &str,
        auth_api_key: &str,
        auth_login: &str,
        auth_password: &str,
        timeout_secs: u64,
    ) -> Result<LlmResponse, String> {
        // Encode image to base64
        let mut buffer = Vec::new();
        image
            .to_rgba8()
            .write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Png)
            .map_err(|e| format!("Image encoding error: {}", e))?;

        let img_base64 = base64::engine::general_purpose::STANDARD.encode(&buffer);

        let feedback_prompt = if !user_feedback.is_empty() {
            format!("\nUSER INTERACTIVE HINT / DIRECTION:\n'{}'\n(CRITICAL: The user has directly entered this feedback or correction on the console logs. You MUST prioritize and execute based on this hint/direction, even if it contradicts previous instructions or seems counter-intuitive!)\n", user_feedback)
        } else {
            String::new()
        };

        let prompt = format!(
            "You are the visual operating mind of this PC (Visual OS Orchestrator).\n\
               Situation Context: '{}'\n\
               Global user task: '{}'\n\
               Stop condition (Do while): '{}'\n\
               Behavioral Directives:\n'{}'\n\
               {}\n\n\
               Analyze the attached screenshot of the target application with extreme precision.\n\n\
               You have access to mouse control actions. You must determine the absolute next step based on the screenshot, context, task, directives, and any user live feedback.\n\n\
               Coordinate Calibration & Alignment Instructions:\n\
               - The coordinates are relative to the screenshot image itself (0.0 to 1.0, or 0 to 1000, or raw pixel coordinates like 89.0).\n\
               - x = 0.0 is the left edge of the screenshot, x = 1.0 is the right edge. x = 0.5 is the exact center horizontally.\n\
               - y = 0.0 is the top edge, y = 1.0 is the bottom edge. y = 0.5 is the exact center vertically.\n\
               - If the target element is near the left edge (e.g. bookmarks bar on the left), x MUST be very low (e.g. 0.05 to 0.25). A value like 0.36 is more than one-third of the screen width and lies much further to the right.\n\
               - In the \"report\" field, you MUST write down a step-by-step calibration analysis before selecting the coordinates:\n\
                 1. Identify the target element's text/label.\n\
                 2. Estimate its approximate position as a percentage of the width (e.g., first quarter, middle, third quarter).\n\
                 3. Explain why the selected relative x coordinate matches that estimation.\n\
                 4. Check if the vertical y coordinate corresponds to the element's height zone.\n\n\
               Strict Output format: You must output ONLY a valid JSON object enclosed in double curly braces (or standard markdown json block) with the following fields:\n\
               - \"status_display\": brief text to show on status bar\n\
               - \"action\": one of \"CLICK_AND_TYPE\" (if you want to click on coordinate and optionally write/paste text), \"SCROLL\" (if you need to scroll), \"WAIT\" (if the app is loading/busy), \"SUCCESS\" (if the task is accomplished and verified), \"FAIL\" (if the goal is impossible to achieve or blocked)\n\
               - \"relative_click_position\": [x, y] coordinates (float between 0.0 and 1.0) relative to the screen/target area, or [0.0, 0.0] if not clicking\n\
               - \"text_to_type\": string to write/paste after click, or empty if not writing\n\
               - \"scroll_value\": positive integer to scroll up, negative to scroll down, or 0 if not scrolling\n\
               - \"wait_seconds\": integer (e.g. 5, 15) to pause after action, recommended 10-15s for stability\n\
               - \"report\": a mandatory detailed analysis report containing your visual calibration steps and logic.\n\n\
               Let's proceed.\n\n\
               JSON response:",
            contexte, objectif, task, directives, feedback_prompt
        );

        let request = LlmRequest {
            model: model.to_string(),
            messages: vec![Message {
                role: "user".to_string(),
                content: vec![
                    MessageContent {
                        content_type: "text".to_string(),
                        text: Some(prompt),
                        image_url: None,
                    },
                    MessageContent {
                        content_type: "image_url".to_string(),
                        text: None,
                        image_url: Some(ImageUrl {
                            url: format!("data:image/png;base64,{}", img_base64),
                        }),
                    },
                ],
            }],
            temperature: 0.1,
        };

        let req = self.client.post(url)
            .header("Content-Type", "application/json")
            .json(&request);
        let req = self.apply_auth_and_timeout(req, auth_mode, auth_api_key, auth_login, auth_password, timeout_secs);
        let response = req.send()
            .await
            .map_err(|e| format!("HTTP error: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("API error: {}", response.status()));
        }

        let body: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("JSON parse error: {}", e))?;

        let content = body["choices"][0]["message"]["content"]
            .as_str()
            .ok_or_else(|| "No content in response".to_string())?;

        // Strip markdown code blocks if present (common with LLM responses)
        let content = strip_markdown_code_blocks(content);

        // Parse JSON response with validation
        let response: LlmResponse = serde_json::from_str(&content)
            .map_err(|e| format!("LLM response parse error: {} (raw: {})", e, &content[..content.len().min(200)]))?;

        // Validate required fields
        if response.action.is_empty() {
            return Err("LLM response missing 'action' field".to_string());
        }

        let valid_actions = ["CLICK_AND_TYPE", "SCROLL", "WAIT", "SUCCESS", "FAIL"];
        if !valid_actions.contains(&response.action.as_str()) {
            return Err(format!("LLM returned unknown action: '{}'. Expected one of: {:?}", response.action, valid_actions));
        }

        Ok(response)
    }

    /// Optimize a single prompt field.
    pub async fn optimize_field(
        &self,
        text: &str,
        field_type: &str,
        url: &str,
        model: &str,
        auth_mode: &str,
        auth_api_key: &str,
        auth_login: &str,
        auth_password: &str,
        timeout_secs: u64,
    ) -> Result<String, String> {
        let system_prompt = match field_type {
            "contexte" => crate::content::PROMPT_OPTIMIZE_CONTEXT,
            "objectif" => crate::content::PROMPT_OPTIMIZE_OBJECTIF,
            "task" => crate::content::PROMPT_OPTIMIZE_TASK,
            "directives" => crate::content::PROMPT_OPTIMIZE_DIRECTIVES,
            "user_feedback" => crate::content::PROMPT_OPTIMIZE_FEEDBACK,
            _ => "You are an expert Prompt Engineer. Optimize this text.",
        };

        let request = LlmRequest {
            model: model.to_string(),
            messages: vec![
                Message {
                    role: "system".to_string(),
                    content: vec![MessageContent {
                        content_type: "text".to_string(),
                        text: Some(system_prompt.to_string()),
                        image_url: None,
                    }],
                },
                Message {
                    role: "user".to_string(),
                    content: vec![MessageContent {
                        content_type: "text".to_string(),
                        text: Some(format!("Optimize: {}", text)),
                        image_url: None,
                    }],
                },
            ],
            temperature: 0.3,
        };

        let req = self.client.post(url)
            .header("Content-Type", "application/json")
            .json(&request);
        let req = self.apply_auth_and_timeout(req, auth_mode, auth_api_key, auth_login, auth_password, timeout_secs);
        let response = req.send()
            .await
            .map_err(|e| format!("HTTP error: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("API error: {}", response.status()));
        }

        let body: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("JSON parse error: {}", e))?;

        let content = body["choices"][0]["message"]["content"]
            .as_str()
            .ok_or("No content in response")?;

        let content = strip_markdown_code_blocks(content);

        Ok(content)
    }

    /// Generate a complete configuration from a user request.
    pub async fn generate_config(
        &self,
        user_request: &str,
        url: &str,
        model: &str,
        auth_mode: &str,
        auth_api_key: &str,
        auth_login: &str,
        auth_password: &str,
        timeout_secs: u64,
    ) -> Result<serde_json::Value, String> {
        let request = LlmRequest {
            model: model.to_string(),
            messages: vec![
                Message {
                    role: "system".to_string(),
                    content: vec![MessageContent {
                        content_type: "text".to_string(),
                        text: Some(crate::content::PROMPT_GLOBAL_GENERATION.to_string()),
                        image_url: None,
                    }],
                },
                Message {
                    role: "user".to_string(),
                    content: vec![MessageContent {
                        content_type: "text".to_string(),
                        text: Some(format!(
                            "Create configuration for: {}",
                            user_request
                        )),
                        image_url: None,
                    }],
                },
            ],
            temperature: 0.3,
        };

        let req = self.client.post(url)
            .header("Content-Type", "application/json")
            .json(&request);
        let req = self.apply_auth_and_timeout(req, auth_mode, auth_api_key, auth_login, auth_password, timeout_secs);
        let response = req.send()
            .await
            .map_err(|e| format!("HTTP error: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("API error: {}", response.status()));
        }

        let body: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("JSON parse error: {}", e))?;

        let content = body["choices"][0]["message"]["content"]
            .as_str()
            .ok_or("No content in response")?;

        let content = strip_markdown_code_blocks(content);

        // Try to parse as JSON
        serde_json::from_str(&content).map_err(|e| format!("JSON parse error: {}", e))
    }
}

fn strip_markdown_code_blocks(s: &str) -> String {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_llm_response_deserialize() {
        let json = r#"{
            "status_display": "Test",
            "action": "CLICK_AND_TYPE",
            "relative_click_position": [0.5, 0.5],
            "text_to_type": "hello",
            "scroll_value": -6,
            "wait_seconds": 15
        }"#;

        let response: LlmResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.action, "CLICK_AND_TYPE");
        assert_eq!(response.text_to_type, "hello");
        assert_eq!(response.wait_seconds, 15);
    }

    #[test]
    fn test_llm_response_defaults() {
        let json = r#"{
            "status_display": "Test",
            "action": "WAIT",
            "relative_click_position": [0.0, 0.0],
            "text_to_type": "",
            "scroll_value": -6,
            "wait_seconds": 15
        }"#;

        let response: LlmResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.action, "WAIT");
        assert!(response.text_to_type.is_empty());
    }

    #[test]
    fn test_strip_markdown_code_blocks() {
        let input1 = "```json\n{\n  \"key\": \"value\"\n}\n```";
        assert_eq!(strip_markdown_code_blocks(input1), "{\n  \"key\": \"value\"\n}");

        let input2 = "```\nhello world\n```";
        assert_eq!(strip_markdown_code_blocks(input2), "hello world");

        let input3 = "hello world";
        assert_eq!(strip_markdown_code_blocks(input3), "hello world");
    }

    #[tokio::test]
    async fn test_is_engine_busy_unreachable() {
        let client = LlmClient::new();
        let is_busy = client.is_engine_busy("http://127.0.0.1:65535/chat/completions", "model").await;
        assert!(is_busy);
    }

    #[tokio::test]
    async fn test_execute_decision_unreachable() {
        let client = LlmClient::new();
        let image = image::DynamicImage::ImageRgba8(image::ImageBuffer::new(100, 100));
        let res = client.execute_decision(
            &image,
            "ctx",
            "obj",
            "task",
            "dirs",
            "",
            "http://127.0.0.1:65535/chat/completions",
            "model",
            "none",
            "",
            "",
            "",
            120
        ).await;
        assert!(res.is_err());
    }

    #[tokio::test]
    async fn test_optimize_field_unreachable() {
        let client = LlmClient::new();
        let res = client.optimize_field(
            "some text",
            "contexte",
            "http://127.0.0.1:65535/chat/completions",
            "model",
            "none",
            "",
            "",
            "",
            120
        ).await;
        assert!(res.is_err());
    }

    #[tokio::test]
    async fn test_generate_config_unreachable() {
        let client = LlmClient::new();
        let res = client.generate_config(
            "user request",
            "http://127.0.0.1:65535/chat/completions",
            "model",
            "none",
            "",
            "",
            "",
            120
        ).await;
        assert!(res.is_err());
    }
}
