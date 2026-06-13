use super::*;
use tokio::net::TcpListener;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

async fn start_mock_server(response_body: &'static str, status_line: &'static str) -> (String, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let url = format!("http://127.0.0.1:{}/chat/completions", port);
    
    let handle = tokio::spawn(async move {
        if let Ok((mut socket, _)) = listener.accept().await {
            let mut request_bytes = Vec::new();
            let mut buf = [0u8; 1024];
            let mut content_length = None;
            let mut header_end_idx = None;

            loop {
                let n = socket.read(&mut buf).await.unwrap();
                if n == 0 {
                    break;
                }
                request_bytes.extend_from_slice(&buf[..n]);

                if let Some(idx) = request_bytes.windows(4).position(|w| w == b"\r\n\r\n") {
                    header_end_idx = Some(idx + 4);
                    let headers_str = String::from_utf8_lossy(&request_bytes[..idx]);
                    for line in headers_str.lines() {
                        if line.to_lowercase().starts_with("content-length:") {
                            if let Some(val_str) = line.split(':').nth(1) {
                                if let Ok(val) = val_str.trim().parse::<usize>() {
                                    content_length = Some(val);
                                }
                            }
                        }
                    }
                    break;
                }
            }

            if let (Some(body_start), Some(len)) = (header_end_idx, content_length) {
                while request_bytes.len() < body_start + len {
                    let mut body_buf = vec![0u8; (body_start + len - request_bytes.len()).min(4096)];
                    let n = socket.read(&mut body_buf).await.unwrap();
                    if n == 0 {
                        break;
                    }
                    request_bytes.extend_from_slice(&body_buf[..n]);
                }
            }
            
            let response = format!(
                "HTTP/1.1 {}\r\nContent-Length: {}\r\nContent-Type: application/json\r\n\r\n{}",
                status_line,
                response_body.len(),
                response_body
            );
            socket.write_all(response.as_bytes()).await.unwrap();
        }
    });
    
    (url, handle)
}

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

