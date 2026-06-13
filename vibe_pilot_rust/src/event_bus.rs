//! Thread-safe event bus for VibePilot.
//!
//! Uses flume channels for async event propagation from the orchestrator
//! to the UI thread.

use std::collections::HashMap;
use std::fmt;

// ============================================================
// Focused event enums (split from monolithic EventType)
// ============================================================

/// Notification events: fire-and-forget messages to the UI.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum NotificationEvent {
    Log(String),
    UpdateStatus { text: String, color: String },
    AppendAction { action: String, display: String },
    AppendReport(String),
    LoopDetectedAlert { message: String },
    ClearActionConfirmation,
}

impl fmt::Display for NotificationEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NotificationEvent::Log(_) => write!(f, "LOG"),
            NotificationEvent::UpdateStatus { .. } => write!(f, "UPDATE_STATUS"),
            NotificationEvent::AppendAction { .. } => write!(f, "APPEND_ACTION"),
            NotificationEvent::AppendReport(_) => write!(f, "APPEND_REPORT"),
            NotificationEvent::LoopDetectedAlert { .. } => write!(f, "LOOP_DETECTED_ALERT"),
            NotificationEvent::ClearActionConfirmation => write!(f, "CLEAR_ACTION_CONFIRMATION"),
        }
    }
}

/// Query events: request-response messages from orchestrator to UI.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum QueryEvent {
    GetLangText(String),
    GetDernierMouvementHumain,
    GetModePauseForcee,
    GetActionConfirmationStatus,
    GetOrchestratorRunning,
    GetUserFeedback,
}

impl fmt::Display for QueryEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            QueryEvent::GetLangText(_) => write!(f, "GET_LANG_TEXT"),
            QueryEvent::GetDernierMouvementHumain => write!(f, "GET_DERNIER_MOUVEMENT_HUMAIN"),
            QueryEvent::GetModePauseForcee => write!(f, "GET_MODE_PAUSE_FORCEE"),
            QueryEvent::GetActionConfirmationStatus => write!(f, "GET_ACTION_CONFIRMATION_STATUS"),
            QueryEvent::GetOrchestratorRunning => write!(f, "GET_ORCHESTRATOR_RUNNING"),
            QueryEvent::GetUserFeedback => write!(f, "GET_USER_FEEDBACK"),
        }
    }
}

/// Command events: direct UI actions from the orchestrator.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CommandEvent {
    SelectTab(usize),
    UpdateField { field: String, value: String },
    UpdateAllFields { contexte: String, task: String, objectif: String, directives: String },
    ShowActionConfirmation { action: String, text: String, scroll: i32 },
    SetGeneratingPrompts(bool),
    UpdateModelsList { engine: String, models: Vec<String> },
    CreateProfileWithPrompts { profile_name: String, contexte: String, task: String, objectif: String, directives: String },
    ShowProfileReadyPopup { profile_name: String },
    StopOrchestrator,
    StartOrchestrator,
    LoadProfile(String),
}

impl fmt::Display for CommandEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CommandEvent::SelectTab(_) => write!(f, "SELECT_TAB"),
            CommandEvent::UpdateField { .. } => write!(f, "UPDATE_FIELD"),
            CommandEvent::UpdateAllFields { .. } => write!(f, "UPDATE_ALL_FIELDS"),
            CommandEvent::ShowActionConfirmation { .. } => write!(f, "SHOW_ACTION_CONFIRMATION"),
            CommandEvent::SetGeneratingPrompts(_) => write!(f, "SET_GENERATING_PROMPTS"),
            CommandEvent::UpdateModelsList { .. } => write!(f, "UPDATE_MODELS_LIST"),
            CommandEvent::CreateProfileWithPrompts { .. } => write!(f, "CREATE_PROFILE_WITH_PROMPTS"),
            CommandEvent::ShowProfileReadyPopup { .. } => write!(f, "SHOW_PROFILE_READY_POPUP"),
            CommandEvent::StopOrchestrator => write!(f, "STOP_ORCHESTRATOR"),
            CommandEvent::StartOrchestrator => write!(f, "START_ORCHESTRATOR"),
            CommandEvent::LoadProfile(_) => write!(f, "LOAD_PROFILE"),
        }
    }
}

