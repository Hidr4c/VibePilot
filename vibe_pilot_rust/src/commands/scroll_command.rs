//! Scroll command implementation.
//!
//! Encapsulates a mouse scroll action.

use super::{Command, CommandOutcome, ExecutionContext};

/// A command that scrolls the mouse wheel.
#[derive(Debug, Clone)]
pub struct ScrollCommand {
    /// Scroll value (positive = up, negative = down).
    pub scroll_value: i32,
    /// Scroll direction: "vertical" or "horizontal".
    pub direction: String,
    /// Description for logging.
    description: String,
}

impl ScrollCommand {
    /// Creates a new vertical scroll command.
    pub fn vertical(scroll_value: i32) -> Self {
        Self {
            scroll_value,
            direction: "vertical".to_string(),
            description: format!("Scroll {} (vertical)", scroll_value),
        }
    }

    /// Creates a new horizontal scroll command.
    pub fn horizontal(scroll_value: i32) -> Self {
        Self {
            scroll_value,
            direction: "horizontal".to_string(),
            description: format!("Scroll {} (horizontal)", scroll_value),
        }
    }
}

#[async_trait::async_trait]
impl Command for ScrollCommand {
    async fn execute(&self, _context: &mut ExecutionContext) -> CommandOutcome {
        if self.scroll_value == 0 {
            return CommandOutcome::Waited {
                reason: "Zero scroll value".to_string(),
                duration_secs: 0,
            };
        }
        CommandOutcome::Success
    }

    fn description(&self) -> &str {
        &self.description
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_scroll_command_vertical() {
        let cmd = ScrollCommand::vertical(-6);
        let mut ctx = ExecutionContext::default();
        let outcome = cmd.execute(&mut ctx).await;
        assert!(outcome.is_success());
    }

    #[tokio::test]
    async fn test_scroll_command_horizontal() {
        let cmd = ScrollCommand::horizontal(3);
        let mut ctx = ExecutionContext::default();
        let outcome = cmd.execute(&mut ctx).await;
        assert!(outcome.is_success());
    }

    #[tokio::test]
    async fn test_scroll_command_zero_value() {
        let cmd = ScrollCommand::vertical(0);
        let mut ctx = ExecutionContext::default();
        let outcome = cmd.execute(&mut ctx).await;
        assert!(!outcome.is_success());
    }

    #[test]
    fn test_scroll_command_descriptions() {
        let v = ScrollCommand::vertical(-6);
        assert_eq!(v.description, "Scroll -6 (vertical)");

        let h = ScrollCommand::horizontal(3);
        assert_eq!(h.description, "Scroll 3 (horizontal)");
    }
}
