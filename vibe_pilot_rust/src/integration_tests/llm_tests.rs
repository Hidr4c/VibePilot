use crate::llm_client::LlmResponse;

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
    assert_eq!(response.scroll_value, -6);
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
fn test_llm_response_success_action() {
    let json = r#"{
        "status_display": "Done",
        "action": "SUCCESS",
        "relative_click_position": [1.0, 1.0],
        "text_to_type": "",
        "scroll_value": 0,
        "wait_seconds": 0
    }"#;
    let response: LlmResponse = serde_json::from_str(json).unwrap();
    assert_eq!(response.action, "SUCCESS");
}

#[test]
fn test_llm_response_fail_action() {
    let json = r#"{
        "status_display": "Failed",
        "action": "FAIL",
        "relative_click_position": [0.0, 0.0],
        "text_to_type": "",
        "scroll_value": 0,
        "wait_seconds": 30
    }"#;
    let response: LlmResponse = serde_json::from_str(json).unwrap();
    assert_eq!(response.action, "FAIL");
    assert_eq!(response.wait_seconds, 30);
}

#[test]
fn test_llm_response_scroll_action() {
    let json = r#"{
        "status_display": "Scrolling",
        "action": "SCROLL",
        "relative_click_position": [0.5, 0.5],
        "text_to_type": "",
        "scroll_value": -12,
        "wait_seconds": 5
    }"#;
    let response: LlmResponse = serde_json::from_str(json).unwrap();
    assert_eq!(response.action, "SCROLL");
    assert_eq!(response.scroll_value, -12);
}
