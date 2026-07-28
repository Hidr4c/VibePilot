//! Text Script Protocol Parser & Serializer for VibePilot Macro Sequences.
//!
//! Provides a human-readable and AI-generatable DSL for exporting, editing,
//! and importing macro action sequences via plain text.
//!
//! Multi-screen support: scripts can use relative coordinates (`rx=0.5 ry=0.3`)
//! in addition to absolute pixel coordinates (`x=500 y=300`). Relative coordinates
//! are resolved to absolute desktop pixels using the target window/monitor bounds
//! via `resolve_relative_coords()`.

use crate::macro_recorder::{ActionKind, RecordedAction, MacroSequence};
use crate::peripheral_controller::ScreenOffset;

/// Marker value for coordinates that need relative resolution.
/// Uses i32::MIN as an impossible pixel coordinate sentinel.
const RELATIVE_COORD_SENTINEL: i32 = i32::MIN;

/// Holds unresolved relative coordinate info for a single action.
#[derive(Debug, Clone)]
pub struct RelativeCoordEntry {
    pub action_id: u64,
    pub rx: Option<f64>,
    pub ry: Option<f64>,
    /// For DRAG: relative coords of the destination
    pub rx2: Option<f64>,
    pub ry2: Option<f64>,
}

pub struct MacroScriptParser;

impl MacroScriptParser {
    /// Serialize a `MacroSequence` into a human-readable and AI-friendly text script protocol.
    pub fn to_script(
        sequence: &MacroSequence,
        iterations: u32,
        start_delay: u32,
        record_delay: u32,
    ) -> String {
        let mut lines = Vec::new();
        lines.push("# VibePilot Macro Script Protocol".to_string());
        lines.push("# Format: COMMAND key=value ...".to_string());

        let global_delay = sequence.global_delay_override_ms.unwrap_or(0);
        lines.push(format!(
            "OPTIONS iterations={} start_delay={} record_delay={} global_delay_ms={}",
            iterations, start_delay, record_delay, global_delay
        ));
        lines.push(String::new());

        for action in &sequence.actions {
            if !action.enabled {
                lines.push(format!("# DISABLED {}", Self::action_to_line(action)));
            } else {
                lines.push(Self::action_to_line(action));
            }
        }

        lines.join("\n")
    }

    fn action_to_line(action: &RecordedAction) -> String {
        let delay = action.delay_ms;
        match &action.action {
            ActionKind::Click { x, y, button, double_click } => {
                format!("CLICK x={} y={} button={} double={} delay={}", x, y, button, double_click, delay)
            }
            ActionKind::MouseDown { x, y, button } => {
                format!("MOUSEDOWN x={} y={} button={} delay={}", x, y, button, delay)
            }
            ActionKind::MouseUp { x, y, button } => {
                format!("MOUSEUP x={} y={} button={} delay={}", x, y, button, delay)
            }
            ActionKind::Move { x, y } => {
                format!("MOVE x={} y={} delay={}", x, y, delay)
            }
            ActionKind::DragAndDrop { from_x, from_y, to_x, to_y, button } => {
                format!("DRAG from_x={} from_y={} to_x={} to_y={} button={} delay={}", from_x, from_y, to_x, to_y, button, delay)
            }
            ActionKind::KeyPress { key } => {
                format!("KEYPRESS key={} delay={}", key, delay)
            }
            ActionKind::KeyHold { key, hold_duration_ms } => {
                format!("KEYHOLD key={} duration={} delay={}", key, hold_duration_ms, delay)
            }
            ActionKind::KeyCombo { keys } => {
                format!("KEYCOMBO keys={} delay={}", keys.join(","), delay)
            }
            ActionKind::Scroll { dx, dy } => {
                format!("SCROLL dx={} dy={} delay={}", dx, dy, delay)
            }
            ActionKind::Wait { ms } => {
                format!("SLEEP ms={} delay={}", ms, delay)
            }
        }
    }

