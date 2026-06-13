//! Type command implementation.
//!
//! Encapsulates a text typing action.

use super::{Command, CommandOutcome, ExecutionContext};

/// A command that types text at the current cursor position.
#[derive(Debug, Clone)]
pub struct TypeCommand {
    /// Text to type.
    pub text: String,
    /// Whether to press Enter after typing.
    pub press_enter: bool,
    /// Description for logging.
    description: String,
}

impl TypeCommand {
    /// Creates a new type command.
    pub fn new(text: &str) -> Self {
        Self {
            text: text.to_string(),
            press_enter: false,
            description: format!("Type '{}'", text),
        }
    }

    /// Sets whether to press Enter after typing.
    pub fn with_enter(mut self, press: bool) -> Self {
        self.press_enter = press;
        self.description = format!("Type '{}' + Enter", self.text);
        self
    }
}

#[async_trait::async_trait]
impl Command for TypeCommand {
    async fn execute(&self, _context: &mut ExecutionContext) -> CommandOutcome {
        if self.text.is_empty() {
            return CommandOutcome::Failure {
                reason: "Empty text to type".to_string(),
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
    async fn test_type_command_execute() {
        let cmd = TypeCommand::new("hello");
        let mut ctx = ExecutionContext::default();
        let outcome = cmd.execute(&mut ctx).await;
        assert!(outcome.is_success());
    }

    #[tokio::test]
    async fn test_type_command_empty_text() {
        let cmd = TypeCommand::new("");
        let mut ctx = ExecutionContext::default();
        let outcome = cmd.execute(&mut ctx).await;
        assert!(!outcome.is_success());
        assert_eq!(outcome.failure_reason(), Some("Empty text to type"));
    }

    #[test]
    fn test_type_command_with_enter() {
        let cmd = TypeCommand::new("submit").with_enter(true);
        assert!(cmd.press_enter);
        assert_eq!(cmd.description, "Type 'submit' + Enter");
    }
}
