//! Macro action recording and management module for VibePilot.
//!
//! Provides global keyboard and mouse event capture, interactive video-game style
//! keybinding capture, key name normalization, intelligent key hold duration tracking,
//! multi-run session management, sequence modeling, multi-key shortcuts, drag & drop tracking,
//! node editing support, IoC traits & factories, and persistent macro storage.

pub mod player;
pub mod storage;
pub mod script_parser;

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::time::Instant;

/// Helper to normalize raw `rdev::Key` names into human-readable hotkey labels.
pub fn normalize_key_name(key: &rdev::Key) -> String {
    let raw = format!("{:?}", key);
    match raw.as_str() {
        "Alt" | "AltLeft" | "AltRight" => "Alt".to_string(),
        "ControlLeft" | "ControlRight" | "Ctrl" => "Ctrl".to_string(),
        "ShiftLeft" | "ShiftRight" | "Shift" => "Shift".to_string(),
        "MetaLeft" | "MetaRight" | "Win" => "Win".to_string(),
        "Space" => "Space".to_string(),
        k if k.starts_with("Key") && k.len() == 4 => k[3..].to_string(),
        k => k.to_string(),
    }
}

/// Type of recorded input action.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum ActionKind {
    Click {
        x: i32,
        y: i32,
        button: String,
        double_click: bool,
    },
    MouseDown {
        x: i32,
        y: i32,
        button: String,
    },
    MouseUp {
        x: i32,
        y: i32,
        button: String,
    },
    DragAndDrop {
        from_x: i32,
        from_y: i32,
        to_x: i32,
        to_y: i32,
        button: String,
    },
    Move {
        x: i32,
        y: i32,
    },
    KeyPress {
        key: String,
    },
    KeyHold {
        key: String,
        hold_duration_ms: u64,
    },
    KeyCombo {
        keys: Vec<String>,
    },
    Scroll {
        dx: i32,
        dy: i32,
    },
    Wait {
        ms: u64,
    },
}

impl ActionKind {
    pub fn description(&self) -> String {
        match self {
            ActionKind::Click { x, y, button, double_click } => {
                if *double_click {
                    format!("Double Clic ({button}) à ({x}, {y})")
                } else {
                    format!("Clic ({button}) à ({x}, {y})")
                }
            }
            ActionKind::MouseDown { x, y, button } => format!("Bouton enfoncé ({button}) à ({x}, {y})"),
            ActionKind::MouseUp { x, y, button } => format!("Bouton relâché ({button}) à ({x}, {y})"),
            ActionKind::DragAndDrop { from_x, from_y, to_x, to_y, button } => {
                format!("Glisser-déposer ({button}) de ({from_x}, {from_y}) ➔ ({to_x}, {to_y})")
            }
            ActionKind::Move { x, y } => format!("Déplacement à ({x}, {y})"),
            ActionKind::KeyPress { key } => format!("Touche '{key}'"),
            ActionKind::KeyHold { key, hold_duration_ms } => {
                format!("Touche '{key}' maintenue pendant {hold_duration_ms} ms")
            }
            ActionKind::KeyCombo { keys } => format!("Combinaison: {}", keys.join(" + ")),
            ActionKind::Scroll { dx, dy } => format!("Défilement (dx: {dx}, dy: {dy})"),
            ActionKind::Wait { ms } => format!("Attente {ms} ms"),
        }
    }
}

/// A single recorded action node inside the macro sequence.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RecordedAction {
    pub id: u64,
    pub delay_ms: u64,
    pub action: ActionKind,
    pub label: String,
    pub enabled: bool,
}

impl RecordedAction {
    pub fn new(id: u64, delay_ms: u64, action: ActionKind) -> Self {
        let label = action.description();
        Self {
            id,
            delay_ms,
            action,
            label,
            enabled: true,
        }
    }
}

/// Sequence of recorded actions.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct MacroSequence {
    pub name: String,
    pub actions: Vec<RecordedAction>,
    pub next_id: u64,
    pub global_delay_override_ms: Option<u64>,
}

impl MacroSequence {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            actions: Vec::new(),
            next_id: 1,
            global_delay_override_ms: None,
        }
    }

    pub fn add_action(&mut self, delay_ms: u64, action: ActionKind) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.actions.push(RecordedAction::new(id, delay_ms, action));
        id
    }

    pub fn remove_action(&mut self, id: u64) -> bool {
        let original_len = self.actions.len();
        self.actions.retain(|a| a.id != id);
        self.actions.len() < original_len
    }

    pub fn get_action_mut(&mut self, id: u64) -> Option<&mut RecordedAction> {
        self.actions.iter_mut().find(|a| a.id == id)
    }

    pub fn clear(&mut self) {
        self.actions.clear();
        self.next_id = 1;
    }
}

