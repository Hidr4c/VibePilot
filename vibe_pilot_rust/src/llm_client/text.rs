use std::future::Future;
use std::pin::Pin;
use super::types::{LlmRequest, Message, MessageContent};
use super::{LlmClient, LlmTextProvider, strip_markdown_code_blocks};

impl LlmTextProvider for LlmClient {
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
    ) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send + 'a>> {
        Box::pin(async move {
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
                .ok_or("No content in response")?;

            let content = strip_markdown_code_blocks(content);

            Ok(content)
        })
    }

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
    ) -> Pin<Box<dyn Future<Output = Result<serde_json::Value, String>> + Send + 'a>> {
        Box::pin(async move {
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
                max_tokens: Some(2048),
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

            serde_json::from_str(&content).map_err(|e| format!("JSON parse error: {}", e))
        })
    }

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
    ) -> Pin<Box<dyn Future<Output = Result<serde_json::Value, String>> + Send + 'a>> {
        Box::pin(async move {
            let system_prompt = "You are a master RPA workflow orchestrator. \
                Your task is to break down the user's automation request into a Directed Acyclic Graph (DAG) of parallel/dependent VibePilot agent nodes. \
                Output ONLY a valid JSON array of objects representing nodes, with no markdown formatting, comments, or extra text. \
                Each node object in the array must contain:\n\
                - \"id\": a unique alphanumeric string (e.g. \"node_1\")\n\
                - \"name\": a friendly name (e.g. \"1. Open Chrome\")\n\
                - \"profile_name\": the target configuration profile to launch\n\
                - \"worker_url\": the address of the worker client (default to \"http://127.0.0.1:4040\")\n\
                - \"dependencies\": a JSON array of strings representing node IDs that must be completed successfully before this node starts\n\
                - \"duration_secs\": estimated execution duration (as a float, e.g. 15.0)\n\n\
                Example format:\n\
                [\n\
                  {\"id\": \"node_1\", \"name\": \"1. Fetch Data\", \"profile_name\": \"Fetch Excel\", \"worker_url\": \"http://127.0.0.1:4040\", \"dependencies\": [], \"duration_secs\": 20.0},\n\
                  {\"id\": \"node_2\", \"name\": \"2. Format Sheet\", \"profile_name\": \"Excel Formatter\", \"worker_url\": \"http://127.0.0.1:4040\", \"dependencies\": [\"node_1\"], \"duration_secs\": 15.0}\n\
                ]";

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
                            text: Some(format!("Create a dependent grid task graph for: {}", prompt)),
                            image_url: None,
                        }],
                    },
                ],
                temperature: 0.2,
                max_tokens: Some(2048),
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

            serde_json::from_str(&content).map_err(|e| format!("JSON parse error: {}", e))
        })
    }

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
    ) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send + 'a>> {
        Box::pin(async move {
            let system_prompt = "You are an expert project planner and coordinator. \
                Your job is to break down the user's objective into a list of clear, sequential, and logical sub-tasks. \
                Output ONLY a valid JSON array of objects, with no other text, comments or markdown formatting (unless in a ```json code block). \
                Each object in the array must have the following fields:\n\
                - \"id\": a unique sequential integer starting at 1\n\
                - \"description\": a brief description of the sub-task in technical English or localized French if appropriate\n\
                - \"depends_on\": a JSON array of integers representing the IDs of tasks that must be completed BEFORE this task can start (dependencies).\n\n\
                Example output format:\n\
                [\n\
                  {\"id\": 1, \"description\": \"Open VS Code\", \"depends_on\": []},\n\
                  {\"id\": 2, \"description\": \"Open workspace folder\", \"depends_on\": [1]},\n\
                  {\"id\": 3, \"description\": \"Run the test suite\", \"depends_on\": [2]}\n\
                ]";

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
                            text: Some(format!(
                                "Context: {}\nTask instructions: {}\nDecompose this objective into a JSON task graph: {}",
                                contexte, task, objectif
                            )),
                            image_url: None,
                        }],
                    },
                ],
                temperature: 0.2,
                max_tokens: Some(1536),
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
        })
    }

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
    ) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send + 'a>> {
        Box::pin(async move {
            let system_prompt = if langue == "Français" {
                "Vous êtes un assistant IA chargé de maintenir le résumé historique des actions d'un agent d'automatisation. \
                 Résumez de manière extrêmement compacte (2-3 phrases maximum, sans détails inutiles comme les timestamps ou les coordonnées précises) les nouvelles actions de l'agent et fusionnez-les avec le résumé précédent de façon cohérente."
            } else {
                "You are an AI assistant tasked with keeping a running summary of an automation agent's past actions. \
                 Synthesize the new actions very compactly (2-3 sentences max, omitting coordinates or timestamp details) and merge them logically with the previous history summary."
            };

            let user_prompt = if previous_summary.is_empty() {
                if langue == "Français" {
                    format!("Nouvelles actions à résumer :\n{}\n\nRésumé :", old_steps_text)
                } else {
                    format!("New actions to summarize:\n{}\n\nSummary:", old_steps_text)
                }
            } else {
                if langue == "Français" {
                    format!("Résumé précédent :\n{}\n\nNouvelles actions à fusionner :\n{}\n\nNouveau résumé fusionné :", previous_summary, old_steps_text)
                } else {
                    format!("Previous summary:\n{}\n\nNew actions to merge:\n{}\n\nNew merged summary:", previous_summary, old_steps_text)
                }
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
                            text: Some(user_prompt),
                            image_url: None,
                        }],
                    },
                ],
                temperature: 0.2,
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
                .ok_or_else(|| "No content in response".to_string())?;

            let content = strip_markdown_code_blocks(content);

            Ok(content.trim().to_string())
        })
    }
}
