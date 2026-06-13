use super::*;

#[test]
fn test_new_bus_has_channel() {
    let (bus, _rx) = EventBus::new();
    assert_eq!(bus.query_handler_count(), 0);
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
fn test_subscribers() {
    let (bus, _rx) = EventBus::new();
    let counter = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    
    let c_clone = counter.clone();
    bus.subscribe(Box::new(move |_| {
        c_clone.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }));

    let c_clone2 = counter.clone();
    bus.subscribe_for(
        EventType::GetUserFeedback,
        Box::new(move |_| {
            c_clone2.fetch_add(10, std::sync::atomic::Ordering::Relaxed);
        })
    );

    assert_eq!(bus.subscriber_count(), 2);

    bus.emit_notification(NotificationEvent::Log("test".to_string()));
    assert_eq!(counter.load(std::sync::atomic::Ordering::Relaxed), 1);

    bus.emit_query(QueryEvent::GetUserFeedback);
    assert_eq!(counter.load(std::sync::atomic::Ordering::Relaxed), 12);
}

#[test]
fn test_event_bus_clone_and_default() {
    let bus1 = EventBus::default();
    let bus2 = bus1.clone();
    assert_eq!(bus2.query_handler_count(), 0);
    let _receiver = bus2.receiver();
}

#[test]
fn test_notification_event_display() {
    assert_eq!(NotificationEvent::Log("x".to_string()).to_string(), "LOG");
    assert_eq!(NotificationEvent::UpdateStatus { text: "x".to_string(), color: "y".to_string() }.to_string(), "UPDATE_STATUS");
    assert_eq!(NotificationEvent::AppendAction { action: "x".to_string(), display: "y".to_string() }.to_string(), "APPEND_ACTION");
    assert_eq!(NotificationEvent::AppendReport("x".to_string()).to_string(), "APPEND_REPORT");
    assert_eq!(NotificationEvent::LoopDetectedAlert { message: "x".to_string() }.to_string(), "LOOP_DETECTED_ALERT");
    assert_eq!(NotificationEvent::ClearActionConfirmation.to_string(), "CLEAR_ACTION_CONFIRMATION");
}

#[test]
fn test_query_event_display() {
    assert_eq!(QueryEvent::GetLangText("x".to_string()).to_string(), "GET_LANG_TEXT");
    assert_eq!(QueryEvent::GetDernierMouvementHumain.to_string(), "GET_DERNIER_MOUVEMENT_HUMAIN");
    assert_eq!(QueryEvent::GetModePauseForcee.to_string(), "GET_MODE_PAUSE_FORCEE");
    assert_eq!(QueryEvent::GetActionConfirmationStatus.to_string(), "GET_ACTION_CONFIRMATION_STATUS");
    assert_eq!(QueryEvent::GetOrchestratorRunning.to_string(), "GET_ORCHESTRATOR_RUNNING");
    assert_eq!(QueryEvent::GetUserFeedback.to_string(), "GET_USER_FEEDBACK");
}

#[test]
fn test_command_event_display() {
    assert_eq!(CommandEvent::SelectTab(1).to_string(), "SELECT_TAB");
    assert_eq!(CommandEvent::UpdateField { field: "x".to_string(), value: "y".to_string() }.to_string(), "UPDATE_FIELD");
    assert_eq!(CommandEvent::UpdateAllFields { contexte: "a".to_string(), task: "b".to_string(), objectif: "c".to_string(), directives: "d".to_string() }.to_string(), "UPDATE_ALL_FIELDS");
    assert_eq!(CommandEvent::ShowActionConfirmation { action: "x".to_string(), text: "y".to_string(), scroll: 1 }.to_string(), "SHOW_ACTION_CONFIRMATION");
    assert_eq!(CommandEvent::SetGeneratingPrompts(true).to_string(), "SET_GENERATING_PROMPTS");
    assert_eq!(CommandEvent::UpdateModelsList { engine: "x".to_string(), models: vec![] }.to_string(), "UPDATE_MODELS_LIST");
    assert_eq!(CommandEvent::CreateProfileWithPrompts { profile_name: "a".to_string(), contexte: "b".to_string(), task: "c".to_string(), objectif: "d".to_string(), directives: "e".to_string() }.to_string(), "CREATE_PROFILE_WITH_PROMPTS");
    assert_eq!(CommandEvent::ShowProfileReadyPopup { profile_name: "x".to_string() }.to_string(), "SHOW_PROFILE_READY_POPUP");
    assert_eq!(CommandEvent::StopOrchestrator.to_string(), "STOP_ORCHESTRATOR");
}

#[test]
fn test_event_bus_emit_legacy() {
    let bus = EventBus::default();
    let counter = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let c_clone = counter.clone();
    bus.subscribe(Box::new(move |_| {
        c_clone.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }));

    bus.emit(EventType::Log("test".to_string()));
    bus.emit(EventType::UpdateStatus { text: "x".to_string(), color: "y".to_string() });
    bus.emit(EventType::AppendAction { action: "x".to_string(), display: "y".to_string() });
    bus.emit(EventType::AppendReport("x".to_string()));
    bus.emit(EventType::LoopDetectedAlert { message: "x".to_string() });
    bus.emit(EventType::ClearActionConfirmation);
    bus.emit(EventType::SelectTab(1));
    bus.emit(EventType::UpdateField { field: "x".to_string(), value: "y".to_string() });
    bus.emit(EventType::UpdateAllFields { contexte: "a".to_string(), task: "b".to_string(), objectif: "c".to_string(), directives: "d".to_string() });
    bus.emit(EventType::ShowActionConfirmation { action: "x".to_string(), text: "y".to_string(), scroll: 1 });
    bus.emit(EventType::SetGeneratingPrompts(true));
    bus.emit(EventType::UpdateModelsList { engine: "x".to_string(), models: vec![] });
    bus.emit(EventType::CreateProfileWithPrompts { profile_name: "a".to_string(), contexte: "b".to_string(), task: "c".to_string(), blueprint: "d".to_string(), instructions: "e".to_string() });
    bus.emit(EventType::ShowProfileReadyPopup { profile_name: "x".to_string() });
    bus.emit(EventType::StopOrchestrator);
    bus.emit(EventType::GetLangText("x".to_string())); // Query event ignored by emit

    assert_eq!(counter.load(std::sync::atomic::Ordering::Relaxed), 15);
}

#[test]
fn test_event_type_conversions() {
    let ev1: EventType = NotificationEvent::Log("x".to_string()).into();
    assert_eq!(ev1, EventType::Log("x".to_string()));

    let ev2: EventType = QueryEvent::GetLangText("x".to_string()).into();
    assert_eq!(ev2, EventType::GetLangText("x".to_string()));

    let ev3: EventType = CommandEvent::SelectTab(1).into();
    assert_eq!(ev3, EventType::SelectTab(1));
}
