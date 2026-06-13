use crate::event_bus::{EventBus, EventType, NotificationEvent, CommandEvent, QueryEvent};

#[test]
fn test_new_bus_is_empty() {
    let (bus, _rx) = EventBus::new();
    assert_eq!(bus.query_handler_count(), 0);
}

#[test]
fn test_subscribe_and_emit() {
    let (bus, _rx) = EventBus::new();
    let received = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let received_clone = received.clone();

    bus.subscribe(Box::new(move |_| {
        received_clone.lock().unwrap().push("received".to_string());
    }));

    bus.emit_notification(NotificationEvent::Log("test".to_string()));
    assert_eq!(received.lock().unwrap().len(), 1);
}

#[test]
fn test_multiple_subscribers() {
    let (bus, _rx) = EventBus::new();
    let count = std::sync::Arc::new(std::sync::Mutex::new(0));

    for _ in 0..3 {
        let count_clone = count.clone();
        bus.subscribe_for(
            EventType::Log("test".to_string()),
            Box::new(move |_e| {
                *count_clone.lock().unwrap() += 1;
            }),
        );
    }

    bus.emit_notification(NotificationEvent::Log("test".to_string()));
    assert_eq!(*count.lock().unwrap(), 3);
}

#[test]
fn test_query_handler() {
    let (bus, _rx) = EventBus::new();
    bus.register_query(
        QueryEvent::GetLangText("btn_lancer".to_string()),
        Box::new(|_| "Lancer".to_string()),
    );
    let response = bus.emit_query(QueryEvent::GetLangText("btn_lancer".to_string()));
    assert_eq!(response, "Lancer");
}

#[test]
fn test_query_no_handler_returns_empty() {
    let (bus, _rx) = EventBus::new();
    let response = bus.emit_query(QueryEvent::GetLangText("unknown".to_string()));
    assert_eq!(response, "");
}

#[test]
fn test_event_display() {
    assert_eq!(EventType::Log(String::new()).to_string(), "LOG");
    assert_eq!(
        EventType::UpdateStatus { text: "x".to_string(), color: "y".to_string() }.to_string(),
        "UPDATE_STATUS"
    );
}

#[test]
fn test_event_hash() {
    use std::collections::HashMap;
    let mut map = HashMap::new();
    map.insert(EventType::Log(String::new()), "test");
    assert!(map.contains_key(&EventType::Log(String::new())));
}

#[test]
fn test_clone_bus() {
    let (bus1, _rx1) = EventBus::new();
    bus1.register_query(
        QueryEvent::GetLangText("test".to_string()),
        Box::new(|_| "response".to_string()),
    );
    let bus2 = bus1.clone();
    let response = bus2.emit_query(QueryEvent::GetLangText("test".to_string()));
    assert_eq!(response, "response");
}

#[test]
fn test_default_bus() {
    let bus = EventBus::default();
    assert_eq!(bus.query_handler_count(), 0);
}