#[tokio::test]
async fn test_decompose_objective_unreachable() {
    let client = LlmClient::new();
    let res = client.decompose_objective(
        "objective",
        "context",
        "task",
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
async fn test_identify_roi_unreachable() {
    let client = LlmClient::new();
    let image = image::DynamicImage::ImageRgba8(image::ImageBuffer::new(100, 100));
    let res = client.identify_roi(
        &image,
        "context",
        "objective",
        "task",
        "http://127.0.0.1:65535/chat/completions",
        "model",
        "none",
        " ",
        " ",
        " ",
        120
    ).await;
    assert!(res.is_err());
}

#[tokio::test]
async fn test_compress_history_unreachable() {
    let client = LlmClient::new();
    let res = client.compress_history(
        "old steps",
        "prev summary",
        "English",
        "http://127.0.0.1:65535/chat/completions",
        "model",
        "none",
        " ",
        " ",
        " ",
        120
    ).await;
    assert!(res.is_err());
}

#[tokio::test]
async fn test_is_engine_busy_not_busy() {
    let (url, _handle) = start_mock_server("[]", "200 OK").await;
    let client = LlmClient::new();
    let busy = client.is_engine_busy(&url, "model").await;
    assert!(!busy);
}

#[tokio::test]
async fn test_fetch_models_success() {
    let body = r#"{"data": [{"id": "model-1"}, {"id": "model-2"}]}"#;
    let (url, _handle) = start_mock_server(body, "200 OK").await;
    let client = LlmClient::new();
    let res = client.fetch_models(&url, "none", "", "", "").await;
    assert!(res.is_ok());
    let models = res.unwrap();
    assert_eq!(models, vec!["model-1".to_string(), "model-2".to_string()]);
}

#[tokio::test]
async fn test_fetch_models_http_error() {
    let (url, _handle) = start_mock_server("error details", "500 Internal Server Error").await;
    let client = LlmClient::new();
    let res = client.fetch_models(&url, "none", "", "", "").await;
    assert!(res.is_err());
    assert!(res.unwrap_err().contains("HTTP error: 500"));
}

#[tokio::test]
async fn test_fetch_models_json_error() {
    let (url, _handle) = start_mock_server("invalid json", "200 OK").await;
    let client = LlmClient::new();
    let res = client.fetch_models(&url, "none", "", "", "").await;
    assert!(res.is_err());
    assert!(res.unwrap_err().contains("JSON parse error"));
}

#[tokio::test]
async fn test_fetch_models_empty() {
    let (url, _handle) = start_mock_server(r#"{"data": []}"#, "200 OK").await;
    let client = LlmClient::new();
    let res = client.fetch_models(&url, "none", "", "", "").await;
    assert!(res.is_err());
    assert!(res.unwrap_err().contains("No models found"));
}

#[tokio::test]
async fn test_execute_decision_success() {
    let response_json = r#"{
        "choices": [{
            "message": {
                "content": "```json\n{\n  \"status_display\": \"Clicking\",\n  \"action\": \"CLICK_AND_TYPE\",\n  \"relative_click_position\": [0.2, 0.4],\n  \"text_to_type\": \"test\"\n}\n```"
            }
        }]
    }"#;
    let (url, _handle) = start_mock_server(response_json, "200 OK").await;
    let client = LlmClient::new();
    let image = image::DynamicImage::ImageRgba8(image::ImageBuffer::new(10, 10));
    let res = client.execute_decision(
        &image, "ctx", "obj", "task", "dirs", "feedback",
        &url, "model", "api_key", "my_key", "", "", 10
    ).await;
    assert!(res.is_ok());
    let response = res.unwrap();
    assert_eq!(response.action, "CLICK_AND_TYPE");
    assert_eq!(response.relative_click_position, vec![0.2, 0.4]);
    assert_eq!(response.text_to_type, "test");
}

#[tokio::test]
async fn test_execute_decision_unknown_action() {
    let response_json = r#"{
        "choices": [{
            "message": {
                "content": "{\"status_display\": \"Oops\", \"action\": \"UNKNOWN_ACTION_XYZ\"}"
            }
        }]
    }"#;
    let (url, _handle) = start_mock_server(response_json, "200 OK").await;
    let client = LlmClient::new();
    let image = image::DynamicImage::ImageRgba8(image::ImageBuffer::new(10, 10));
    let res = client.execute_decision(
        &image, "ctx", "obj", "task", "dirs", "",
        &url, "model", "basic_auth", "", "user", "pass", 10
    ).await;
    assert!(res.is_err());
    assert!(res.unwrap_err().contains("unknown action"));
}

#[tokio::test]
async fn test_execute_decision_missing_content() {
    let response_json = r#"{"choices": [{"message": {}}]}"#;
    let (url, _handle) = start_mock_server(response_json, "200 OK").await;
    let client = LlmClient::new();
    let image = image::DynamicImage::ImageRgba8(image::ImageBuffer::new(10, 10));
    let res = client.execute_decision(
        &image, "ctx", "obj", "task", "dirs", "",
        &url, "model", "none", "", "", "", 10
    ).await;
    assert!(res.is_err());
}

#[tokio::test]
async fn test_optimize_field_success() {
    let response_json = r#"{"choices": [{"message": {"content": "Optimized PromptText"}}]}"#;
    let (url, _handle) = start_mock_server(response_json, "200 OK").await;
    let client = LlmClient::new();
    let res = client.optimize_field("original text", "contexte", &url, "model", "none", "", "", "", 10).await;
    assert!(res.is_ok());
    assert_eq!(res.unwrap(), "Optimized PromptText");
}

#[tokio::test]
async fn test_generate_config_success() {
    let response_json = r#"{"choices": [{"message": {"content": "{\"contexte\": \"new ctx\"}"}}]}"#;
    let (url, _handle) = start_mock_server(response_json, "200 OK").await;
    let client = LlmClient::new();
    let res = client.generate_config("user request", &url, "model", "none", "", "", "", 10).await;
    assert!(res.is_ok());
    let val = res.unwrap();
    assert_eq!(val["contexte"], "new ctx");
}

#[tokio::test]
async fn test_decompose_objective_success() {
    let response_json = r#"{"choices": [{"message": {"content": "[{\"id\": 1}]"}}]}"#;
    let (url, _handle) = start_mock_server(response_json, "200 OK").await;
    let client = LlmClient::new();
    let res = client.decompose_objective("obj", "ctx", "task", &url, "model", "none", "", "", "", 10).await;
    assert!(res.is_ok());
    assert_eq!(res.unwrap(), "[{\"id\": 1}]");
}

#[tokio::test]
async fn test_identify_roi_success() {
    let response_json = r#"{"choices": [{"message": {"content": "{\"x\": 0.5}"}}]}"#;
    let (url, _handle) = start_mock_server(response_json, "200 OK").await;
    let client = LlmClient::new();
    let image = image::DynamicImage::ImageRgba8(image::ImageBuffer::new(10, 10));
    let res = client.identify_roi(&image, "ctx", "obj", "task", &url, "model", "none", "", "", "", 10).await;
    assert!(res.is_ok());
    assert_eq!(res.unwrap(), "{\"x\": 0.5}");
}

#[tokio::test]
async fn test_compress_history_success() {
    let response_json = r#"{"choices": [{"message": {"content": "Compressed text"}}]}"#;
    
    // English
    let (url1, _handle1) = start_mock_server(response_json, "200 OK").await;
    let client = LlmClient::new();
    let res1 = client.compress_history("old steps", "prev", "English", &url1, "model", "none", "", "", "", 10).await;
    assert!(res1.is_ok());
    assert_eq!(res1.unwrap(), "Compressed text");

    // French
    let (url2, _handle2) = start_mock_server(response_json, "200 OK").await;
    let res2 = client.compress_history("old steps", "prev", "Français", &url2, "model", "none", "", "", "", 10).await;
    assert!(res2.is_ok());
    assert_eq!(res2.unwrap(), "Compressed text");
}

#[test]
fn test_should_zoom_low_confidence() {
    assert!(should_zoom(0.5, 300, None));
    assert!(should_zoom(0.59, 300, None));
}

#[test]
fn test_should_zoom_high_confidence_no_zoom() {
    assert!(!should_zoom(0.6, 300, None));
    assert!(!should_zoom(1.0, 300, None));
}

#[test]
fn test_should_zoom_small_target() {
    assert!(should_zoom(1.0, 199, None));
    assert!(!should_zoom(1.0, 200, None));
}

#[test]
fn test_should_zoom_high_entropy() {
    assert!(should_zoom(1.0, 300, Some(4.1)));
    assert!(!should_zoom(1.0, 300, Some(4.0)));
}

#[test]
fn test_should_zoom_none_entropy() {
    assert!(!should_zoom(1.0, 300, None));
}

#[test]
fn test_should_zoom_multiple_triggers() {
    assert!(should_zoom(0.5, 100, Some(5.0)));
}
