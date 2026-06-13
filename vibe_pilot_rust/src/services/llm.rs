//! LLM operations service.
//!
//! Handles all LLM-related async operations: optimizing fields,
//! generating configs, fetching models, and more.

use std::sync::Arc;
use tokio::runtime::Handle;
use crate::event_bus::{EventBus, NotificationEvent, CommandEvent};
use crate::llm_client::{LlmProvider, LlmMetadataProvider};

pub struct LlmService {
    llm_client: Arc<dyn LlmProvider>,
    rt: Handle,
    bus: EventBus,
}

impl LlmService {
    pub fn new(
        llm_client: Arc<dyn LlmProvider>,
        rt: Handle,
        bus: EventBus,
    ) -> Self {
        Self { llm_client, rt, bus }
    }

    pub fn optimize_prompt_field(
        &self,
        current_config: &crate::config::SavedConfig,
        field_type: &str,
        user_feedback: &str,
    ) {
        let url = current_config.url_api.clone();
        let model = current_config.nom_modele.clone();
        let auth_mode = current_config.auth_mode.clone();
        let auth_api_key = current_config.auth_api_key.clone();
        let auth_login = current_config.auth_login.clone();
        let auth_password = current_config.auth_password.clone();
        let timeout_secs = current_config.request_timeout_secs;
        let text = match field_type {
            "contexte" => current_config.contexte.clone(),
            "task" => current_config.task.clone(),
            "objectif" => current_config.objectif.clone(),
            "directives" => current_config.directives.clone(),
            "user_feedback" => user_feedback.to_string(),
            _ => return,
        };
        let field = field_type.to_string();
        let bus = self.bus.clone();
        let client = self.llm_client.clone();

        self.bus.emit_notification(NotificationEvent::Log(format!(
            "Sending Text LLM Request (Optimize Field) | Model: '{}' | Endpoint: '{}' | Field: '{}'",
            model, url, field
        )));

        self.rt.spawn(async move {
            match client.optimize_field(&text, &field, &url, &model, &auth_mode, &auth_api_key, &auth_login, &auth_password, timeout_secs).await {
                Ok(optimized) => {
                    bus.emit_notification(NotificationEvent::Log(format!("Received LLM Response (Optimize Field) | Success (Field '{}' optimized)", field)));
                    bus.emit_command(CommandEvent::UpdateField { field, value: optimized });
                }
                Err(e) => {
                    bus.emit_notification(NotificationEvent::Log(format!("LLM Error (Optimize Field) | Failed to optimize field '{}': {}", field, e)));
                }
            }
        });
    }

