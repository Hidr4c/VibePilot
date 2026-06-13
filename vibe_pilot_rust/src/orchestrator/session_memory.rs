use std::collections::VecDeque;
use serde::{Serialize, Deserialize};

pub const MAX_HISTORY_STEPS: usize = 7;

/// A single recorded action step.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ActionStep {
    pub timestamp: String,
    pub action_type: String,
    pub coordinates: Option<(f64, f64)>,
    pub text_typed: Option<String>,
    pub llm_report: String,
    pub was_repeated: bool,
}

/// Snapshot of state before an action, for undo purposes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ActionSnapshot {
    pub timestamp: String,
    pub action_type: String,
    pub action_description: String,
    pub relative_click_position: Vec<f64>,
    pub text_typed: Option<String>,
    pub scroll_value: i32,
    pub screenshot_hash: u64,
    pub window_title: String,
    pub reflection_feedback: Option<String>,
    pub active_task_id: Option<u64>,
}

/// Structured session memory providing compact context to the LLM.
pub struct SessionMemory {
    pub steps: VecDeque<ActionStep>,
    pub total_actions_count: u32,
    pub successful_clicks: u32,
    pub scroll_count: u32,
    pub failed_attempts: u32,
    pub session_start: std::time::Instant,
    pub last_significant_change: Option<std::time::Instant>,
    pub current_phase: String,
    pub compressed_history: String,
    pub max_steps: usize,
}

impl SessionMemory {
    pub fn new() -> Self {
        Self {
            steps: VecDeque::new(),
            total_actions_count: 0,
            successful_clicks: 0,
            scroll_count: 0,
            failed_attempts: 0,
            session_start: std::time::Instant::now(),
            last_significant_change: None,
            current_phase: String::new(),
            compressed_history: String::new(),
            max_steps: MAX_HISTORY_STEPS,
        }
    }

    /// Record a new action step into memory.
    pub fn record(&mut self, step: ActionStep) {
        self.total_actions_count += 1;
        match step.action_type.as_str() {
            "CLICK_AND_TYPE" => self.successful_clicks += 1,
            "SCROLL" => self.scroll_count += 1,
            "FAIL" => self.failed_attempts += 1,
            _ => {}
        }

        if !step.llm_report.is_empty() && step.llm_report.len() > 10 {
            let phase_end = step.llm_report.char_indices()
                .nth(80)
                .map(|(i, _)| i)
                .unwrap_or(step.llm_report.len());
            self.current_phase = step.llm_report[..phase_end].to_string();
        }

        self.steps.push_back(step);
        while self.steps.len() > self.max_steps {
            self.steps.pop_front();
        }
    }

    /// Note that a significant visual change occurred.
    pub fn mark_significant_change(&mut self) {
        self.last_significant_change = Some(std::time::Instant::now());
    }

    /// Format the session memory as a compact prompt section for the LLM.
    pub fn format_for_prompt(&self, langue: &str) -> String {
        let elapsed = self.session_start.elapsed().as_secs();
        let minutes = elapsed / 60;
        let seconds = elapsed % 60;

        let stale_warning = if let Some(last_change) = self.last_significant_change {
            let since_change = last_change.elapsed().as_secs();
            if since_change > 30 {
                if langue == "Français" {
                    format!("\n⚠️ ALERTE : Aucun changement visuel significatif depuis {}s.", since_change)
                } else {
                    format!("\n⚠️ ALERT: No significant visual change detected for {}s.", since_change)
                }
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        let phase_line = if !self.current_phase.is_empty() {
            if langue == "Français" {
                format!("\n🎯 PHASE ACTUELLE : {}", self.current_phase)
            } else {
                format!("\n🎯 CURRENT PHASE: {}", self.current_phase)
            }
        } else {
            String::new()
        };

        let stats_line = if langue == "Français" {
            format!("\n📊 STATS SESSION : {} actions total | {} clics | {} scrolls | {} échecs | Durée: {}m{:02}s",
                self.total_actions_count, self.successful_clicks, self.scroll_count,
                self.failed_attempts, minutes, seconds)
        } else {
            format!("\n📊 SESSION STATS: {} actions total | {} clicks | {} scrolls | {} failures | Running for {}m{:02}s",
                self.total_actions_count, self.successful_clicks, self.scroll_count,
                self.failed_attempts, minutes, seconds)
        };

        let history_summary = if !self.compressed_history.is_empty() {
            if langue == "Français" {
                format!("\n📜 RÉSUMÉ DE L'HISTORIQUE COMPRESSÉ :\n{}\n", self.compressed_history)
            } else {
                format!("\n📜 COMPRESSED HISTORY SUMMARY:\n{}\n", self.compressed_history)
            }
        } else {
            String::new()
        };

        let history_title = if langue == "Français" {
            "\n📜 DERNIÈRES ACTIONS :"
        } else {
            "\n📜 RECENT ACTIONS:"
        };

        let mut history_lines = String::new();
        for step in &self.steps {
            let coords = step.coordinates
                .map(|(x, y)| format!("({:.3}, {:.3})", x, y))
                .unwrap_or_default();
            let text_info = step.text_typed.as_ref()
                .filter(|t| !t.is_empty())
                .map(|t| {
                    let preview = if t.len() > 30 { format!("{}...", &t[..27]) } else { t.clone() };
                    format!(" typed: \"{}\"", preview)
                })
                .unwrap_or_default();
            let repeat_marker = if step.was_repeated { " [REPEATED]" } else { "" };
            let report_preview = if step.llm_report.len() > 60 {
                format!("{}...", &step.llm_report[..57])
            } else {
                step.llm_report.clone()
            };
            history_lines.push_str(&format!("\n- [{}] {} {}{}{} → \"{}\"",
                step.timestamp, step.action_type, coords, text_info, repeat_marker, report_preview));
        }

        format!("{}{}{}{}{}{}", history_summary, stats_line, phase_line, stale_warning, history_title, history_lines)
    }
}

impl Default for SessionMemory {
    fn default() -> Self {
        Self::new()
    }
}
