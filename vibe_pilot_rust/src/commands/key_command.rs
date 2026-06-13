//! Key combination command implementation.
//!
//! Encapsulates keyboard shortcut actions.

use super::{Command, CommandOutcome, ExecutionContext};

/// A command that presses a key combination.
#[derive(Debug, Clone)]
pub struct KeyCombinationCommand {
    /// Keys to press (e.g., `["Ctrl", "C"]`).
    pub keys: Vec<String>,
    /// Description for logging.
    description: String,
}

impl KeyCombinationCommand {
    /// Creates a new key combination command.
    pub fn new(keys: &[&str]) -> Self {
        let desc = keys.join(" + ");
        Self {
            keys: keys.iter().map(|k| k.to_string()).collect(),
            description: format!("Press '{}'", desc),
        }
    }
}

#[async_trait::async_trait]
impl Command for KeyCombinationCommand {
    async fn execute(&self, _context: &mut ExecutionContext) -> CommandOutcome {
        if self.keys.is_empty() {
            return CommandOutcome::Failure {
                reason: "No keys specified".to_string(),
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
    async fn test_key_command_execute() {
        let cmd = KeyCombinationCommand::new(&["Ctrl", "C"]);
        let mut ctx = ExecutionContext::default();
        let outcome = cmd.execute(&mut ctx).await;
        assert!(outcome.is_success());
    }

    #[tokio::test]
    async fn test_key_command_empty_keys() {
        let cmd = KeyCombinationCommand::new(&[]);
        let mut ctx = ExecutionContext::default();
        let outcome = cmd.execute(&mut ctx).await;
        assert!(!outcome.is_success());
    }

    #[test]
    fn test_key_command_description() {
        let cmd = KeyCombinationCommand::new(&["Ctrl", "Shift", "Escape"]);
        assert_eq!(cmd.description, "Press 'Ctrl + Shift + Escape'");
        assert_eq!(cmd.keys, vec!["Ctrl", "Shift", "Escape"]);
    }
}