    /// Parse a script text block into a `MacroSequence`, extracted run options,
    /// and a list of unresolved relative coordinate entries for multi-screen resolution.
    pub fn from_script(
        script_text: &str,
    ) -> Result<(MacroSequence, Option<u32>, Option<u32>, Option<u32>, Vec<RelativeCoordEntry>), String> {
        let mut sequence = MacroSequence::new("Imported Run");
        let mut parsed_iterations: Option<u32> = None;
        let mut parsed_start_delay: Option<u32> = None;
        let mut parsed_record_delay: Option<u32> = None;
        let mut relative_entries: Vec<RelativeCoordEntry> = Vec::new();

        for (line_num, raw_line) in script_text.lines().enumerate() {
            let line = raw_line.trim();

            if line.is_empty() {
                continue;
            }

            let (enabled, active_line) = if line.starts_with("# DISABLED ") {
                (false, line.trim_start_matches("# DISABLED ").trim())
            } else if line.starts_with('#') {
                continue;
            } else {
                (true, line)
            };

            let parts: Vec<&str> = active_line.split_whitespace().collect();
            if parts.is_empty() {
                continue;
            }

            let command = parts[0].to_uppercase();
            let mut params = std::collections::HashMap::new();

            for part in &parts[1..] {
                if let Some((k, v)) = part.split_once('=') {
                    params.insert(k.to_lowercase(), v.to_string());
                }
            }

            match command.as_str() {
                "OPTIONS" | "OPTION" => {
                    if let Some(val) = params.get("iterations") {
                        if let Ok(n) = val.parse::<u32>() {
                            parsed_iterations = Some(n);
                        }
                    }
                    if let Some(val) = params.get("start_delay").or_else(|| params.get("start_delay_sec")) {
                        if let Ok(n) = val.parse::<u32>() {
                            parsed_start_delay = Some(n);
                        }
                    }
                    if let Some(val) = params.get("record_delay").or_else(|| params.get("record_delay_sec")) {
                        if let Ok(n) = val.parse::<u32>() {
                            parsed_record_delay = Some(n);
                        }
                    }
                    if let Some(val) = params.get("global_delay_ms") {
                        if let Ok(n) = val.parse::<u64>() {
                            sequence.global_delay_override_ms = if n > 0 { Some(n) } else { None };
                        }
                    }
                }
                "CLICK" => {
                    let rx = params.get("rx").and_then(|v| v.parse::<f64>().ok());
                    let ry = params.get("ry").and_then(|v| v.parse::<f64>().ok());
                    let x = if rx.is_some() { RELATIVE_COORD_SENTINEL } else { params.get("x").and_then(|v| v.parse::<i32>().ok()).unwrap_or(0) };
                    let y = if ry.is_some() { RELATIVE_COORD_SENTINEL } else { params.get("y").and_then(|v| v.parse::<i32>().ok()).unwrap_or(0) };
                    let button = params.get("button").cloned().unwrap_or_else(|| "Left".to_string());
                    let double_click = params.get("double").map(|v| v == "true").unwrap_or(false);
                    let delay = params.get("delay").and_then(|v| v.parse::<u64>().ok()).unwrap_or(200);

                    let act_id = sequence.add_action(delay, ActionKind::Click { x, y, button, double_click });
                    if let Some(act) = sequence.actions.iter_mut().find(|a| a.id == act_id) {
                        act.enabled = enabled;
                    }
                    if rx.is_some() || ry.is_some() {
                        relative_entries.push(RelativeCoordEntry { action_id: act_id, rx, ry, rx2: None, ry2: None });
                    }
                }
                "MOUSEDOWN" => {
                    let rx = params.get("rx").and_then(|v| v.parse::<f64>().ok());
                    let ry = params.get("ry").and_then(|v| v.parse::<f64>().ok());
                    let x = if rx.is_some() { RELATIVE_COORD_SENTINEL } else { params.get("x").and_then(|v| v.parse::<i32>().ok()).unwrap_or(0) };
                    let y = if ry.is_some() { RELATIVE_COORD_SENTINEL } else { params.get("y").and_then(|v| v.parse::<i32>().ok()).unwrap_or(0) };
                    let button = params.get("button").cloned().unwrap_or_else(|| "Left".to_string());
                    let delay = params.get("delay").and_then(|v| v.parse::<u64>().ok()).unwrap_or(200);

                    let act_id = sequence.add_action(delay, ActionKind::MouseDown { x, y, button });
                    if let Some(act) = sequence.actions.iter_mut().find(|a| a.id == act_id) {
                        act.enabled = enabled;
                    }
                    if rx.is_some() || ry.is_some() {
                        relative_entries.push(RelativeCoordEntry { action_id: act_id, rx, ry, rx2: None, ry2: None });
                    }
                }
                "MOUSEUP" => {
                    let rx = params.get("rx").and_then(|v| v.parse::<f64>().ok());
                    let ry = params.get("ry").and_then(|v| v.parse::<f64>().ok());
                    let x = if rx.is_some() { RELATIVE_COORD_SENTINEL } else { params.get("x").and_then(|v| v.parse::<i32>().ok()).unwrap_or(0) };
                    let y = if ry.is_some() { RELATIVE_COORD_SENTINEL } else { params.get("y").and_then(|v| v.parse::<i32>().ok()).unwrap_or(0) };
                    let button = params.get("button").cloned().unwrap_or_else(|| "Left".to_string());
                    let delay = params.get("delay").and_then(|v| v.parse::<u64>().ok()).unwrap_or(200);

                    let act_id = sequence.add_action(delay, ActionKind::MouseUp { x, y, button });
                    if let Some(act) = sequence.actions.iter_mut().find(|a| a.id == act_id) {
                        act.enabled = enabled;
                    }
                    if rx.is_some() || ry.is_some() {
                        relative_entries.push(RelativeCoordEntry { action_id: act_id, rx, ry, rx2: None, ry2: None });
                    }
                }
                "MOVE" => {
                    let rx = params.get("rx").and_then(|v| v.parse::<f64>().ok());
                    let ry = params.get("ry").and_then(|v| v.parse::<f64>().ok());
                    let x = if rx.is_some() { RELATIVE_COORD_SENTINEL } else { params.get("x").and_then(|v| v.parse::<i32>().ok()).unwrap_or(0) };
                    let y = if ry.is_some() { RELATIVE_COORD_SENTINEL } else { params.get("y").and_then(|v| v.parse::<i32>().ok()).unwrap_or(0) };
                    let delay = params.get("delay").and_then(|v| v.parse::<u64>().ok()).unwrap_or(100);

                    let act_id = sequence.add_action(delay, ActionKind::Move { x, y });
                    if let Some(act) = sequence.actions.iter_mut().find(|a| a.id == act_id) {
                        act.enabled = enabled;
                    }
                    if rx.is_some() || ry.is_some() {
                        relative_entries.push(RelativeCoordEntry { action_id: act_id, rx, ry, rx2: None, ry2: None });
                    }
                }
                "DRAG" | "DRAGANDDROP" => {
                    let from_rx = params.get("from_rx").and_then(|v| v.parse::<f64>().ok());
                    let from_ry = params.get("from_ry").and_then(|v| v.parse::<f64>().ok());
                    let to_rx = params.get("to_rx").and_then(|v| v.parse::<f64>().ok());
                    let to_ry = params.get("to_ry").and_then(|v| v.parse::<f64>().ok());
                    let from_x = if from_rx.is_some() { RELATIVE_COORD_SENTINEL } else { params.get("from_x").and_then(|v| v.parse::<i32>().ok()).unwrap_or(0) };
                    let from_y = if from_ry.is_some() { RELATIVE_COORD_SENTINEL } else { params.get("from_y").and_then(|v| v.parse::<i32>().ok()).unwrap_or(0) };
                    let to_x = if to_rx.is_some() { RELATIVE_COORD_SENTINEL } else { params.get("to_x").and_then(|v| v.parse::<i32>().ok()).unwrap_or(0) };
                    let to_y = if to_ry.is_some() { RELATIVE_COORD_SENTINEL } else { params.get("to_y").and_then(|v| v.parse::<i32>().ok()).unwrap_or(0) };
                    let button = params.get("button").cloned().unwrap_or_else(|| "Left".to_string());
                    let delay = params.get("delay").and_then(|v| v.parse::<u64>().ok()).unwrap_or(200);

                    let act_id = sequence.add_action(delay, ActionKind::DragAndDrop { from_x, from_y, to_x, to_y, button });
                    if let Some(act) = sequence.actions.iter_mut().find(|a| a.id == act_id) {
                        act.enabled = enabled;
                    }
                    if from_rx.is_some() || from_ry.is_some() || to_rx.is_some() || to_ry.is_some() {
                        relative_entries.push(RelativeCoordEntry { action_id: act_id, rx: from_rx, ry: from_ry, rx2: to_rx, ry2: to_ry });
                    }
                }
                "KEYPRESS" | "PRESS" | "KEY" => {
                    let key = params.get("key").cloned().unwrap_or_else(|| "A".to_string());
                    let delay = params.get("delay").and_then(|v| v.parse::<u64>().ok()).unwrap_or(150);

                    let act_id = sequence.add_action(delay, ActionKind::KeyPress { key });
                    if let Some(act) = sequence.actions.iter_mut().find(|a| a.id == act_id) {
                        act.enabled = enabled;
                    }
                }
                "KEYHOLD" | "HOLD" => {
                    let key = params.get("key").cloned().unwrap_or_else(|| "A".to_string());
                    let hold_duration_ms = params.get("duration").and_then(|v| v.parse::<u64>().ok()).unwrap_or(500);
                    let delay = params.get("delay").and_then(|v| v.parse::<u64>().ok()).unwrap_or(150);

                    let act_id = sequence.add_action(delay, ActionKind::KeyHold { key, hold_duration_ms });
                    if let Some(act) = sequence.actions.iter_mut().find(|a| a.id == act_id) {
                        act.enabled = enabled;
                    }
                }
                "KEYCOMBO" | "COMBO" => {
                    let keys_str = params.get("keys").cloned().unwrap_or_else(|| "Ctrl,C".to_string());
                    let keys: Vec<String> = if keys_str.contains(',') {
                        keys_str.split(',').map(|s| s.trim().to_string()).collect()
                    } else {
                        keys_str.split('+').map(|s| s.trim().to_string()).collect()
                    };
                    let delay = params.get("delay").and_then(|v| v.parse::<u64>().ok()).unwrap_or(150);

                    let act_id = sequence.add_action(delay, ActionKind::KeyCombo { keys });
                    if let Some(act) = sequence.actions.iter_mut().find(|a| a.id == act_id) {
                        act.enabled = enabled;
                    }
                }
                "SCROLL" => {
                    let dx = params.get("dx").and_then(|v| v.parse::<i32>().ok()).unwrap_or(0);
                    let dy = params.get("dy").and_then(|v| v.parse::<i32>().ok()).unwrap_or(0);
                    let delay = params.get("delay").and_then(|v| v.parse::<u64>().ok()).unwrap_or(150);

                    let act_id = sequence.add_action(delay, ActionKind::Scroll { dx, dy });
                    if let Some(act) = sequence.actions.iter_mut().find(|a| a.id == act_id) {
                        act.enabled = enabled;
                    }
                }
                "SLEEP" | "WAIT" | "PAUSE" => {
                    let ms = if let Some(sec_val) = params.get("sec") {
                        sec_val.parse::<u64>().map(|s| s * 1000).unwrap_or(500)
                    } else {
                        params.get("ms").and_then(|v| v.parse::<u64>().ok()).unwrap_or(500)
                    };
                    let delay = params.get("delay").and_then(|v| v.parse::<u64>().ok()).unwrap_or(0);

                    let act_id = sequence.add_action(delay, ActionKind::Wait { ms });
                    if let Some(act) = sequence.actions.iter_mut().find(|a| a.id == act_id) {
                        act.enabled = enabled;
                    }
                }
                other => {
                    return Err(format!("Ligne {}: Commande de protocole inconnue '{}'", line_num + 1, other));
                }
            }
        }

        Ok((sequence, parsed_iterations, parsed_start_delay, parsed_record_delay, relative_entries))
    }

