/// Tracks action repetitions and applies graduated escalation strategies.
pub struct LoopBreaker {
    pub last_action_key: String,
    pub last_action: String,
    pub last_coords: Option<(f64, f64)>,
    pub last_text: String,
    pub last_scroll_value: i32,
    pub last_keys: Vec<String>,
    pub repetition_count: u32,
    /// 0=normal, 1=warn LLM, 2=desktop capture, 3=keyboard escape, 4=forced scroll, 5=user alert
    pub escalation_level: u8,
    pub total_consecutive_failures: u32,
}

/// The action the orchestrator should take after consulting the LoopBreaker.
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum LoopAction {
    /// No loop detected, proceed normally.
    Normal,
    /// Inject a warning into the LLM prompt.
    WarnLlm,
    /// Capture the full desktop instead of the target window.
    EscalateToDesktop,
    /// Send Escape + Tab keystrokes to break out of a stuck focus.
    SendEscapeKeys,
    /// Force a scroll to change the visible viewport.
    ForceScroll,
    /// Alert the user and pause the orchestrator.
    AlertUser,
}

impl LoopBreaker {
    pub fn new() -> Self {
        Self {
            last_action_key: String::new(),
            last_action: String::new(),
            last_coords: None,
            last_text: String::new(),
            last_scroll_value: 0,
            last_keys: Vec::new(),
            repetition_count: 0,
            escalation_level: 0,
            total_consecutive_failures: 0,
        }
    }

    /// Register a new action using precise details and return the loop-breaking strategy.
    pub fn register_action_precise(
        &mut self,
        action_key: &str,
        action: &str,
        coords: Option<(f64, f64)>,
        text: &str,
        scroll_value: i32,
        keys: &[String],
    ) -> LoopAction {
        let is_repetition = if self.last_action_key.is_empty() {
            false
        } else if action != self.last_action {
            false
        } else {
            // Compare coordinates if present
            let coords_match = match (coords, self.last_coords) {
                (Some((x1, y1)), Some((x2, y2))) => {
                    let dx = x1 - x2;
                    let dy = y1 - y2;
                    let dist = (dx * dx + dy * dy).sqrt();
                    dist < 0.03 // Euclidean distance < 3% of screen space
                }
                (None, None) => true,
                _ => false,
            };

            coords_match
                && text == self.last_text
                && scroll_value == self.last_scroll_value
                && keys == self.last_keys.as_slice()
        };

        if is_repetition {
            self.repetition_count += 1;
            self.total_consecutive_failures += 1;
            self.escalation_level = match self.repetition_count {
                0 => 0,
                1 => 1, // WarnLlm
                2 => 1, // WarnLlm (Warning 2)
                3 => 1, // WarnLlm (Warning 3)
                4 => 2, // EscalateToDesktop
                5 => 3, // SendEscapeKeys
                6 => 4, // ForceScroll
                _ => 5, // AlertUser (escalation level 5 triggers the user alert and pauses)
            };
        } else {
            self.last_action_key = action_key.to_string();
            self.last_action = action.to_string();
            self.last_coords = coords;
            self.last_text = text.to_string();
            self.last_scroll_value = scroll_value;
            self.last_keys = keys.to_vec();
            self.repetition_count = 0;
            self.escalation_level = 0;
            self.total_consecutive_failures = 0;
        }

        match self.escalation_level {
            0 => LoopAction::Normal,
            1 => LoopAction::WarnLlm,
            2 => LoopAction::EscalateToDesktop,
            3 => LoopAction::SendEscapeKeys,
            4 => LoopAction::ForceScroll,
            _ => LoopAction::AlertUser,
        }
    }

    /// Register a new action with just the formatted action key (backward compatibility).
    pub fn register_action(&mut self, action_key: &str) -> LoopAction {
        let parts: Vec<&str> = action_key.split(':').collect();
        let action = parts.first().copied().unwrap_or("");
        let x = parts.get(1).and_then(|v| v.parse::<f64>().ok());
        let y = parts.get(2).and_then(|v| v.parse::<f64>().ok());
        let coords = match (x, y) {
            (Some(xv), Some(yv)) => Some((xv, yv)),
            _ => None,
        };
        self.register_action_precise(action_key, action, coords, "", 0, &[])
    }

    /// Returns the current repetition count for prompt injection.
    pub fn repetition_count(&self) -> u32 {
        self.repetition_count
    }

    /// Reset escalation (e.g., after a successful different action).
    pub fn reset(&mut self) {
        self.last_action_key.clear();
        self.last_action.clear();
        self.last_coords = None;
        self.last_text.clear();
        self.last_scroll_value = 0;
        self.last_keys.clear();
        self.repetition_count = 0;
        self.escalation_level = 0;
        self.total_consecutive_failures = 0;
    }
}

impl Default for LoopBreaker {
    fn default() -> Self {
        Self::new()
    }
}