    pub fn generate_prompts_from_request(
        &self,
        current_config: &crate::config::SavedConfig,
        quick_start_profile_name: &str,
    ) {
        let url = current_config.url_api.clone();
        let model = current_config.nom_modele.clone();
        let request = current_config.demande_generique.clone();
        if request.is_empty() {
            self.bus.emit_notification(NotificationEvent::Log("Please enter a request first!".to_string()));
            return;
        }

        let bus = self.bus.clone();
        let client = self.llm_client.clone();
        let profile_name = quick_start_profile_name.trim().to_string();
        let auth_mode = current_config.auth_mode.clone();
        let auth_api_key = current_config.auth_api_key.clone();
        let auth_login = current_config.auth_login.clone();
        let auth_password = current_config.auth_password.clone();
        let timeout_secs = current_config.request_timeout_secs;

        self.bus.emit_notification(NotificationEvent::Log(format!(
            "Sending Text LLM Request (Generate Prompts) | Model: '{}' | Endpoint: '{}'",
            model, url
        )));

        self.rt.spawn(async move {
            match client.generate_config(&request, &url, &model, &auth_mode, &auth_api_key, &auth_login, &auth_password, timeout_secs).await {
                Ok(json_value) => {
                    bus.emit_notification(NotificationEvent::Log("Received LLM Response (Generate Prompts) | Success".to_string()));

                    let get_value = |json: &serde_json::Value, keys: &[&str]| -> serde_json::Value {
                        for key in keys {
                            if let Some(val) = json.get(key) {
                                if !val.is_null() {
                                    return val.clone();
                                }
                            }
                        }
                        serde_json::Value::Null
                    };

                    let parse_to_string = |val: &serde_json::Value| -> String {
                        if val.is_null() {
                            return String::new();
                        }
                        if let Some(s) = val.as_str() {
                            s.to_string()
                        } else if let Some(arr) = val.as_array() {
                            arr.iter()
                                .map(|v| {
                                    if let Some(s) = v.as_str() {
                                        s.to_string()
                                    } else {
                                        v.to_string()
                                    }
                                })
                                .collect::<Vec<String>>()
                                .join("\n")
                        } else {
                            val.to_string()
                        }
                    };

                    let contexte = parse_to_string(&get_value(&json_value, &["contexte", "context"]));
                    let task = parse_to_string(&get_value(&json_value, &["task", "tache", "instructions"]));
                    let objectif = parse_to_string(&get_value(&json_value, &["objectif", "stop_condition", "objective"]));
                    let directives = parse_to_string(&get_value(&json_value, &["directives", "rules", "system_rules", "directives_systeme"]));

                    bus.emit_command(CommandEvent::CreateProfileWithPrompts {
                        profile_name,
                        contexte,
                        task,
                        objectif,
                        directives,
                    });
                }
                Err(e) => {
                    bus.emit_notification(NotificationEvent::Log(format!("LLM Error (Generate Prompts) | Failed to generate prompts: {}", e)));
                }
            }
            bus.emit_command(CommandEvent::SetGeneratingPrompts(false));
        });
    }

    pub fn scan_models(
        &self,
        current_config: &crate::config::SavedConfig,
    ) {
        let url = current_config.url_api.clone();
        let engine = current_config.moteur.clone();
        let auth_mode = current_config.auth_mode.clone();
        let auth_api_key = current_config.auth_api_key.clone();
        let auth_login = current_config.auth_login.clone();
        let auth_password = current_config.auth_password.clone();
        let bus = self.bus.clone();

        self.bus.emit_notification(NotificationEvent::Log(format!("Scanning models for engine '{}'...", engine)));

        let client = crate::llm_client::LlmClient::new();
        self.rt.spawn(async move {
            match client.fetch_models(&url, &auth_mode, &auth_api_key, &auth_login, &auth_password).await {
                Ok(models) => {
                    bus.emit_notification(NotificationEvent::Log(format!("Scan success! Found {} models.", models.len())));
                    bus.emit_command(CommandEvent::UpdateModelsList { engine, models });
                }
                Err(e) => {
                    bus.emit_notification(NotificationEvent::Log(format!("Model scan failed: {}", e)));
                }
            }
        });
    }