/// Original EventType enum kept for backward compatibility with
/// query handler keys and subscriber filters.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum EventType {
    GetLangText(String),
    GetDernierMouvementHumain,
    GetModePauseForcee,
    GetActionConfirmationStatus,
    GetOrchestratorRunning,
    GetUserFeedback,
    // Legacy variants for backward compatibility
    Log(String),
    UpdateStatus { text: String, color: String },
    AppendAction { action: String, display: String },
    AppendReport(String),
    LoopDetectedAlert { message: String },
    ClearActionConfirmation,
    SelectTab(usize),
    UpdateField { field: String, value: String },
    UpdateAllFields { contexte: String, task: String, objectif: String, directives: String },
    ShowActionConfirmation { action: String, text: String, scroll: i32 },
    SetGeneratingPrompts(bool),
    UpdateModelsList { engine: String, models: Vec<String> },
    CreateProfileWithPrompts { profile_name: String, contexte: String, task: String, blueprint: String, instructions: String },
    ShowProfileReadyPopup { profile_name: String },
    StopOrchestrator,
    StartOrchestrator,
    LoadProfile(String),
}

impl fmt::Display for EventType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EventType::GetLangText(_) => write!(f, "GET_LANG_TEXT"),
            EventType::GetDernierMouvementHumain => write!(f, "GET_DERNIER_MOUVEMENT_HUMAIN"),
            EventType::GetModePauseForcee => write!(f, "GET_MODE_PAUSE_FORCEE"),
            EventType::GetActionConfirmationStatus => write!(f, "GET_ACTION_CONFIRMATION_STATUS"),
            EventType::GetOrchestratorRunning => write!(f, "GET_ORCHESTRATOR_RUNNING"),
            EventType::GetUserFeedback => write!(f, "GET_USER_FEEDBACK"),
            EventType::Log(_) => write!(f, "LOG"),
            EventType::UpdateStatus { .. } => write!(f, "UPDATE_STATUS"),
            EventType::AppendAction { .. } => write!(f, "APPEND_ACTION"),
            EventType::AppendReport(_) => write!(f, "APPEND_REPORT"),
            EventType::LoopDetectedAlert { .. } => write!(f, "LOOP_DETECTED_ALERT"),
            EventType::ClearActionConfirmation => write!(f, "CLEAR_ACTION_CONFIRMATION"),
            EventType::SelectTab(_) => write!(f, "SELECT_TAB"),
            EventType::UpdateField { .. } => write!(f, "UPDATE_FIELD"),
            EventType::UpdateAllFields { .. } => write!(f, "UPDATE_ALL_FIELDS"),
            EventType::ShowActionConfirmation { .. } => write!(f, "SHOW_ACTION_CONFIRMATION"),
            EventType::SetGeneratingPrompts(_) => write!(f, "SET_GENERATING_PROMPTS"),
            EventType::UpdateModelsList { .. } => write!(f, "UPDATE_MODELS_LIST"),
            EventType::CreateProfileWithPrompts { .. } => write!(f, "CREATE_PROFILE_WITH_PROMPTS"),
            EventType::ShowProfileReadyPopup { .. } => write!(f, "SHOW_PROFILE_READY_POPUP"),
            EventType::StopOrchestrator => write!(f, "STOP_ORCHESTRATOR"),
            EventType::StartOrchestrator => write!(f, "START_ORCHESTRATOR"),
            EventType::LoadProfile(_) => write!(f, "LOAD_PROFILE"),
        }
    }
}

impl From<NotificationEvent> for EventType {
    fn from(event: NotificationEvent) -> Self {
        match event {
            NotificationEvent::Log(msg) => EventType::Log(msg),
            NotificationEvent::UpdateStatus { text, color } => EventType::UpdateStatus { text, color },
            NotificationEvent::AppendAction { action, display } => EventType::AppendAction { action, display },
            NotificationEvent::AppendReport(report) => EventType::AppendReport(report),
            NotificationEvent::LoopDetectedAlert { message } => EventType::LoopDetectedAlert { message },
            NotificationEvent::ClearActionConfirmation => EventType::ClearActionConfirmation,
        }
    }
}

impl From<QueryEvent> for EventType {
    fn from(event: QueryEvent) -> Self {
        match event {
            QueryEvent::GetLangText(key) => EventType::GetLangText(key),
            QueryEvent::GetDernierMouvementHumain => EventType::GetDernierMouvementHumain,
            QueryEvent::GetModePauseForcee => EventType::GetModePauseForcee,
            QueryEvent::GetActionConfirmationStatus => EventType::GetActionConfirmationStatus,
            QueryEvent::GetOrchestratorRunning => EventType::GetOrchestratorRunning,
            QueryEvent::GetUserFeedback => EventType::GetUserFeedback,
        }
    }
}

