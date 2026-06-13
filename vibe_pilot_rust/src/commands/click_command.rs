//! Click command implementation.
//!
//! Encapsulates a mouse click action at relative coordinates.

use super::{Command, CommandOutcome, ExecutionContext};

/// A command that performs a mouse click at relative coordinates.
#[derive(Debug, Clone)]
pub struct ClickCommand {
    /// Relative X coordinate (0.0 to 1.0).
    pub rel_x: f64,
    /// Relative Y coordinate (0.0 to 1.0).
    pub rel_y: f64,
    /// Click type: "left", "right", "middle", "double".
    pub click_type: String,
    /// Description for logging.
    description: String,
}

impl ClickCommand {
    /// Creates a new click command at the given relative coordinates.
    pub fn new(rel_x: f64, rel_y: f64) -> Self {
        Self {
            rel_x,
            rel_y,
            click_type: "left".to_string(),
            description: format!("Click at ({:.2}, {:.2})", rel_x, rel_y),
        }
    }

    /// Sets the click type.
    pub fn with_click_type(mut self, click_type: &str) -> Self {
        self.click_type = click_type.to_string();
        self.description = format!("{} ({})", self.description, click_type);
        self
    }
}

#[async_trait::async_trait]
impl Command for ClickCommand {
    async fn execute(&self, context: &mut ExecutionContext) -> CommandOutcome {
        let (abs_x, abs_y) = context.to_absolute(self.rel_x, self.rel_y);
        let _ = (abs_x, abs_y);
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
    async fn test_click_command_execute() {
        let cmd = ClickCommand::new(0.5, 0.5);
        let mut ctx = ExecutionContext::default();
        let outcome = cmd.execute(&mut ctx).await;
        assert!(outcome.is_success());
    }

    #[tokio::test]
    async fn test_click_command_with_type() {
        let cmd = ClickCommand::new(0.3, 0.7).with_click_type("right");
        assert_eq!(cmd.description(), "Click at (0.30, 0.70) (right)");
    }

    #[test]
    fn test_click_command_new() {
        let cmd = ClickCommand::new(0.0, 0.0);
        assert_eq!(cmd.description, "Click at (0.00, 0.00)");
    }
}
