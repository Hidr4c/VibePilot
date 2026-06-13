use std::future::Future;
use std::pin::Pin;
use super::{LlmClient, LlmMetadataProvider};

impl LlmMetadataProvider for LlmClient {
    fn is_engine_busy<'a>(&'a self, url: &'a str, _model: &'a str) -> Pin<Box<dyn Future<Output = bool> + Send + 'a>> {
        Box::pin(async move {
            let test_url = if url.ends_with("/chat/completions") {
                url.trim_end_matches("/chat/completions").to_owned() + "/models"
            } else {
                url.to_string()
            };

            match self.client.get(test_url).timeout(std::time::Duration::from_secs(2)).send().await {
                Ok(resp) => !resp.status().is_success(),
                Err(_) => true,
            }
        })
    }

    fn fetch_models<'a>(
        &'a self,
        url: &'a str,
        auth_mode: &'a str,
        auth_api_key: &'a str,
        auth_login: &'a str,
        auth_password: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<String>, String>> + Send + 'a>> {
        Box::pin(async move {
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
        })
    }
}
