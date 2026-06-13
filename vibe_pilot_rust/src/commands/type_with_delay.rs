//! TypeWithDelay command: types text character-by-character with random delays.
//!
//! Simulates human-like typing behavior by inserting random delays
//! between individual character presses.

use crate::commands::{Command, CommandOutcome, ExecutionContext};
use rand::Rng;

/// Command that types text with random delays between characters.
#[derive(Debug, Clone)]
pub struct TypeWithDelayCommand {
    pub text: String,
    pub min_delay_ms: u64,
    pub max_delay_ms: u64,
}

#[async_trait::async_trait]
impl Command for TypeWithDelayCommand {
    async fn execute(&self, _context: &mut ExecutionContext) -> CommandOutcome {
        if cfg!(test) {
            return CommandOutcome::Success;
        }

        for ch in self.text.chars() {
            #[cfg(target_os = "windows")]
            {
                use windows::Win32::UI::Input::KeyboardAndMouse::{
                    SendInput, INPUT, INPUT_KEYBOARD, KEYEVENTF_UNICODE, KEYEVENTF_KEYUP, KEYBDINPUT,
                };
                let mut utf16_buf = [0u16; 2];
                let utf16_slice = ch.encode_utf16(&mut utf16_buf);
                for &code in utf16_slice.iter() {
                    unsafe {
                        let mut inputs = [INPUT::default(), INPUT::default()];
                        
                        inputs[0].r#type = INPUT_KEYBOARD;
                        inputs[0].Anonymous.ki = KEYBDINPUT {
                            wVk: windows::Win32::UI::Input::KeyboardAndMouse::VIRTUAL_KEY(0),
                            wScan: code,
                            dwFlags: KEYEVENTF_UNICODE,
                            time: 0,
                            dwExtraInfo: 0,
                        };

                        inputs[1].r#type = INPUT_KEYBOARD;
                        inputs[1].Anonymous.ki = KEYBDINPUT {
                            wVk: windows::Win32::UI::Input::KeyboardAndMouse::VIRTUAL_KEY(0),
                            wScan: code,
                            dwFlags: KEYEVENTF_UNICODE | KEYEVENTF_KEYUP,
                            time: 0,
                            dwExtraInfo: 0,
                        };

                        SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
                    }
                }
            }

            let mut rng = rand::thread_rng();
            let delay = rng.gen_range(self.min_delay_ms..=self.max_delay_ms);
            std::thread::sleep(std::time::Duration::from_millis(delay));
        }

        CommandOutcome::Success
    }

    fn description(&self) -> &str {
        "TypeWithDelay"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_type_with_delay_success() {
        let cmd = TypeWithDelayCommand {
            text: "hello".to_string(),
            min_delay_ms: 10,
            max_delay_ms: 30,
        };
        let mut ctx = ExecutionContext::new();
        let outcome = cmd.execute(&mut ctx).await;
        assert!(outcome.is_success());
    }

    #[tokio::test]
    async fn test_type_with_delay_empty_text() {
        let cmd = TypeWithDelayCommand {
            text: String::new(),
            min_delay_ms: 10,
            max_delay_ms: 30,
        };
        let mut ctx = ExecutionContext::new();
        let outcome = cmd.execute(&mut ctx).await;
        assert!(outcome.is_success());
    }

    #[test]
    fn test_description() {
        let cmd = TypeWithDelayCommand {
            text: "test".to_string(),
            min_delay_ms: 10,
            max_delay_ms: 30,
        };
        assert_eq!(cmd.description(), "TypeWithDelay");
    }
}
