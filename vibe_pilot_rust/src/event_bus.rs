//! Thread-safe event bus for VibePilot.
//!
//! Uses flume channels for async event propagation from the orchestrator
//! to the UI thread.

use std::collections::HashMap;
use std::fmt;

/// Unique event types for the orchestrator.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum EventType {
    Log(String),
    UpdateStatus { text: String, color: String },
    AppendAction { action: String, display: String },
    SelectTab(usize),
    GetLangText(String),
    GetDernierMouvementHumain,
    GetModePauseForcee,
    UpdateField { field: String, value: String },
    UpdateAllFields { contexte: String, task: String, objectif: String, directives: String },
    ShowActionConfirmation { action: String, text: String, scroll: i32 },
    GetActionConfirmationStatus,
    ClearActionConfirmation,
}

impl fmt::Display for EventType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EventType::Log(_) => write!(f, "LOG"),
            EventType::UpdateStatus { .. } => write!(f, "UPDATE_STATUS"),
            EventType::AppendAction { .. } => write!(f, "APPEND_ACTION"),
            EventType::SelectTab(_) => write!(f, "SELECT_TAB"),
            EventType::GetLangText(_) => write!(f, "GET_LANG_TEXT"),
            EventType::GetDernierMouvementHumain => write!(f, "GET_DERNIER_MOUVEMENT_HUMAIN"),
            EventType::GetModePauseForcee => write!(f, "GET_MODE_PAUSE_FORCEE"),
            EventType::UpdateField { .. } => write!(f, "UPDATE_FIELD"),
            EventType::UpdateAllFields { .. } => write!(f, "UPDATE_ALL_FIELDS"),
            EventType::ShowActionConfirmation { .. } => write!(f, "SHOW_ACTION_CONFIRMATION"),
            EventType::GetActionConfirmationStatus => write!(f, "GET_ACTION_CONFIRMATION_STATUS"),
            EventType::ClearActionConfirmation => write!(f, "CLEAR_ACTION_CONFIRMATION"),
        }
    }
}

/// Event data that can be sent through the bus.
#[derive(Debug, Clone)]
pub enum Event {
    Emit { event: EventType },
    Query { event: EventType, sender: flume::Sender<EventResponse> },
}

/// Response to a query event.
#[derive(Debug, Clone)]
pub struct EventResponse {
    pub event_type: EventType,
    pub response: String,
}

/// Query handler type.
pub type QueryHandler = Box<dyn Fn(&EventType) -> String + Send + Sync>;

/// Subscriber handler type for emit events.
pub type SubscriberHandler = Box<dyn Fn(&EventType) + Send + Sync>;

/// Filtered subscriber with an optional EventType filter.
struct FilteredSubscriber {
    filter: Option<EventType>,
    handler: SubscriberHandler,
}

/// Thread-safe event bus for VibePilot.
pub struct EventBus {
    sender: flume::Sender<Event>,
    receiver: flume::Receiver<Event>,
    query_handlers: std::sync::Arc<std::sync::Mutex<HashMap<EventType, QueryHandler>>>,
    subscribers: std::sync::Arc<std::sync::Mutex<Vec<FilteredSubscriber>>>,
}

impl EventBus {
    /// Create a new event bus and return the sender side plus a receiver channel.
    pub fn new() -> (Self, flume::Receiver<Event>) {
        let (sender, receiver) = flume::unbounded();
        (
            Self {
                sender,
                receiver: receiver.clone(),
                query_handlers: std::sync::Arc::new(std::sync::Mutex::new(HashMap::new())),
                subscribers: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
            },
            receiver,
        )
    }

    /// Emit an event to the channel and call all subscribers.
    pub fn emit(&self, event: EventType) {
        let _ = self.sender.send(Event::Emit { event: event.clone() });
        let subs = self.subscribers.lock().unwrap();
        for sub in subs.iter() {
            if let Some(ref filter) = sub.filter {
                if *filter != event {
                    continue;
                }
            }
            (sub.handler)(&event);
        }
    }

    /// Subscribe to events with a callback handler.
    pub fn subscribe(&self, handler: SubscriberHandler) {
        let mut subs = self.subscribers.lock().unwrap();
        subs.push(FilteredSubscriber {
            filter: None,
            handler,
        });
    }

    /// Subscribe to a specific event type with a callback handler.
    pub fn subscribe_for(&self, filter: EventType, handler: SubscriberHandler) {
        let mut subs = self.subscribers.lock().unwrap();
        subs.push(FilteredSubscriber {
            filter: Some(filter),
            handler,
        });
    }

    /// Register a query handler for an event type.
    pub fn register_query(&self, event: EventType, handler: QueryHandler) {
        let mut handlers = self.query_handlers.lock().unwrap();
        handlers.insert(event, handler);
    }

    /// Emit a query and return the response.
    pub fn emit_query(&self, event: EventType) -> String {
        let handlers = self.query_handlers.lock().unwrap();
        if let Some(handler) = handlers.get(&event) {
            return handler(&event);
        }
        String::new()
    }

    /// Get the event receiver for polling in the UI loop.
    pub fn receiver(&self) -> flume::Receiver<Event> {
        self.receiver.clone()
    }

    /// Get the number of query handlers.
    pub fn query_handler_count(&self) -> usize {
        let handlers = self.query_handlers.lock().unwrap();
        handlers.len()
    }

    /// Get the number of subscribers.
    pub fn subscriber_count(&self) -> usize {
        let subs = self.subscribers.lock().unwrap();
        subs.len()
    }
}

impl Clone for EventBus {
    fn clone(&self) -> Self {
        EventBus {
            sender: self.sender.clone(),
            receiver: self.receiver.clone(),
            query_handlers: self.query_handlers.clone(),
            subscribers: self.subscribers.clone(),
        }
    }
}

impl Default for EventBus {
    fn default() -> Self {
        let (bus, _rx) = Self::new();
        bus
    }
}

#[cfg(test)]
mod tests {
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
            EventType::GetLangText("btn_lancer".to_string()),
            Box::new(|_| "Lancer".to_string()),
        );
        let response = bus.emit_query(EventType::GetLangText("btn_lancer".to_string()));
        assert_eq!(response, "Lancer");
    }

    #[test]
    fn test_query_no_handler_returns_empty() {
        let (bus, _rx) = EventBus::new();
        let response = bus.emit_query(EventType::GetLangText("unknown".to_string()));
        assert_eq!(response, "");
    }

    #[test]
    fn test_event_display() {
        assert_eq!(EventType::Log(String::new()).to_string(), "LOG");
        assert_eq!(EventType::UpdateStatus { text: "x".to_string(), color: "y".to_string() }.to_string(), "UPDATE_STATUS");
    }
}
