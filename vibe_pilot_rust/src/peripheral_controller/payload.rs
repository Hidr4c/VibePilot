use crate::screen_capture::ScreenInfo;

/// Action payload from the LLM response.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct ActionPayload {
    pub action: String,
    pub relative_click_position: Vec<f64>,
    pub text_to_type: String,
    pub scroll_value: i32,
    pub wait_seconds: i32,
}

/// Screen offset info for coordinate translation.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct ScreenOffset {
    pub titre: String,
    pub left: i32,
    pub top: i32,
    pub width: i32,
    pub height: i32,
    pub y_offset: i32,
}

impl ScreenOffset {
    pub fn from_screen_info(screen: ScreenInfo) -> Self {
        ScreenOffset {
            titre: screen.title,
            left: screen.bbox.left,
            top: screen.bbox.top,
            width: screen.bbox.width,
            height: screen.bbox.height,
            y_offset: screen.bbox.top,
        }
    }
}