    /// Resolve relative coordinates in a parsed `MacroSequence` using the provided screen offsets.
    ///
    /// For each `RelativeCoordEntry`, converts `rx`/`ry` (float 0.0-1.0) to absolute pixel
    /// coordinates based on the target window/monitor bounds from `offsets`.
    /// This ensures correct positioning on multi-monitor setups where monitors can have
    /// negative coordinates (e.g., monitor to the left of the primary).
    ///
    /// If `offsets` is empty, falls back to a 1920x1080 default screen at (0,0).
    pub fn resolve_relative_coords(
        sequence: &mut MacroSequence,
        relative_entries: &[RelativeCoordEntry],
        offsets: &[ScreenOffset],
    ) {
        if relative_entries.is_empty() {
            return;
        }

        // Determine target bounds from offsets
        let (target_left, target_top, target_w, target_h) = if !offsets.is_empty() {
            let target = &offsets[0];
            (target.left, target.top, target.width, target.height)
        } else {
            // Fallback: assume primary screen at origin
            (0, 0, 1920, 1080)
        };

        let resolve = |rx: f64, ry: f64| -> (i32, i32) {
            let abs_x = target_left + (target_w as f64 * rx.clamp(0.0, 1.0)) as i32;
            let abs_y = target_top + (target_h as f64 * ry.clamp(0.0, 1.0)) as i32;
            (abs_x, abs_y)
        };

        for entry in relative_entries {
            if let Some(act) = sequence.actions.iter_mut().find(|a| a.id == entry.action_id) {
                match &mut act.action {
                    ActionKind::Click { x, y, .. }
                    | ActionKind::MouseDown { x, y, .. }
                    | ActionKind::MouseUp { x, y, .. }
                    | ActionKind::Move { x, y } => {
                        let rx_val = entry.rx.unwrap_or(0.5);
                        let ry_val = entry.ry.unwrap_or(0.5);
                        let (ax, ay) = resolve(rx_val, ry_val);
                        *x = ax;
                        *y = ay;
                    }
                    ActionKind::DragAndDrop { from_x, from_y, to_x, to_y, .. } => {
                        if entry.rx.is_some() || entry.ry.is_some() {
                            let rx_val = entry.rx.unwrap_or(0.5);
                            let ry_val = entry.ry.unwrap_or(0.5);
                            let (ax, ay) = resolve(rx_val, ry_val);
                            *from_x = ax;
                            *from_y = ay;
                        }
                        if entry.rx2.is_some() || entry.ry2.is_some() {
                            let rx2_val = entry.rx2.unwrap_or(0.5);
                            let ry2_val = entry.ry2.unwrap_or(0.5);
                            let (ax2, ay2) = resolve(rx2_val, ry2_val);
                            *to_x = ax2;
                            *to_y = ay2;
                        }
                    }
                    _ => {} // non-coordinate actions, ignore
                }
                // Update the label to reflect resolved coordinates
                act.label = act.action.description();
            }
        }
    }

