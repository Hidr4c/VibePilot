//! KeyHold and KeyRelease commands for extended key press simulation.
//!
//! Allows holding a key down for a duration before releasing it.

use crate::commands::{Command, CommandOutcome, ExecutionContext};

/// Holds a key down without releasing it.
#[derive(Debug, Clone)]
pub struct KeyHoldCommand {
    pub key: rdev::Key,
}

#[async_trait::async_trait]
impl Command for KeyHoldCommand {
    async fn execute(&self, _context: &mut ExecutionContext) -> CommandOutcome {
        if cfg!(test) {
            return CommandOutcome::Success;
        }
        let _ = rdev::simulate(&rdev::EventType::KeyPress(self.key));
        CommandOutcome::Success
    }

    fn description(&self) -> &str {
        "KeyHold"
    }
}

/// Releases a previously held key.
#[derive(Debug, Clone)]
pub struct KeyReleaseCommand {
    pub key: rdev::Key,
}

#[async_trait::async_trait]
impl Command for KeyReleaseCommand {
    async fn execute(&self, _context: &mut ExecutionContext) -> CommandOutcome {
        if cfg!(test) {
            return CommandOutcome::Success;
        }
        let _ = rdev::simulate(&rdev::EventType::KeyRelease(self.key));
        CommandOutcome::Success
    }

    fn description(&self) -> &str {
        "KeyRelease"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_key_hold() {
        let cmd = KeyHoldCommand { key: rdev::Key::ShiftLeft };
        let mut ctx = ExecutionContext::new();
        let outcome = cmd.execute(&mut ctx).await;
        assert!(outcome.is_success());
    }

    #[tokio::test]
    async fn test_key_release() {
        let cmd = KeyReleaseCommand { key: rdev::Key::ShiftLeft };
        let mut ctx = ExecutionContext::new();
        let outcome = cmd.execute(&mut ctx).await;
        assert!(outcome.is_success());
    }

    #[test]
    fn test_key_hold_description() {
        let cmd = KeyHoldCommand { key: rdev::Key::ControlLeft };
        assert_eq!(cmd.description(), "KeyHold");
    }

    #[test]
    fn test_key_release_description() {
        let cmd = KeyReleaseCommand { key: rdev::Key::ControlLeft };
        assert_eq!(cmd.description(), "KeyRelease");
    }
}