    pub fn scan_models_vision(
        &self,
        current_config: &crate::config::SavedConfig,
    ) {
        let url = current_config.url_api_vision.clone();
        let engine = current_config.moteur_vision.clone();
        let auth_mode = current_config.auth_mode_vision.clone();
        let auth_api_key = current_config.auth_api_key_vision.clone();
        let auth_login = current_config.auth_login_vision.clone();
        let auth_password = current_config.auth_password_vision.clone();
        let bus = self.bus.clone();
        let client = self.llm_client.clone();

        self.bus.emit_notification(NotificationEvent::Log(format!("Scanning models for vision engine '{}'...", engine)));

        self.rt.spawn(async move {
            match client.fetch_models(&url, &auth_mode, &auth_api_key, &auth_login, &auth_password).await {
                Ok(models) => {
                    bus.emit_notification(NotificationEvent::Log(format!("Vision scan success! Found {} models.", models.len())));
                    bus.emit_command(CommandEvent::UpdateModelsList { engine, models });
                }
                Err(e) => {
                    bus.emit_notification(NotificationEvent::Log(format!("Vision model scan failed: {}", e)));
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orchestrator::tests::MockLlmClient;
    use crate::config::SavedConfig;
    use crate::event_bus::EventBus;

    #[tokio::test]
    async fn test_llm_service_optimize_field() {
        let (bus, rx) = EventBus::new();
        let client = Arc::new(MockLlmClient {
            next_response: std::sync::Mutex::new(crate::llm_client::LlmResponse::default()),
            decompose_response: std::sync::Mutex::new(String::new()),
            call_count: std::sync::Mutex::new(0),
        });
        
        let rt = tokio::runtime::Handle::current();
        let service = LlmService::new(client, rt, bus);

        let config = SavedConfig::default();
        service.optimize_prompt_field(&config, "contexte", "some feedback");

        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        let mut found_update = false;
        while let Ok(event) = rx.try_recv() {
            if let crate::event_bus::Event::Command { event: CommandEvent::UpdateField { field, value } } = event {
                if field == "contexte" && value == "optimized" {
                    found_update = true;
                }
            }
        }
        assert!(found_update);
    }

    #[tokio::test]
    async fn test_llm_service_generate_prompts() {
        let (bus, rx) = EventBus::new();
        let client = Arc::new(MockLlmClient {
            next_response: std::sync::Mutex::new(crate::llm_client::LlmResponse::default()),
            decompose_response: std::sync::Mutex::new(String::new()),
            call_count: std::sync::Mutex::new(0),
        });
        
        let rt = tokio::runtime::Handle::current();
        let service = LlmService::new(client, rt, bus);

        let mut config = SavedConfig::default();
        config.demande_generique = "build a website".to_string();
        
        service.generate_prompts_from_request(&config, "new-profile");

        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        let mut found = false;
        while let Ok(event) = rx.try_recv() {
            if let crate::event_bus::Event::Command { event: CommandEvent::CreateProfileWithPrompts { profile_name, .. } } = event {
                if profile_name == "new-profile" {
                    found = true;
                }
            }
        }
        assert!(found);
    }

    #[tokio::test]
    async fn test_llm_service_scan_models_vision() {
        let (bus, rx) = EventBus::new();
        let client = Arc::new(MockLlmClient {
            next_response: std::sync::Mutex::new(crate::llm_client::LlmResponse::default()),
            decompose_response: std::sync::Mutex::new(String::new()),
            call_count: std::sync::Mutex::new(0),
        });
        
        let rt = tokio::runtime::Handle::current();
        let service = LlmService::new(client, rt, bus);

        let config = SavedConfig::default();
        service.scan_models_vision(&config);

        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        let mut found = false;
        while let Ok(event) = rx.try_recv() {
            if let crate::event_bus::Event::Command { event: CommandEvent::UpdateModelsList { .. } } = event {
                found = true;
            }
        }
        assert!(found);
    }

    #[tokio::test]
    async fn test_llm_service_scan_models() {
        use tokio::net::TcpListener;
        use tokio::io::{AsyncWriteExt, AsyncReadExt};

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();

        tokio::spawn(async move {
            if let Ok((mut socket, _)) = listener.accept().await {
                let mut buf = [0; 1024];
                let _ = socket.read(&mut buf).await;
                let body = r#"{"data":[{"id":"model1"},{"id":"model2"}]}"#;
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = socket.write_all(response.as_bytes()).await;
                let _ = socket.flush().await;
                let _ = socket.shutdown().await;
            }
        });

        // Give listener a moment to start accepting
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        let (bus, rx) = EventBus::new();
        let client = Arc::new(MockLlmClient {
            next_response: std::sync::Mutex::new(crate::llm_client::LlmResponse::default()),
            decompose_response: std::sync::Mutex::new(String::new()),
            call_count: std::sync::Mutex::new(0),
        });
        
        let rt = tokio::runtime::Handle::current();
        let service = LlmService::new(client, rt, bus);

        let mut config = SavedConfig::default();
        config.url_api = format!("http://127.0.0.1:{}", port);
        
        service.scan_models(&config);

        // Wait for spawned client fetch_models call
        tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;

        let mut found = false;
        let mut logs = Vec::new();
        while let Ok(event) = rx.try_recv() {
            logs.push(format!("{:?}", event));
            if let crate::event_bus::Event::Command { event: CommandEvent::UpdateModelsList { .. } } = event {
                found = true;
            }
        }
        assert!(found, "Logs: {:?}", logs);
    }
}