// === HOTKEY CONFIGURATION & INTERACTIVE BINDING ===

/// Target hotkey action currently being interactively rebound.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BindingTarget {
    StartRecording,
    StopRecording,
}

/// Configurable global hotkeys for start/stop recording.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HotkeyConfig {
    pub start_recording_keys: Vec<String>,
    pub stop_recording_keys: Vec<String>,
}

impl Default for HotkeyConfig {
    fn default() -> Self {
        Self {
            start_recording_keys: vec!["Alt".to_string(), "S".to_string()],
            stop_recording_keys: vec!["Alt".to_string(), "Space".to_string()],
        }
    }
}

impl HotkeyConfig {
    pub fn matches_keys(&self, pressed: &HashSet<String>, target_keys: &[String]) -> bool {
        if target_keys.is_empty() || pressed.is_empty() {
            return false;
        }
        let normalized_pressed: HashSet<String> = pressed.iter().map(|k| match k.as_str() {
            "AltLeft" | "AltRight" => "Alt".to_string(),
            "ControlLeft" | "ControlRight" => "Ctrl".to_string(),
            "ShiftLeft" | "ShiftRight" => "Shift".to_string(),
            "MetaLeft" | "MetaRight" => "Win".to_string(),
            k if k.starts_with("Key") && k.len() == 4 => k[3..].to_string(),
            other => other.to_string(),
        }).collect();

        target_keys.iter().all(|k| {
            normalized_pressed.contains(k) || pressed.contains(k)
        })
    }
}

// === RUN SESSION MANAGEMENT ===

fn default_iterations() -> u32 {
    1
}

/// Execution statistics for a single recording Run.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct MacroStats {
    pub total_launches: u64,
    pub total_iterations_executed: u64,
    pub completed_runs_count: u64,
    pub interruption_count: u64,
}

/// A single recording Run session.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MacroRun {
    pub id: String,
    pub name: String,
    pub sequence: MacroSequence,
    #[serde(default = "default_iterations")]
    pub iterations: u32,
    #[serde(default)]
    pub start_delay_sec: u32,
    #[serde(default)]
    pub record_start_delay_sec: u32,
    #[serde(default)]
    pub stats: MacroStats,
}

impl MacroRun {
    pub fn new(id: impl Into<String>, name: impl Into<String>, sequence: MacroSequence) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            sequence,
            iterations: 1,
            start_delay_sec: 0,
            record_start_delay_sec: 0,
            stats: MacroStats::default(),
        }
    }
}

/// Manager handling multi-run recording sessions.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MacroSessionManager {
    pub runs: Vec<MacroRun>,
    pub active_index: usize,
}

impl Default for MacroSessionManager {
    fn default() -> Self {
        let initial_run = MacroRun::new("run_1", "Run #1", MacroSequence::new("Run #1"));
        Self {
            runs: vec![initial_run],
            active_index: 0,
        }
    }
}

impl MacroSessionManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn active_run(&self) -> Option<&MacroRun> {
        self.runs.get(self.active_index)
    }

    pub fn active_run_mut(&mut self) -> Option<&mut MacroRun> {
        self.runs.get_mut(self.active_index)
    }

    pub fn add_new_run(&mut self) -> usize {
        let run_number = self.runs.len() + 1;
        let id = format!("run_{run_number}");
        let name = format!("Run #{run_number}");
        let new_run = MacroRun::new(id, name.clone(), MacroSequence::new(name));
        self.runs.push(new_run);
        self.active_index = self.runs.len() - 1;
        self.active_index
    }

    pub fn select_first(&mut self) {
        if !self.runs.is_empty() {
            self.active_index = 0;
        }
    }

    pub fn select_previous(&mut self) {
        if self.active_index > 0 {
            self.active_index -= 1;
        }
    }

    pub fn select_next(&mut self) {
        if self.active_index + 1 < self.runs.len() {
            self.active_index += 1;
        }
    }

    pub fn select_last(&mut self) {
        if !self.runs.is_empty() {
            self.active_index = self.runs.len() - 1;
        }
    }

    pub fn delete_active_run(&mut self) -> bool {
        if self.runs.len() <= 1 {
            if let Some(run) = self.runs.get_mut(0) {
                run.sequence.clear();
            }
            return false;
        }

        self.runs.remove(self.active_index);
        if self.active_index >= self.runs.len() {
            self.active_index = self.runs.len() - 1;
        }
        true
    }
}

