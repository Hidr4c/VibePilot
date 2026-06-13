//! Wait command implementation.
//!
//! Encapsulates a wait/pause action.

use super::{Command, CommandOutcome, ExecutionContext};

/// A command that waits for a specified duration.
#[derive(Debug, Clone)]
pub struct WaitCommand {
    /// Duration in seconds.
    pub duration_secs: u32,
    /// Optional condition to wait for.
    pub condition: Option<String>,
    /// Description for logging.
    description: String,
}

impl WaitCommand {
    /// Creates a new wait command.
    pub fn new(duration_secs: u32) -> Self {
        Self {
            duration_secs,
            condition: None,
            description: format!("Wait {}s", duration_secs),
        }
    }

    /// Sets a condition to wait for.
    pub fn with_condition(mut self, condition: &str) -> Self {
        self.condition = Some(condition.to_string());
        self.description = format!("Wait for '{}' ({}s)", condition, self.duration_secs);
        self
    }
}

#[async_trait::async_trait]
impl Command for WaitCommand {
    async fn execute(&self, _context: &mut ExecutionContext) -> CommandOutcome {
        if self.duration_secs == 0 {
            return CommandOutcome::Waited {
                reason: "Zero duration wait".to_string(),
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
    async fn test_wait_command_execute() {
        let cmd = WaitCommand::new(5);
        let mut ctx = ExecutionContext::default();
        let outcome = cmd.execute(&mut ctx).await;
        assert!(outcome.is_success());
    }

    #[tokio::test]
    async fn test_wait_command_zero_duration() {
        let cmd = WaitCommand::new(0);
        let mut ctx = ExecutionContext::default();
        let outcome = cmd.execute(&mut ctx).await;
        assert!(!outcome.is_success());
    }

    #[test]
    fn test_wait_command_with_condition() {
        let cmd = WaitCommand::new(30).with_condition("screen_stable");
        assert_eq!(cmd.condition, Some("screen_stable".to_string()));
        assert_eq!(cmd.description, "Wait for 'screen_stable' (30s)");
    }
}