impl From<CommandEvent> for EventType {
    fn from(event: CommandEvent) -> Self {
        match event {
            CommandEvent::SelectTab(idx) => EventType::SelectTab(idx),
            CommandEvent::UpdateField { field, value } => EventType::UpdateField { field, value },
            CommandEvent::UpdateAllFields { contexte, task, objectif, directives } => EventType::UpdateAllFields { contexte, task, objectif, directives },
            CommandEvent::ShowActionConfirmation { action, text, scroll } => EventType::ShowActionConfirmation { action, text, scroll },
            CommandEvent::SetGeneratingPrompts(status) => EventType::SetGeneratingPrompts(status),
            CommandEvent::UpdateModelsList { engine, models } => EventType::UpdateModelsList { engine, models },
            CommandEvent::CreateProfileWithPrompts { profile_name, contexte, task, objectif, directives } => EventType::CreateProfileWithPrompts { profile_name, contexte, task, blueprint: objectif, instructions: directives },
            CommandEvent::ShowProfileReadyPopup { profile_name } => EventType::ShowProfileReadyPopup { profile_name },
            CommandEvent::StopOrchestrator => EventType::StopOrchestrator,
            CommandEvent::StartOrchestrator => EventType::StartOrchestrator,
            CommandEvent::LoadProfile(p) => EventType::LoadProfile(p),
        }
    }
}

/// Event data that can be sent through the bus.
#[derive(Debug, Clone)]
pub enum Event {
    Emit { event: NotificationEvent },
    Query { event: QueryEvent, sender: flume::Sender<EventResponse> },
    Command { event: CommandEvent },
}

/// Response to a query event.
#[derive(Debug, Clone)]
pub struct EventResponse {
    pub event_type: QueryEvent,
    pub response: String,
}

/// Query handler type.
pub type QueryHandler = Box<dyn Fn(&QueryEvent) -> String + Send + Sync>;

/// Subscriber handler type for emit events.
pub type SubscriberHandler = Box<dyn Fn(&EventType) + Send + Sync>;

/// Filtered subscriber with an optional filter.
struct FilteredSubscriber {
    filter: Option<EventType>,
    handler: SubscriberHandler,
}

/// Thread-safe event bus for inter-component communication in VibePilot.
///
/// Uses [flume](https://docs.rs/flume) channels for async event propagation
/// from the orchestrator to the UI thread, plus synchronous query handlers
/// for request-response patterns.
///
/// # Architecture
///
/// - **Emit events**: Fire-and-forget notifications (logs, status updates).
/// - **Query handlers**: Synchronous request-response (e.g., "is paused?").
/// - **Subscribers**: Callback-based listeners for specific event types.
///
/// # Examples
///
/// ```no_run
/// let (bus, _rx) = EventBus::new();
/// bus.emit_notification(NotificationEvent::Log("Hello".to_string()));
/// bus.register_query(
///     QueryEvent::GetModePauseForcee,
///     Box::new(|_| "false".to_string()),
/// );
/// let paused = bus.emit_query(QueryEvent::GetModePauseForcee);
/// ```
pub struct EventBus {
    sender: flume::Sender<Event>,
    receiver: flume::Receiver<Event>,
    query_handlers: std::sync::Arc<std::sync::Mutex<HashMap<QueryEvent, QueryHandler>>>,
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

    fn notify_subscribers(&self, legacy_event: EventType) {
        if let Ok(subs) = self.subscribers.lock() {
            for sub in subs.iter() {
                if let Some(ref filter) = sub.filter {
                    if *filter != legacy_event {
                        continue;
                    }
                }
                (sub.handler)(&legacy_event);
            }
        }
    }

    /// Emit a notification event.
    pub fn emit_notification(&self, event: NotificationEvent) {
        let _ = self.sender.send(Event::Emit { event: event.clone() });
        self.notify_subscribers(EventType::from(event));
    }

    /// Emit a command event.
    pub fn emit_command(&self, event: CommandEvent) {
        let _ = self.sender.send(Event::Command { event: event.clone() });
        self.notify_subscribers(EventType::from(event));
    }

    /// Subscribe to all events with a callback handler.
    pub fn subscribe(&self, handler: SubscriberHandler) {
        if let Ok(mut subs) = self.subscribers.lock() {
            subs.push(FilteredSubscriber {
                filter: None,
                handler,
            });
        }
    }

