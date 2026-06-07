//! LLM API client for VibePilot.
//!
//! Handles communication with local LLM servers (LM Studio, Ollama, etc.)
//! for vision-based decision making.

use serde::{Deserialize, Serialize};
use base64::Engine;
use image::DynamicImage;
use std::io::Cursor;

/// API response structure from the LLM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmResponse {
    pub status_display: String,
    pub action: String,
    pub relative_click_position: Vec<f64>,
    pub text_to_type: String,
    pub scroll_value: i32,
    pub wait_seconds: i32,
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

    /// Check if the engine is busy (health check).
    pub async fn is_engine_busy(&self, url: &str, _model: &str) -> bool {
        let test_url = if url.ends_with("/chat/completions") {
            url.trim_end_matches("/chat/completions").to_owned() + "/models"
        } else {
            url.to_string()
        };

        match self.client.get(test_url).timeout(std::time::Duration::from_secs(2)).send().await {
            Ok(resp) => resp.status().is_success(),
            Err(_) => true, // busy or unreachable
        }
    }

    /// Execute a decision protocol: send screenshot + prompts to the LLM.
    pub async fn execute_decision(
        &self,
        image: &DynamicImage,
        contexte: &str,
        objectif: &str,
        task: &str,
        directives: &str,
        url: &str,
        model: &str,
    ) -> Result<LlmResponse, String> {
        // Encode image to base64
        let mut buffer = Vec::new();
        image
            .to_rgba8()
            .write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Png)
            .map_err(|e| format!("Image encoding error: {}", e))?;

        let img_base64 = base64::engine::general_purpose::STANDARD.encode(&buffer);

        let prompt = format!(
            "You are the visual operating mind of this PC (Visual OS Orchestrator).\n\
             Situation Context: '{}'\n\
             Global user task: '{}'\n\
             Stop condition (Do while): '{}'\n\n\
             Analyze the attached screenshot of the target application with extreme precision.\n\n\
             CRITICAL OPERATIONAL COMPREHENSION RULES:\n\
             {}\n\n\
             You must respond EXCLUSIVELY with a strict JSON object:\n\
             {{\n\
               \"status_display\": \"short summary\",\n\
               \"action\": \"CLICK_AND_TYPE\" | \"SCROLL\" | \"WAIT\" | \"SUCCESS\" | \"FAIL\",\n\
               \"relative_click_position\": [0.5, 0.5],\n\
               \"text_to_type\": \"\",\n\
               \"scroll_value\": -6,\n\
               \"wait_seconds\": 15\n\
             }}",
            contexte, objectif, task, directives
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

        let response = self
            .client
            .post(url)
            .header("Content-Type", "application/json")
            .json(&request)
            .timeout(std::time::Duration::from_secs(60))
            .send()
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

        // Parse JSON response
        let response: LlmResponse = serde_json::from_str(content)
            .map_err(|e| format!("LLM response parse error: {}", e))?;

        Ok(response)
    }

    /// Optimize a single prompt field.
    pub async fn optimize_field(
        &self,
        text: &str,
        field_type: &str,
        url: &str,
        model: &str,
    ) -> Result<String, String> {
        let system_prompt = match field_type {
            "contexte" => crate::content::PROMPT_OPTIMIZE_CONTEXT,
            "objectif" => crate::content::PROMPT_OPTIMIZE_OBJECTIF,
            "task" => crate::content::PROMPT_OPTIMIZE_TASK,
            "directives" => crate::content::PROMPT_OPTIMIZE_DIRECTIVES,
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

        let response = self
            .client
            .post(url)
            .header("Content-Type", "application/json")
            .json(&request)
            .timeout(std::time::Duration::from_secs(40))
            .send()
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

        let response = self
            .client
            .post(url)
            .header("Content-Type", "application/json")
            .json(&request)
            .timeout(std::time::Duration::from_secs(50))
            .send()
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
            "http://127.0.0.1:65535/chat/completions",
            "model"
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
            "model"
        ).await;
        assert!(res.is_err());
    }

    #[tokio::test]
    async fn test_generate_config_unreachable() {
        let client = LlmClient::new();
        let res = client.generate_config(
            "user request",
            "http://127.0.0.1:65535/chat/completions",
            "model"
        ).await;
        assert!(res.is_err());
    }
}