// === IoC Interface Trait & Factory ===

/// Abstraction for global input macro recording.
pub trait MacroRecorderTrait: Send + Sync {
    fn is_recording(&self) -> bool;
    fn start_recording(&self);
    fn stop_recording(&self);
    fn get_sequence(&self) -> MacroSequence;
    fn set_sequence(&self, seq: MacroSequence);
    fn clear(&self);
    fn set_hotkeys(&self, config: HotkeyConfig);
    fn get_hotkeys(&self) -> HotkeyConfig;
    fn start_binding_capture(&self, target: BindingTarget);
    fn active_binding_capture(&self) -> Option<BindingTarget>;
    fn get_captured_binding_keys(&self) -> Vec<String>;
    fn commit_binding_capture(&self);
    fn cancel_binding_capture(&self);
}

pub struct MacroRecorderFactory;

impl MacroRecorderFactory {
    pub fn create() -> Arc<dyn MacroRecorderTrait> {
        Arc::new(MacroRecorder::new())
    }
}

/// State tracker during live recording to collapse drag-and-drop, multi-key combos & key hold duration.
#[derive(Default)]
struct LiveStateTracker {
    pressed_keys: HashSet<String>,
    key_down_instants: HashMap<String, Instant>,
    current_mouse_pos: (i32, i32),
    mouse_down: Option<(i32, i32, String)>,
}

/// Concrete implementation of MacroRecorderTrait.
pub struct MacroRecorder {
    recording: Arc<AtomicBool>,
    sequence: Arc<Mutex<MacroSequence>>,
    last_event_time: Arc<Mutex<Option<Instant>>>,
    hotkeys: Arc<Mutex<HotkeyConfig>>,
    binding_target: Arc<Mutex<Option<BindingTarget>>>,
    captured_binding_keys: Arc<Mutex<Vec<String>>>,
}

impl Default for MacroRecorder {
    fn default() -> Self {
        Self::new()
    }
}

impl MacroRecorder {
    pub fn new() -> Self {
        let rec = Self {
            recording: Arc::new(AtomicBool::new(false)),
            sequence: Arc::new(Mutex::new(MacroSequence::new("Enregistrement"))),
            last_event_time: Arc::new(Mutex::new(None)),
            hotkeys: Arc::new(Mutex::new(HotkeyConfig::default())),
            binding_target: Arc::new(Mutex::new(None)),
            captured_binding_keys: Arc::new(Mutex::new(Vec::new())),
        };
        rec.spawn_listener();
        rec
    }

