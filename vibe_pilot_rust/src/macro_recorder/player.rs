//! Macro playback player engine.
//!
//! Executes recorded or edited macro sequences sequentially with
//! adjustable iterations, cancellation support, global delay pacing overrides,
//! start countdown timers, multi-monitor coordinate preservation, key hold duration simulation,
//! multi-key shortcuts, drag & drop, and IoC abstractions.

use super::{ActionKind, MacroSequence};
use crate::peripheral_controller::PeripheralInput;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::Duration;

pub trait MacroPlayerTrait: Send + Sync {
    fn play_async(
        &self,
        sequence: MacroSequence,
        iterations: u32,
        start_delay_sec: u32,
        controller: Arc<dyn PeripheralInput>,
        cancel_flag: Arc<AtomicBool>,
    ) -> tokio::task::JoinHandle<Result<(), String>>;
}

pub struct MacroPlayer;

fn parse_key_str(key: &str) -> rdev::Key {
    use rdev::Key;
    match key {
        "Alt" | "AltLeft" => Key::Alt,
        "AltRight" | "AltGr" => Key::AltGr,
        "Ctrl" | "ControlLeft" => Key::ControlLeft,
        "ControlRight" => Key::ControlRight,
        "Shift" | "ShiftLeft" => Key::ShiftLeft,
        "ShiftRight" => Key::ShiftRight,
        "Win" | "MetaLeft" => Key::MetaLeft,
        "MetaRight" => Key::MetaRight,
        "Space" => Key::Space,
        "Return" | "Enter" => Key::Return,
        "Tab" => Key::Tab,
        "Back" | "Backspace" => Key::Backspace,
        "Escape" | "Esc" => Key::Escape,
        "Up" | "UpArrow" => Key::UpArrow,
        "Down" | "DownArrow" => Key::DownArrow,
        "Left" | "LeftArrow" => Key::LeftArrow,
        "Right" | "RightArrow" => Key::RightArrow,
        "Delete" => Key::Delete,
        "Home" => Key::Home,
        "End" => Key::End,
        "PageUp" => Key::PageUp,
        "PageDown" => Key::PageDown,
        "A" | "a" | "KeyA" => Key::KeyA,
        "B" | "b" | "KeyB" => Key::KeyB,
        "C" | "c" | "KeyC" => Key::KeyC,
        "D" | "d" | "KeyD" => Key::KeyD,
        "E" | "e" | "KeyE" => Key::KeyE,
        "F" | "f" | "KeyF" => Key::KeyF,
        "G" | "g" | "KeyG" => Key::KeyG,
        "H" | "h" | "KeyH" => Key::KeyH,
        "I" | "i" | "KeyI" => Key::KeyI,
        "J" | "j" | "KeyJ" => Key::KeyJ,
        "K" | "k" | "KeyK" => Key::KeyK,
        "L" | "l" | "KeyL" => Key::KeyL,
        "M" | "m" | "KeyM" => Key::KeyM,
        "N" | "n" | "KeyN" => Key::KeyN,
        "O" | "o" | "KeyO" => Key::KeyO,
        "P" | "p" | "KeyP" => Key::KeyP,
        "Q" | "q" | "KeyQ" => Key::KeyQ,
        "R" | "r" | "KeyR" => Key::KeyR,
        "S" | "s" | "KeyS" => Key::KeyS,
        "T" | "t" | "KeyT" => Key::KeyT,
        "U" | "u" | "KeyU" => Key::KeyU,
        "V" | "v" | "KeyV" => Key::KeyV,
        "W" | "w" | "KeyW" => Key::KeyW,
        "X" | "x" | "KeyX" => Key::KeyX,
        "Y" | "y" | "KeyY" => Key::KeyY,
        "Z" | "z" | "KeyZ" => Key::KeyZ,
        "0" | "Num0" => Key::Num0,
        "1" | "Num1" => Key::Num1,
        "2" | "Num2" => Key::Num2,
        "3" | "Num3" => Key::Num3,
        "4" | "Num4" => Key::Num4,
        "5" | "Num5" => Key::Num5,
        "6" | "Num6" => Key::Num6,
        "7" | "Num7" => Key::Num7,
        "8" | "Num8" => Key::Num8,
        "9" | "Num9" => Key::Num9,
        _ => Key::Unknown(0),
    }
}

