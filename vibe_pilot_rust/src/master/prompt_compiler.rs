use crate::master::dag::{GridGraph, GridNode};
use crate::llm_client::LlmClientFactory;

/// Compiler that orchestrates prompt-to-DAG conversion using LLM providers.
pub struct LlmPromptCompiler;

impl LlmPromptCompiler {
    /// Queries the configured LLM and compiles the user's natural language request into a GridGraph.
    pub async fn compile(
        prompt: &str,
        url: &str,
        model: &str,
        auth_mode: &str,
        auth_api_key: &str,
        auth_login: &str,
        auth_password: &str,
        timeout_secs: u64,
    ) -> Result<GridGraph, String> {
        let client = LlmClientFactory::create();
        let json_value = client.compile_dag(
            prompt,
            url,
            model,
            auth_mode,
            auth_api_key,
            auth_login,
            auth_password,
            timeout_secs
        ).await?;

        let mut graph = GridGraph::new();

        if let Some(arr) = json_value.as_array() {
            for item in arr {
                let id = item.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let name = item.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let profile = item.get("profile_name").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let url = item.get("worker_url").and_then(|v| v.as_str()).unwrap_or("http://127.0.0.1:4040").to_string();
                let duration = item.get("duration_secs").and_then(|v| v.as_f64()).unwrap_or(10.0) as f32;

                if id.is_empty() { continue; }

                let mut node = GridNode::new(id.clone(), name, profile, url);
                node.duration_secs = duration;

                graph.add_node(node);
            }

            // Link dependencies in a second pass
            for item in arr {
                let to_id = item.get("id").and_then(|v| v.as_str()).unwrap_or("");
                if to_id.is_empty() { continue; }

                if let Some(deps) = item.get("dependencies").and_then(|v| v.as_array()) {
                    for dep in deps {
                        if let Some(from_id) = dep.as_str() {
                            graph.add_dependency(from_id, to_id);
                        }
                    }
                }
            }
            graph.compute_schedule();
        }

        Ok(graph)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::net::TcpListener;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    #[tokio::test]
    async fn test_llm_prompt_compiler_real_network() {
        // Bind TcpListener on an ephemeral port
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();

        // Spawn mock completions HTTP responder
        tokio::spawn(async move {
            if let Ok((mut socket, _)) = listener.accept().await {
                let mut buffer = [0; 4096];
                let _ = socket.read(&mut buffer).await;
                
                // Pre-canned choices structure containing JSON array string
                let content_body = r#"[{"id": "node_1", "name": "1. Fetch Data", "profile_name": "Fetch Excel", "worker_url": "http://127.0.0.1:4040", "dependencies": [], "duration_secs": 20.0},{"id": "node_2", "name": "2. Format Sheet", "profile_name": "Excel Formatter", "worker_url": "http://127.0.0.1:4040", "dependencies": ["node_1"], "duration_secs": 15.0}]"#;
                let response_json = serde_json::json!({
                    "choices": [
                        {
                            "message": {
                                "content": content_body
                            }
                        }
                    ]
                });
                let response_body = serde_json::to_string(&response_json).unwrap();
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    response_body.len(),
                    response_body
                );
                let _ = socket.write_all(response.as_bytes()).await;
                let _ = socket.flush().await;
                let _ = socket.shutdown().await;
            }
        });

        // Give the listener a fraction of time to start accepting
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        let url = format!("http://127.0.0.1:{}", port);
        let graph = LlmPromptCompiler::compile(
            "test request",
            &url,
            "mock-model",
            "none",
            "",
            "",
            "",
            5,
        ).await.unwrap();

        assert_eq!(graph.nodes.len(), 2);
        let node1 = graph.nodes.get("node_1").unwrap();
        let node2 = graph.nodes.get("node_2").unwrap();
        assert_eq!(node1.name, "1. Fetch Data");
        assert_eq!(node2.profile_name, "Excel Formatter");
        assert_eq!(node2.dependencies, vec!["node_1".to_string()]);
    }
}

