//! Decision router for VibePilot.
//!
//! Separates fast actions (local heuristics like spinner/modal detection, state reuse)
//! from slow actions (full LLM query).

use crate::llm_client::LlmResponse;
use crate::orchestrator::ActionStep;
use crate::screen_capture::ScreenCapturerTrait;
use crate::ocr::TextDetector;
use image::DynamicImage;
use std::sync::Arc;

/// The decision route result.
#[derive(Debug, PartialEq)]
pub enum DecisionRoute {
    /// Action resolved locally without calling the LLM.
    Fast(Box<LlmResponse>),
    /// Action requires consulting the LLM.
    Slow,
}

/// Context passed to the decision router.
pub struct DecisionContext<'a> {
    pub screenshot: &'a DynamicImage,
    pub last_screenshot: Option<&'a DynamicImage>,
    pub last_action: Option<&'a ActionStep>,
    pub current_subtask_description: Option<&'a str>,
    pub target_window_title: Option<&'a str>,
    pub langue: &'a str,
}

/// Router that identifies if an action can be completed via local heuristics.
pub struct DecisionRouter {
    capturer: Arc<dyn ScreenCapturerTrait>,
    text_detector: Option<Arc<dyn TextDetector>>,
}

impl DecisionRouter {
    pub fn new(capturer: Arc<dyn ScreenCapturerTrait>) -> Self {
        DecisionRouter {
            capturer,
            text_detector: None,
        }
    }

    /// Sets a custom text detector for modal dialog button detection.
    pub fn with_text_detector(mut self, detector: Arc<dyn TextDetector>) -> Self {
        self.text_detector = Some(detector);
        self
    }

    /// Evaluates if the current screen state can be routed via a fast action.
    pub fn route(&self, ctx: &DecisionContext) -> DecisionRoute {
        // 1. Detect modal dialog with OK/Cancel buttons
        if let Some((rx, ry)) = self.detect_modal_dialog(ctx) {
            let status = if ctx.langue == "Français" {
                "Fast : Clic sur bouton OK de la modale"
            } else {
                "Fast: Click OK button on modal dialog"
            }.to_string();

            let report = if ctx.langue == "Français" {
                "Boîte de dialogue modale détectée en premier plan. Clic sur OK estimé localement."
            } else {
                "Modal dialog box detected in foreground. Estimated OK click resolved locally."
            }.to_string();

            return DecisionRoute::Fast(Box::new(LlmResponse {
                status_display: status,
                action: "CLICK_AND_TYPE".to_string(),
                relative_click_position: vec![rx, ry],
                wait_seconds: 5,
                report: Some(report),
                ..Default::default()
            }));
        }

        // 2. Detect loading screen or animated spinner
        if let Some(last_screenshot) = ctx.last_screenshot {
            if self.detect_spinner(ctx.screenshot, last_screenshot) {
                // If spinner is detected, we WAIT
                let status = if ctx.langue == "Français" {
                    "Fast : Attente du chargement..."
                } else {
                    "Fast: Waiting for loading..."
                }.to_string();

                let report = if ctx.langue == "Français" {
                    "Spinner/page de chargement détecté(e) via les modifications locales de l'écran."
                } else {
                    "Spinner/loading page detected via small localized screen changes."
                }.to_string();

                return DecisionRoute::Fast(Box::new(LlmResponse {
                    status_display: status,
                    action: "WAIT".to_string(),
                    wait_seconds: 30,
                    duration_secs: Some(30),
                    condition: Some("screen_stable".to_string()),
                    report: Some(report),
                    ..Default::default()
                }));
            }

            // 3. Detect identical screen and subtask -> reuse last strategy if appropriate
            if self.is_screen_identical(ctx.screenshot, last_screenshot) {
                if let Some(last_act) = ctx.last_action {
                    // Only reuse if last subtask is unchanged or we are in the same workflow
                    let reuse_allowed = match last_act.action_type.as_str() {
                        "WAIT" => true,
                        "SCROLL" => !last_act.was_repeated, // Scroll up to 2 times
                        "CLICK_AND_TYPE" => false, // Never reuse click on identical screen to allow coordinate calibration
                        _ => false,
                    };

                    if reuse_allowed {
                        let status = if ctx.langue == "Français" {
                            format!("Fast : Répétition de l'action {}", last_act.action_type)
                        } else {
                            format!("Fast: Repeating action {}", last_act.action_type)
                        };

                        let report = if ctx.langue == "Français" {
                            format!("Écran identique détecté. Réutilisation de la stratégie précédente ({}) pour éviter la latence LLM.", last_act.action_type)
                        } else {
                            format!("Identical screen detected. Reusing the previous strategy ({}) to avoid LLM latency.", last_act.action_type)
                        };

                        return DecisionRoute::Fast(Box::new(LlmResponse {
                            status_display: status,
                            action: last_act.action_type.clone(),
                            relative_click_position: last_act.coordinates.map(|(x, y)| vec![x, y]).unwrap_or_else(|| vec![0.0, 0.0]),
                            text_to_type: last_act.text_typed.clone().unwrap_or_default(),
                            scroll_value: if last_act.action_type == "SCROLL" { -6 } else { 0 },
                            wait_seconds: 5,
                            report: Some(report),
                            ..Default::default()
                        }));
                    }
                }
            }
        }

        DecisionRoute::Slow
    }