impl MacroPlayer {
    /// Replays the given macro sequence for `iterations` count with optional initial countdown delay.
    pub async fn play(
        sequence: MacroSequence,
        iterations: u32,
        start_delay_sec: u32,
        controller: Arc<dyn PeripheralInput>,
        cancel_flag: Arc<AtomicBool>,
        on_countdown: Option<Box<dyn Fn(u32) + Send + Sync>>,
        on_step: Option<Box<dyn Fn(u32, usize, &ActionKind) + Send + Sync>>,
    ) -> Result<(), String> {
        // --- 1. START COUNTDOWN TIMER ---
        if start_delay_sec > 0 {
            for remaining in (1..=start_delay_sec).rev() {
                if cancel_flag.load(Ordering::Relaxed) {
                    return Ok(());
                }
                if let Some(ref cb) = on_countdown {
                    cb(remaining);
                }
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        }

        let total_iterations = if iterations == 0 { 1 } else { iterations };

        for iter in 1..=total_iterations {
            if cancel_flag.load(Ordering::Relaxed) {
                return Ok(());
            }

            for (idx, step) in sequence.actions.iter().enumerate() {
                if cancel_flag.load(Ordering::Relaxed) {
                    return Ok(());
                }

                if !step.enabled {
                    continue;
                }

                // Respect delay between actions or global delay override
                let effective_delay = match sequence.global_delay_override_ms {
                    Some(global_ms) => step.delay_ms.max(global_ms),
                    None => step.delay_ms,
                };

                if effective_delay > 0 {
                    tokio::time::sleep(Duration::from_millis(effective_delay)).await;
                }

                if cancel_flag.load(Ordering::Relaxed) {
                    return Ok(());
                }

                if let Some(ref cb) = on_step {
                    cb(iter, idx, &step.action);
                }

                match &step.action {
                    ActionKind::Click { x, y, button, double_click } => {
                        #[cfg(target_os = "windows")]
                        {
                            crate::peripheral_controller::win32_move_mouse_absolute(*x, *y);
                            std::thread::sleep(Duration::from_millis(20));
                            match button.as_str() {
                                "Right" => { let _ = controller.right_click(); }
                                "Middle" => { let _ = controller.middle_click(); }
                                _ => {
                                    if *double_click {
                                        let _ = controller.double_click();
                                    } else {
                                        crate::peripheral_controller::win32_click_current_position();
                                    }
                                }
                            }
                        }
                        #[cfg(not(target_os = "windows"))]
                        {
                            let _ = (x, y, button, double_click);
                        }
                    }
                    ActionKind::MouseDown { x, y, button: _ } => {
                        #[cfg(target_os = "windows")]
                        {
                            crate::peripheral_controller::win32_move_mouse_absolute(*x, *y);
                        }
                        #[cfg(not(target_os = "windows"))]
                        {
                            let _ = (x, y);
                        }
                    }
                    ActionKind::MouseUp { x, y, button: _ } => {
                        #[cfg(target_os = "windows")]
                        {
                            crate::peripheral_controller::win32_move_mouse_absolute(*x, *y);
                        }
                        #[cfg(not(target_os = "windows"))]
                        {
                            let _ = (x, y);
                        }
                    }
                    ActionKind::DragAndDrop { from_x, from_y, to_x, to_y, button } => {
                        let btn = match button.as_str() {
                            "Right" => rdev::Button::Right,
                            "Middle" => rdev::Button::Middle,
                            _ => rdev::Button::Left,
                        };
                        let _ = controller.drag_and_drop((*from_x, *from_y), (*to_x, *to_y), btn);
                    }
                    ActionKind::Move { x, y } => {
                        #[cfg(target_os = "windows")]
                        {
                            crate::peripheral_controller::win32_move_mouse_absolute(*x, *y);
                        }
                        #[cfg(not(target_os = "windows"))]
                        {
                            let _ = (x, y);
                        }
                    }
                    ActionKind::KeyPress { key } => {
                        use rdev::{simulate, EventType};
                        let key_enum = parse_key_str(key);
                        if key_enum != rdev::Key::Unknown(0) {
                            let _ = simulate(&EventType::KeyPress(key_enum));
                            tokio::time::sleep(Duration::from_millis(20)).await;
                            let _ = simulate(&EventType::KeyRelease(key_enum));
                        } else {
                            #[cfg(target_os = "windows")]
                            {
                                crate::peripheral_controller::native_win::win32_type_text(key);
                            }
                        }
                    }
                    ActionKind::KeyHold { key, hold_duration_ms } => {
                        use rdev::{simulate, EventType};
                        let key_enum = parse_key_str(key);
                        if key_enum != rdev::Key::Unknown(0) {
                            let _ = simulate(&EventType::KeyPress(key_enum));
                            tokio::time::sleep(Duration::from_millis(*hold_duration_ms)).await;
                            let _ = simulate(&EventType::KeyRelease(key_enum));
                        } else {
                            #[cfg(target_os = "windows")]
                            {
                                crate::peripheral_controller::native_win::win32_type_text(key);
                                tokio::time::sleep(Duration::from_millis(*hold_duration_ms)).await;
                            }
                        }
                    }
                    ActionKind::KeyCombo { keys } => {
                        use rdev::{simulate, EventType};
                        let mut parsed_keys = Vec::new();
                        for k in keys {
                            let key_enum = parse_key_str(k);
                            if key_enum != rdev::Key::Unknown(0) {
                                parsed_keys.push(key_enum);
                            }
                        }

                        for k in &parsed_keys {
                            let _ = simulate(&EventType::KeyPress(*k));
                        }
                        tokio::time::sleep(Duration::from_millis(50)).await;

                        for k in parsed_keys.iter().rev() {
                            let _ = simulate(&EventType::KeyRelease(*k));
                        }
                    }
                    ActionKind::Scroll { dx: _, dy } => {
                        let _ = controller.scroll_vertical(*dy);
                    }
                    ActionKind::Wait { ms } => {
                        tokio::time::sleep(Duration::from_millis(*ms)).await;
                    }
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::macro_recorder::RecordedAction;
    use crate::peripheral_controller::MockPeripheralInput;

    #[tokio::test]
    async fn test_macro_player_execution_with_key_hold() {
        let mut seq = MacroSequence::new("Test KeyHold");
        seq.actions.push(RecordedAction::new(
            1,
            5,
            ActionKind::KeyHold {
                key: "Space".to_string(),
                hold_duration_ms: 10,
            },
        ));

        let controller: Arc<dyn PeripheralInput> = Arc::new(MockPeripheralInput::new());
        let cancel = Arc::new(AtomicBool::new(false));

        let res = MacroPlayer::play(seq, 1, 0, controller, cancel, None, None).await;
        assert!(res.is_ok());
    }
}