    /// Get a `ScreenOffset` representing the monitor that currently has the focused
    /// (foreground) window. Used by the macro feature to resolve relative coordinates
    /// relative to the active screen on multi-monitor setups.
    ///
    /// On non-Windows platforms, falls back to a default 1920x1080 screen at (0,0).
    pub fn get_focused_screen_offset() -> ScreenOffset {
        #[cfg(target_os = "windows")]
        {
            let (left, top, width, height) = crate::peripheral_controller::win32_get_focused_screen_bounds();
            ScreenOffset {
                titre: "Focused Screen".to_string(),
                left,
                top,
                width,
                height,
                y_offset: top,
            }
        }
        #[cfg(not(target_os = "windows"))]
        {
            ScreenOffset {
                titre: "Default Screen".to_string(),
                left: 0,
                top: 0,
                width: 1920,
                height: 1080,
                y_offset: 0,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_script_parser_roundtrip() {
        let mut seq = MacroSequence::new("Test Script");
        seq.add_action(200, ActionKind::Click { x: 100, y: 200, button: "Left".to_string(), double_click: false });
        seq.add_action(150, ActionKind::KeyPress { key: "A".to_string() });
        seq.add_action(150, ActionKind::KeyCombo { keys: vec!["Ctrl".to_string(), "C".to_string()] });
        seq.add_action(0, ActionKind::Wait { ms: 1000 });

        let script = MacroScriptParser::to_script(&seq, 3, 2, 1);
        assert!(script.contains("CLICK x=100 y=200"));
        assert!(script.contains("KEYPRESS key=A"));
        assert!(script.contains("KEYCOMBO keys=Ctrl,C"));
        assert!(script.contains("SLEEP ms=1000"));
        assert!(script.contains("OPTIONS iterations=3 start_delay=2 record_delay=1"));

        let (parsed_seq, iters, start_del, rec_del, rel_entries) = MacroScriptParser::from_script(&script).expect("Parsing script failed");
        assert_eq!(iters, Some(3));
        assert_eq!(start_del, Some(2));
        assert_eq!(rec_del, Some(1));
        assert_eq!(parsed_seq.actions.len(), 4);
        assert!(rel_entries.is_empty(), "Absolute coords should not produce relative entries");
    }

    #[test]
    fn test_relative_coords_resolution() {
        let script = "CLICK rx=0.5 ry=0.25 button=Left\nMOVE rx=0.0 ry=1.0";
        let (mut seq, _, _, _, rel_entries) = MacroScriptParser::from_script(script).expect("parse failed");
        assert_eq!(rel_entries.len(), 2);

        // Simulate a target window on second monitor at left=-1920, top=0, 1920x1080
        let offsets = vec![ScreenOffset {
            titre: "TestWindow".to_string(),
            left: -1920,
            top: 0,
            width: 1920,
            height: 1080,
            y_offset: 0,
        }];

        MacroScriptParser::resolve_relative_coords(&mut seq, &rel_entries, &offsets);

        // CLICK rx=0.5 ry=0.25 -> x = -1920 + 1920*0.5 = -960, y = 0 + 1080*0.25 = 270
        match &seq.actions[0].action {
            ActionKind::Click { x, y, .. } => {
                assert_eq!(*x, -960);
                assert_eq!(*y, 270);
            }
            _ => panic!("Expected Click action"),
        }

        // MOVE rx=0.0 ry=1.0 -> x = -1920, y = 1080
        match &seq.actions[1].action {
            ActionKind::Move { x, y } => {
                assert_eq!(*x, -1920);
                assert_eq!(*y, 1080);
            }
            _ => panic!("Expected Move action"),
        }
    }
}