    /// Helper to detect if two screenshots are completely identical.
    fn is_screen_identical(&self, img1: &DynamicImage, img2: &DynamicImage) -> bool {
        let target_size = 128u32;
        let small1 = img1.resize_exact(target_size, target_size, image::imageops::FilterType::Nearest);
        let small2 = img2.resize_exact(target_size, target_size, image::imageops::FilterType::Nearest);

        let gray1 = small1.to_luma8();
        let gray2 = small2.to_luma8();

        let total = (target_size * target_size) as f32;
        let mut diff_count = 0u32;

        for (p1, p2) in gray1.pixels().zip(gray2.pixels()) {
            let val1 = p1.0[0] as i16;
            let val2 = p2.0[0] as i16;
            if (val1 - val2).abs() > 10 {
                diff_count += 1;
            }
        }
        let diff_ratio = diff_count as f32 / total;
        diff_ratio < 0.0005 // extremely identical
    }

    /// Helper to detect local animations (spinners) on a mostly static screen.
    fn detect_spinner(&self, img1: &DynamicImage, img2: &DynamicImage) -> bool {
        let target_size = 128u32;
        let small1 = img1.resize_exact(target_size, target_size, image::imageops::FilterType::Nearest);
        let small2 = img2.resize_exact(target_size, target_size, image::imageops::FilterType::Nearest);

        let gray1 = small1.to_luma8();
        let gray2 = small2.to_luma8();

        let mut changed_pixels = 0usize;
        let mut min_x = target_size;
        let mut max_x = 0u32;
        let mut min_y = target_size;
        let mut max_y = 0u32;

        for y in 0..target_size {
            for x in 0..target_size {
                let val1 = gray1.get_pixel(x, y).0[0] as i16;
                let val2 = gray2.get_pixel(x, y).0[0] as i16;
                if (val1 - val2).abs() > 25 {
                    changed_pixels += 1;
                    if x < min_x { min_x = x; }
                    if x > max_x { max_x = x; }
                    if y < min_y { min_y = y; }
                    if y > max_y { max_y = y; }
                }
            }
        }

        let total = (target_size * target_size) as f32;
        let diff_ratio = changed_pixels as f32 / total;

        // Spinner is a small local animation: overall change is very small, but localized in a small bounding box
        if (0.0001..=0.015).contains(&diff_ratio) {
            let span_x = max_x.saturating_sub(min_x);
            let span_y = max_y.saturating_sub(min_y);
            if span_x < 25 && span_y < 25 && span_x > 0 && span_y > 0 {
                return true;
            }
        }
        false
    }

