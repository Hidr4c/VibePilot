use serde::{Deserialize, Serialize};

/// API response structure from the LLM.
/// Confidence level returned by the LLM for its coordinate prediction.
///
/// Range: 0.0 (completely uncertain) to 1.0 (absolutely certain).
/// Values below 0.6 typically trigger a zoom-based refinement.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct LlmResponse {
    pub status_display: String,
    pub action: String,
    /// Confidence score for the predicted coordinates (0.0 to 1.0).
    /// Defaults to 1.0 if omitted by the LLM.
    #[serde(default = "default_confidence")]
    pub confidence: f32,
    #[serde(default)]
    pub relative_click_position: Vec<f64>,
    #[serde(default)]
    pub text_to_type: String,
    #[serde(default)]
    pub scroll_value: i32,
    #[serde(default)]
    pub wait_seconds: i32,
    #[serde(default)]
    pub report: Option<String>,
    #[serde(default)]
    pub keys_to_press: Vec<String>,
    #[serde(default)]
    pub clipboard_op: String,
    #[serde(default)]
    pub drag_from: Vec<f64>,
    #[serde(default)]
    pub drag_to: Vec<f64>,
    #[serde(default)]
    pub relative_move: Vec<f64>,
    #[serde(default)]
    pub scroll_mode: String,
    #[serde(default)]
    pub scroll_direction: String,
    #[serde(default)]
    pub duration_secs: Option<u32>,
    #[serde(default)]
    pub condition: Option<String>,
    #[serde(default)]
    pub shortcut_name: String,
    #[serde(default)]
    pub min_delay_ms: u64,
    #[serde(default)]
    pub max_delay_ms: u64,
    #[serde(default)]
    pub duration_ms: u64,
}

pub fn default_confidence() -> f32 {
    1.0
}

/// Structured wait condition parsed from the LLM or heuristics.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum WaitCondition {
    TimeElapsed,
    ScreenStable { seconds: u32 },
    ElementAppears { description: String },
    VideoEnd,
}

impl WaitCondition {
    /// Parses a condition string and an execution duration into a structured WaitCondition.
    pub fn from_str(cond_str: Option<&str>, default_duration: u32) -> Self {
        match cond_str {
            None => WaitCondition::TimeElapsed,
            Some(s) => {
                let s = s.trim();
                let lower = s.to_lowercase();
                if lower == "time_elapsed" || lower.is_empty() {
                    WaitCondition::TimeElapsed
                } else if lower == "video_end" || lower == "video_finished" {
                    WaitCondition::VideoEnd
                } else if lower.starts_with("screen_stable") {
                    let secs = lower.split(':')
                        .nth(1)
                        .or_else(|| lower.split_whitespace().nth(1))
                        .and_then(|val| val.trim().parse::<u32>().ok())
                        .unwrap_or(default_duration.min(5));
                    WaitCondition::ScreenStable { seconds: secs }
                } else if lower.starts_with("element_appears") {
                    let desc = s.split_once(':')
                        .map(|x| x.1.trim().to_string())
                        .unwrap_or_else(|| "element".to_string());
                    WaitCondition::ElementAppears { description: desc }
                } else {
                    WaitCondition::ElementAppears { description: s.to_string() }
                }
            }
        }
    }
}

#[derive(Debug, Serialize)]
pub struct LlmRequest {
    pub model: String,
    pub messages: Vec<Message>,
    pub temperature: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct Message {
    pub role: String,
    pub content: Vec<MessageContent>,
}

#[derive(Debug, Serialize)]
pub struct MessageContent {
    #[serde(rename = "type")]
    pub content_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_url: Option<ImageUrl>,
}

#[derive(Debug, Serialize)]
pub struct ImageUrl {
    pub url: String,
}