    fn spawn_listener(&self) {
        let recording_flag = self.recording.clone();
        let sequence_ref = self.sequence.clone();
        let last_time_ref = self.last_event_time.clone();
        let hotkeys_ref = self.hotkeys.clone();
        let binding_target_ref = self.binding_target.clone();
        let captured_keys_ref = self.captured_binding_keys.clone();

        std::thread::spawn(move || {
            let tracker = Arc::new(Mutex::new(LiveStateTracker::default()));

            let callback = move |event: rdev::Event| {
                let now = Instant::now();
                let is_rec_now = recording_flag.load(Ordering::Relaxed);

                // --- 1. INTERACTIVE KEYBINDING CAPTURE MODE ("Press key to bind") ---
                let current_binding_target = binding_target_ref.lock().ok().and_then(|g| *g);
                if let Some(target) = current_binding_target {
                    match event.event_type {
                        rdev::EventType::KeyPress(key) => {
                            let norm_key = normalize_key_name(&key);
                            if let Ok(mut keys_guard) = captured_keys_ref.lock() {
                                if !keys_guard.contains(&norm_key) {
                                    keys_guard.push(norm_key);
                                }
                            }
                        }
                        rdev::EventType::KeyRelease(_key) => {
                            let keys = captured_keys_ref
                                .lock()
                                .ok()
                                .map(|g| g.clone())
                                .unwrap_or_default();

                            if !keys.is_empty() {
                                // Sort modifiers first (Alt, Ctrl, Shift, Win) then other keys
                                let mut sorted_keys = keys.clone();
                                sorted_keys.sort_by_key(|k| match k.as_str() {
                                    "Ctrl" => 0,
                                    "Alt" => 1,
                                    "Shift" => 2,
                                    "Win" => 3,
                                    _ => 4,
                                });

                                if let Ok(mut hk) = hotkeys_ref.lock() {
                                    match target {
                                        BindingTarget::StartRecording => hk.start_recording_keys = sorted_keys,
                                        BindingTarget::StopRecording => hk.stop_recording_keys = sorted_keys,
                                    }
                                }
                                if let Ok(mut b) = binding_target_ref.lock() {
                                    *b = None;
                                }
                                if let Ok(mut keys_guard) = captured_keys_ref.lock() {
                                    keys_guard.clear();
                                }
                            }
                        }
                        _ => {}
                    }
                    return;
                }

                // --- 2. REGULAR RECORDING & HOTKEY EVALUATION ---
                let delay_ms = if is_rec_now {
                    if let Ok(mut time_guard) = last_time_ref.lock() {
                        let delay = time_guard
                            .map(|t| now.duration_since(t).as_millis() as u64)
                            .unwrap_or(0);
                        *time_guard = Some(now);
                        delay
                    } else {
                        0
                    }
                } else {
                    0
                };

                let mut tracker_guard = match tracker.lock() {
                    Ok(g) => g,
                    Err(_) => return,
                };

                let mut action_to_emit: Option<ActionKind> = None;

                match event.event_type {
                    rdev::EventType::MouseMove { x, y } => {
                        tracker_guard.current_mouse_pos = (x as i32, y as i32);
                    }
                    rdev::EventType::ButtonPress(btn) => {
                        let btn_str = match btn {
                            rdev::Button::Left => "Left",
                            rdev::Button::Right => "Right",
                            rdev::Button::Middle => "Middle",
                            rdev::Button::Unknown(_code) => "Unknown",
                        }.to_string();

                        tracker_guard.mouse_down = Some((
                            tracker_guard.current_mouse_pos.0,
                            tracker_guard.current_mouse_pos.1,
                            btn_str,
                        ));
                    }
                    rdev::EventType::ButtonRelease(btn) => {
                        let btn_str = match btn {
                            rdev::Button::Left => "Left",
                            rdev::Button::Right => "Right",
                            rdev::Button::Middle => "Middle",
                            rdev::Button::Unknown(_code) => "Unknown",
                        }.to_string();

                        if let Some((from_x, from_y, down_btn)) = tracker_guard.mouse_down.take() {
                            let (to_x, to_y) = tracker_guard.current_mouse_pos;
                            let dx = (to_x - from_x).abs();
                            let dy = (to_y - from_y).abs();

                            if dx > 8 || dy > 8 {
                                action_to_emit = Some(ActionKind::DragAndDrop {
                                    from_x,
                                    from_y,
                                    to_x,
                                    to_y,
                                    button: down_btn,
                                });
                            } else {
                                action_to_emit = Some(ActionKind::Click {
                                    x: to_x,
                                    y: to_y,
                                    button: btn_str,
                                    double_click: false,
                                });
                            }
                        }
                    }
                    rdev::EventType::KeyPress(key) => {
                        let key_name = normalize_key_name(&key);
                        tracker_guard.pressed_keys.insert(key_name.clone());
                        tracker_guard.key_down_instants.entry(key_name.clone()).or_insert(now);

                        // Check for global hotkeys (start/stop)
                        if let Ok(hk) = hotkeys_ref.lock() {
                            if !is_rec_now && hk.matches_keys(&tracker_guard.pressed_keys, &hk.start_recording_keys) {
                                recording_flag.store(true, Ordering::SeqCst);
                                return;
                            } else if is_rec_now && hk.matches_keys(&tracker_guard.pressed_keys, &hk.stop_recording_keys) {
                                recording_flag.store(false, Ordering::SeqCst);
                                return;
                            }
                        }

                        if !is_rec_now {
                            return;
                        }

                        if tracker_guard.pressed_keys.len() > 1 {
                            let combo_keys: Vec<String> = tracker_guard.pressed_keys.iter().cloned().collect();
                            action_to_emit = Some(ActionKind::KeyCombo { keys: combo_keys });
                        } else {
                            action_to_emit = Some(ActionKind::KeyPress { key: key_name });
                        }
                    }
                    rdev::EventType::KeyRelease(key) => {
                        let key_name = normalize_key_name(&key);
                        tracker_guard.pressed_keys.remove(&key_name);

                        if let Some(down_time) = tracker_guard.key_down_instants.remove(&key_name) {
                            let hold_dur = now.duration_since(down_time).as_millis() as u64;
                            if is_rec_now && hold_dur > 150 {
                                action_to_emit = Some(ActionKind::KeyHold {
                                    key: key_name,
                                    hold_duration_ms: hold_dur,
                                });
                            }
                        }
                    }
                    rdev::EventType::Wheel { delta_x, delta_y } => {
                        if is_rec_now {
                            action_to_emit = Some(ActionKind::Scroll {
                                dx: delta_x as i32,
                                dy: delta_y as i32,
                            });
                        }
                    }
                }

                if is_rec_now {
                    if let Some(act) = action_to_emit {
                        if let Ok(mut seq) = sequence_ref.lock() {
                            if let ActionKind::KeyCombo { ref keys } = act {
                                if let Some(last) = seq.actions.last() {
                                    if let ActionKind::KeyCombo { keys: ref last_keys } = last.action {
                                        if keys == last_keys && delay_ms < 100 {
                                            return;
                                        }
                                    }
                                }
                            }
                            seq.add_action(delay_ms, act);
                        }
                    }
                }
            };

            if let Err(err) = rdev::listen(callback) {
                eprintln!("MacroRecorder background listener error: {:?}", err);
            }
        });
    }
}