    /// Helper to detect active modal dialogue coordinates.
    fn detect_modal_dialog(&self, ctx: &DecisionContext) -> Option<(f64, f64)> {
        let fg_title = self.capturer.get_foreground_window_title()?;
        let fg_lower = fg_title.to_lowercase();

        // Keywords indicating a dialog or confirmation window
        let keywords = [
            "dialog", "confirm", "alerte", "alert", "error", "attention", "popup",
            "message", "warning", "question", "erreur", "information", "modal",
            "confirmer", "authentification", "login"
        ];
        let is_dialog_title = keywords.iter().any(|k| fg_lower.contains(k));

        if let Some(target) = ctx.target_window_title {
            if crate::screen_capture::is_window_title_match(&fg_title, target) {
                return None;
            }
        }

        let windows = self.capturer.get_windows();
        let modal_win = windows.iter().find(|w| {
            crate::screen_capture::is_window_title_match(&w.title, &fg_title)
        })?;

        // Dialog boxes are small popup windows
        if (modal_win.bbox.width > 900 || modal_win.bbox.height > 600 || modal_win.bbox.width < 100 || modal_win.bbox.height < 80)
            && !is_dialog_title
        {
            return None;
        }

        // Try text detector first for precise button location
        let target_words = ["ok", "yes", "oui", "valider", "confirm", "accepter", "ok"];
        let button_pos = if let Some(ref detector) = self.text_detector {
            detector.find_text_center(ctx.screenshot, &target_words)
        } else {
            None
        };

        let (button_abs_x, button_abs_y) = if let Some((tx, ty)) = button_pos {
            // Convert relative text position to absolute coordinates
            let bounds = if let Some(target_title) = ctx.target_window_title {
                windows.iter().find(|w| crate::screen_capture::is_window_title_match(&w.title, target_title))
                    .map(|w| w.bbox.clone())
            } else {
                let monitors = self.capturer.get_monitors();
                if monitors.is_empty() {
                    Some(crate::screen_capture::BoundingBox::new(0, 0, 1920, 1080))
                } else {
                    let min_x = monitors.iter().map(|m| m.bbox.left).min();
                    let min_y = monitors.iter().map(|m| m.bbox.top).min();
                    let max_x = monitors.iter().map(|m| m.bbox.left + m.bbox.width).max();
                    let max_y = monitors.iter().map(|m| m.bbox.top + m.bbox.height).max();
                    match (min_x, min_y, max_x, max_y) {
                        (Some(min_x), Some(min_y), Some(max_x), Some(max_y)) => {
                            Some(crate::screen_capture::BoundingBox::new(min_x, min_y, max_x - min_x, max_y - min_y))
                        }
                        _ => None,
                    }
                }
            };
            let bounds = match bounds {
                Some(b) => b,
                None => {
                    let monitors = self.capturer.get_monitors();
                    if monitors.is_empty() {
                        crate::screen_capture::BoundingBox::new(0, 0, 1920, 1080)
                    } else {
                        let min_x = monitors.iter().map(|m| m.bbox.left).min().unwrap_or(0);
                        let min_y = monitors.iter().map(|m| m.bbox.top).min().unwrap_or(0);
                        let max_x = monitors.iter().map(|m| m.bbox.left + m.bbox.width).max().unwrap_or(0);
                        let max_y = monitors.iter().map(|m| m.bbox.top + m.bbox.height).max().unwrap_or(0);
                        crate::screen_capture::BoundingBox::new(min_x, min_y, max_x - min_x, max_y - min_y)
                    }
                }
            };
            (bounds.left + tx as i32, bounds.top + ty as i32)
        } else {
            // Fallback: estimate button location in dialog: x=55%, y=75%
            (
                modal_win.bbox.left + (modal_win.bbox.width as f64 * 0.55) as i32,
                modal_win.bbox.top + (modal_win.bbox.height as f64 * 0.75) as i32,
            )
        };

        // Convert absolute to screenshot relative coordinates
        let bounds = if let Some(target_title) = ctx.target_window_title {
            windows.iter().find(|w| crate::screen_capture::is_window_title_match(&w.title, target_title))
                .map(|w| w.bbox.clone())
        } else {
            let monitors = self.capturer.get_monitors();
            if monitors.is_empty() {
                Some(crate::screen_capture::BoundingBox::new(0, 0, 1920, 1080))
            } else {
                let min_x = monitors.iter().map(|m| m.bbox.left).min();
                let min_y = monitors.iter().map(|m| m.bbox.top).min();
                let max_x = monitors.iter().map(|m| m.bbox.left + m.bbox.width).max();
                let max_y = monitors.iter().map(|m| m.bbox.top + m.bbox.height).max();
                match (min_x, min_y, max_x, max_y) {
                    (Some(min_x), Some(min_y), Some(max_x), Some(max_y)) => {
                        Some(crate::screen_capture::BoundingBox::new(min_x, min_y, max_x - min_x, max_y - min_y))
                    }
                    _ => None,
                }
            }
        };
        let bounds = match bounds {
            Some(b) => b,
            None => {
                // Fallback: if target window not found, use monitors
                let monitors = self.capturer.get_monitors();
                if monitors.is_empty() {
                    crate::screen_capture::BoundingBox::new(0, 0, 1920, 1080)
                } else {
                    let min_x = monitors.iter().map(|m| m.bbox.left).min().unwrap_or(0);
                    let min_y = monitors.iter().map(|m| m.bbox.top).min().unwrap_or(0);
                    let max_x = monitors.iter().map(|m| m.bbox.left + m.bbox.width).max().unwrap_or(0);
                    let max_y = monitors.iter().map(|m| m.bbox.top + m.bbox.height).max().unwrap_or(0);
                    crate::screen_capture::BoundingBox::new(min_x, min_y, max_x - min_x, max_y - min_y)
                }
            }
        };

        if bounds.width <= 0 || bounds.height <= 0 {
            return None;
        }

        let rel_x = (button_abs_x - bounds.left) as f64 / bounds.width as f64;
        let rel_y = (button_abs_y - bounds.top) as f64 / bounds.height as f64;

        Some((rel_x.clamp(0.0, 1.0), rel_y.clamp(0.0, 1.0)))
    }
}

#[cfg(test)]
#[path = "decision_router_tests.rs"]
mod tests;

