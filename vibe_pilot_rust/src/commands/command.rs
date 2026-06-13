//! Command trait and outcome types for peripheral actions.
//!
//! Defines the interface that all action commands must implement.

/// Result of executing a command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandOutcome {
    /// The command executed successfully.
    Success,
    /// The command failed with a reason.
    Failure { reason: String },
    /// The command requires more time (e.g., waiting for an element).
    Waited { reason: String, duration_secs: u32 },
}

impl CommandOutcome {
    /// Returns `true` if the outcome was successful.
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Success)
    }

    /// Returns the failure reason if this is a `Failure` or `Waited` outcome.
    pub fn failure_reason(&self) -> Option<&str> {
        match self {
            Self::Failure { reason } => Some(reason),
            Self::Waited { reason, .. } => Some(reason),
            _ => None,
        }
    }
}

/// Context available during command execution.
pub struct ExecutionContext {
    /// Current mouse offset for absolute coordinate computation.
    pub mouse_offset_x: i32,
    pub mouse_offset_y: i32,
    /// Target window title.
    pub target_window_title: Option<String>,
    /// Current screenshot bounds.
    pub screen_bounds: (i32, i32, i32, i32),
}

impl ExecutionContext {
    /// Creates a new execution context with default values.
    pub fn new() -> Self {
        Self {
            mouse_offset_x: 0,
            mouse_offset_y: 0,
            target_window_title: None,
            screen_bounds: (0, 0, 1920, 1080),
        }
    }

    /// Converts relative coordinates (0..1) to absolute pixel coordinates.
    pub fn to_absolute(&self, rel_x: f64, rel_y: f64) -> (i32, i32) {
        let (left, top, width, height) = self.screen_bounds;
        let abs_x = (left + (rel_x * width as f64) as i32 + self.mouse_offset_x)
            .max(left)
            .min(left + width);
        let abs_y = (top + (rel_y * height as f64) as i32 + self.mouse_offset_y)
            .max(top)
            .min(top + height);
        (abs_x, abs_y)
    }
}

impl Default for ExecutionContext {
    fn default() -> Self { Self::new() }
}

/// Trait for executing a peripheral action.
///
/// Each concrete command implements this trait to encapsulate
/// the logic for a specific action type.
#[async_trait::async_trait]
pub trait Command: Send + Sync {
    /// Executes the command and returns the outcome.
    async fn execute(&self, context: &mut ExecutionContext) -> CommandOutcome;

    /// Returns a human-readable description of this command.
    fn description(&self) -> &str;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_outcome_success() {
        let outcome = CommandOutcome::Success;
        assert!(outcome.is_success());
        assert!(outcome.failure_reason().is_none());
    }

    #[test]
    fn test_command_outcome_failure() {
        let outcome = CommandOutcome::Failure {
            reason: "device error".to_string(),
        };
        assert!(!outcome.is_success());
        assert_eq!(outcome.failure_reason(), Some("device error"));
    }

    #[test]
    fn test_command_outcome_waited() {
        let outcome = CommandOutcome::Waited {
            reason: "element not found".to_string(),
            duration_secs: 30,
        };
        assert!(!outcome.is_success());
        assert_eq!(outcome.failure_reason(), Some("element not found"));
    }

    #[test]
    fn test_execution_context_default() {
        let ctx = ExecutionContext::default();
        assert_eq!(ctx.mouse_offset_x, 0);
        assert_eq!(ctx.mouse_offset_y, 0);
        assert_eq!(ctx.screen_bounds, (0, 0, 1920, 1080));
    }

    #[test]
    fn test_execution_context_to_absolute() {
        let mut ctx = ExecutionContext::new();
        ctx.screen_bounds = (100, 200, 800, 600);
        ctx.mouse_offset_x = 50;
        ctx.mouse_offset_y = 30;

        let (x, y) = ctx.to_absolute(0.5, 0.5);
        assert_eq!(x, 550);
        assert_eq!(y, 530);
    }

    #[test]
    fn test_execution_context_to_absolute_clamped() {
        let mut ctx = ExecutionContext::new();
        ctx.screen_bounds = (0, 0, 1920, 1080);

        let (x, y) = ctx.to_absolute(-0.1, 1.1);
        assert_eq!(x, 0);
        assert_eq!(y, 1080);
    }
}