impl MacroRecorderTrait for MacroRecorder {
    fn is_recording(&self) -> bool {
        self.recording.load(Ordering::Relaxed)
    }

    fn start_recording(&self) {
        if self.is_recording() {
            return;
        }
        self.recording.store(true, Ordering::SeqCst);
        if let Ok(mut time_guard) = self.last_event_time.lock() {
            *time_guard = Some(Instant::now());
        }
    }

    fn stop_recording(&self) {
        self.recording.store(false, Ordering::SeqCst);
    }

    fn get_sequence(&self) -> MacroSequence {
        self.sequence.lock().map(|s| s.clone()).unwrap_or_default()
    }

    fn set_sequence(&self, seq: MacroSequence) {
        if let Ok(mut s) = self.sequence.lock() {
            *s = seq;
        }
    }

    fn clear(&self) {
        if let Ok(mut s) = self.sequence.lock() {
            s.clear();
        }
    }

    fn set_hotkeys(&self, config: HotkeyConfig) {
        if let Ok(mut h) = self.hotkeys.lock() {
            *h = config;
        }
    }

    fn get_hotkeys(&self) -> HotkeyConfig {
        self.hotkeys.lock().map(|h| h.clone()).unwrap_or_default()
    }

    fn start_binding_capture(&self, target: BindingTarget) {
        if let Ok(mut keys) = self.captured_binding_keys.lock() {
            keys.clear();
        }
        if let Ok(mut b) = self.binding_target.lock() {
            *b = Some(target);
        }
    }

    fn active_binding_capture(&self) -> Option<BindingTarget> {
        self.binding_target.lock().ok().and_then(|g| *g)
    }

    fn get_captured_binding_keys(&self) -> Vec<String> {
        self.captured_binding_keys.lock().ok().map(|g| g.clone()).unwrap_or_default()
    }

    fn commit_binding_capture(&self) {
        let current_target = self.active_binding_capture();
        let keys = self.get_captured_binding_keys();
        if let Some(target) = current_target {
            if !keys.is_empty() {
                if let Ok(mut hk) = self.hotkeys.lock() {
                    match target {
                        BindingTarget::StartRecording => hk.start_recording_keys = keys,
                        BindingTarget::StopRecording => hk.stop_recording_keys = keys,
                    }
                }
            }
        }
        self.cancel_binding_capture();
    }

    fn cancel_binding_capture(&self) {
        if let Ok(mut b) = self.binding_target.lock() {
            *b = None;
        }
        if let Ok(mut keys) = self.captured_binding_keys.lock() {
            keys.clear();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hotkey_matching() {
        let hk = HotkeyConfig::default();
        let mut pressed = HashSet::new();
        pressed.insert("Alt".to_string());
        pressed.insert("S".to_string());
        assert!(hk.matches_keys(&pressed, &hk.start_recording_keys));
    }

    #[test]
    fn test_binding_capture_toggle() {
        let recorder = MacroRecorder::new();
        assert_eq!(recorder.active_binding_capture(), None);

        recorder.start_binding_capture(BindingTarget::StartRecording);
        assert_eq!(recorder.active_binding_capture(), Some(BindingTarget::StartRecording));

        recorder.cancel_binding_capture();
        assert_eq!(recorder.active_binding_capture(), None);
    }
}
