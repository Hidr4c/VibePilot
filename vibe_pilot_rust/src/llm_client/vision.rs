use std::future::Future;
use std::pin::Pin;
use std::io::Cursor;
use image::DynamicImage;
use super::types::{LlmRequest, Message, MessageContent, ImageUrl, LlmResponse};
use super::{LlmClient, LlmVisionProvider, strip_markdown_code_blocks};

impl LlmVisionProvider for LlmClient {
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
    ) -> Pin<Box<dyn Future<Output = Result<LlmResponse, String>> + Send + 'a>> {
        Box::pin(async move {
            let mut buffer = Vec::new();
            image
                .to_rgba8()
                .write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Png)
                .map_err(|e| format!("Image encoding error: {}", e))?;

            let img_base64 = custom_base64_encode(&buffer);

            let prompt = crate::prompt_templates::execute_decision_prompt(
                contexte,
                objectif,
                task,
                directives,
                user_feedback,
                "English",
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
                max_tokens: Some(1024),
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

            let content = strip_markdown_code_blocks(content);

            let response: LlmResponse = serde_json::from_str(&content)
                .map_err(|e| format!("LLM response parse error: {} (raw: {})", e, &content[..content.len().min(200)]))?;

            if response.action.is_empty() {
                return Err("LLM response missing 'action' field".to_string());
            }

            let valid_actions = [
                "CLICK_AND_TYPE", "SCROLL", "WAIT", "SUCCESS", "FAIL",
                "KEY_COMBO", "CLIPBOARD", "DRAG_DROP", "RIGHT_CLICK",
                "DOUBLE_CLICK", "MIDDLE_CLICK", "MOUSE_MOVE_RELATIVE",
                "SHORTCUT", "TYPE_WITH_DELAY", "SMOOTH_SCROLL", "KEY_HOLD", "KEY_RELEASE"
            ];
            if !valid_actions.contains(&response.action.as_str()) {
                return Err(format!("LLM returned unknown action: '{}'. Expected one of: {:?}", response.action, valid_actions));
            }

            Ok(response)
        })
    }

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
    ) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send + 'a>> {
        Box::pin(async move {
            let mut buffer = Vec::new();
            image
                .to_rgba8()
                .write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Png)
                .map_err(|e| format!("Image encoding error: {}", e))?;

            let img_base64 = custom_base64_encode(&buffer);

            let system_prompt = "You are an expert computer vision model. \
                Your job is to identify a region of interest (ROI) on the screen containing the interactive elements needed to advance the current task. \
                Output ONLY a valid JSON object representing the region of interest. No explanations, no markdown (unless in a ```json code block). \
                The JSON object must have these exact fields:\n\
                - \"x\": relative horizontal center of the region (0.0 to 1.0)\n\
                - \"y\": relative vertical center of the region (0.0 to 1.0)\n\
                - \"width\": relative width of the region (0.1 to 0.5 recommended)\n\
                - \"height\": relative height of the region (0.1 to 0.5 recommended)\n\
                - \"label\": brief label of what this region contains\n\n\
                Example JSON output:\n\
                {\n  \"x\": 0.35,\n  \"y\": 0.72,\n  \"width\": 0.25,\n  \"height\": 0.15,\n  \"label\": \"compose window send button\"\n}";

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
                        content: vec![
                            MessageContent {
                                content_type: "text".to_string(),
                                text: Some(format!(
                                    "Context: {}\nTask: {}\nObjective: {}\nIdentify the region containing the next target element.",
                                    contexte, task, objectif
                                )),
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
                    },
                ],
                temperature: 0.1,
                max_tokens: Some(512),
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

            Ok(content.to_string())
        })
    }
}

fn custom_base64_encode(input: &[u8]) -> String {
    const CHARSET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::with_capacity((input.len() + 2) / 3 * 4);
    
    let mut i = 0;
    while i < input.len() {
        let b0 = input[i] as usize;
        let b1 = if i + 1 < input.len() { input[i + 1] as usize } else { 0 };
        let b2 = if i + 2 < input.len() { input[i + 2] as usize } else { 0 };
        
        let val = (b0 << 16) | (b1 << 8) | b2;
        
        let c0 = CHARSET[(val >> 18) & 63] as char;
        let c1 = CHARSET[(val >> 12) & 63] as char;
        let c2 = if i + 1 < input.len() { CHARSET[(val >> 6) & 63] as char } else { '=' };
        let c3 = if i + 2 < input.len() { CHARSET[val & 63] as char } else { '=' };
        
        result.push(c0);
        result.push(c1);
        result.push(c2);
        result.push(c3);
        
        i += 3;
    }
    result
}