    /// Subscribe to a specific event type with a callback handler.
    pub fn subscribe_for(&self, filter: EventType, handler: SubscriberHandler) {
        if let Ok(mut subs) = self.subscribers.lock() {
            subs.push(FilteredSubscriber {
                filter: Some(filter),
                handler,
            });
        }
    }

    /// Register a synchronous query handler for an event type.
    ///
    /// Only one handler can be registered per event type. Registering a new
    /// handler for the same event type replaces the previous one.
    pub fn register_query(&self, event: QueryEvent, handler: QueryHandler) {
        if let Ok(mut handlers) = self.query_handlers.lock() {
            handlers.insert(event, handler);
        }
    }

    /// Emit a synchronous query and return the response string.
    ///
    /// Returns an empty string if no handler is registered or the mutex is poisoned.
    pub fn emit_query(&self, event: QueryEvent) -> String {
        self.notify_subscribers(EventType::from(event.clone()));
        if let Ok(handlers) = self.query_handlers.lock() {
            if let Some(handler) = handlers.get(&event) {
                return handler(&event);
            }
        }
        String::new()
    }

    /// Get the event receiver for polling in the UI loop.
    pub fn receiver(&self) -> flume::Receiver<Event> {
        self.receiver.clone()
    }

    /// Get the number of registered query handlers.
    pub fn query_handler_count(&self) -> usize {
        self.query_handlers.lock().map(|h| h.len()).unwrap_or(0)
    }

    /// Get the number of active subscribers.
    pub fn subscriber_count(&self) -> usize {
        self.subscribers.lock().map(|s| s.len()).unwrap_or(0)
    }

    /// Backward-compatible emit method that dispatches to the appropriate handler.
    pub fn emit(&self, event: EventType) {
        match event {
            EventType::Log(msg) => {
                self.emit_notification(NotificationEvent::Log(msg));
            }
            EventType::UpdateStatus { text, color } => {
                self.emit_notification(NotificationEvent::UpdateStatus { text, color });
            }
            EventType::AppendAction { action, display } => {
                self.emit_notification(NotificationEvent::AppendAction { action, display });
            }
            EventType::AppendReport(report) => {
                self.emit_notification(NotificationEvent::AppendReport(report));
            }
            EventType::LoopDetectedAlert { message } => {
                self.emit_notification(NotificationEvent::LoopDetectedAlert { message });
            }
            EventType::ClearActionConfirmation => {
                self.emit_notification(NotificationEvent::ClearActionConfirmation);
            }
            EventType::SelectTab(idx) => {
                self.emit_command(CommandEvent::SelectTab(idx));
            }
            EventType::UpdateField { field, value } => {
                self.emit_command(CommandEvent::UpdateField { field, value });
            }
            EventType::UpdateAllFields { contexte, task, objectif, directives } => {
                self.emit_command(CommandEvent::UpdateAllFields { contexte, task, objectif, directives });
            }
            EventType::ShowActionConfirmation { action, text, scroll } => {
                self.emit_command(CommandEvent::ShowActionConfirmation { action, text, scroll });
            }
            EventType::SetGeneratingPrompts(status) => {
                self.emit_command(CommandEvent::SetGeneratingPrompts(status));
            }
            EventType::UpdateModelsList { engine, models } => {
                self.emit_command(CommandEvent::UpdateModelsList { engine, models });
            }
            EventType::CreateProfileWithPrompts { profile_name, contexte, task, blueprint, instructions } => {
                self.emit_command(CommandEvent::CreateProfileWithPrompts { profile_name, contexte, task, objectif: blueprint, directives: instructions });
            }
            EventType::ShowProfileReadyPopup { profile_name } => {
                self.emit_command(CommandEvent::ShowProfileReadyPopup { profile_name });
            }
            EventType::StopOrchestrator => {
                self.emit_command(CommandEvent::StopOrchestrator);
            }
            EventType::StartOrchestrator => {
                self.emit_command(CommandEvent::StartOrchestrator);
            }
            EventType::LoadProfile(profile_name) => {
                self.emit_command(CommandEvent::LoadProfile(profile_name));
            }
            // Query events - just ignore for emit (use emit_query instead)
            EventType::GetLangText(_)
            | EventType::GetDernierMouvementHumain
            | EventType::GetModePauseForcee
            | EventType::GetActionConfirmationStatus
            | EventType::GetOrchestratorRunning
            | EventType::GetUserFeedback => {}
        }
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
#[path = "event_bus_tests.rs"]
mod tests;

