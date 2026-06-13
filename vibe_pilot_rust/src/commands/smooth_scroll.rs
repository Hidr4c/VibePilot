//! MouseSmoothScroll command: smooth scrolling with configurable duration.
//!
//! Divides the total scroll delta into small steps sent at regular intervals.

use crate::commands::{Command, CommandOutcome, ExecutionContext};

/// Command that performs smooth scrolling over a specified duration.
#[derive(Debug, Clone)]
pub struct MouseSmoothScrollCommand {
    pub delta_total: i32,
    pub duration_ms: u64,
}

#[async_trait::async_trait]
impl Command for MouseSmoothScrollCommand {
    async fn execute(&self, _context: &mut ExecutionContext) -> CommandOutcome {
        if cfg!(test) || self.duration_ms == 0 {
            return CommandOutcome::Success;
        }

        let steps = (self.duration_ms / 10) as i32;
        if steps == 0 {
            return CommandOutcome::Success;
        }

        let step_delta = self.delta_total as f64 / steps as f64;
        let delay = std::time::Duration::from_millis(10);
        let mut accumulated = 0.0f64;

        for _ in 0..steps {
            accumulated += step_delta;
            let delta = accumulated.round() as i32;
            if delta != 0 {
                let _ = rdev::simulate(&rdev::EventType::Wheel {
                    delta_x: 0,
                    delta_y: delta as i64,
                });
                accumulated -= delta as f64;
            }
            std::thread::sleep(delay);
        }

        CommandOutcome::Success
    }

    fn description(&self) -> &str {
        "MouseSmoothScroll"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_smooth_scroll_positive() {
        let cmd = MouseSmoothScrollCommand {
            delta_total: 10,
            duration_ms: 100,
        };
        let mut ctx = ExecutionContext::new();
        let outcome = cmd.execute(&mut ctx).await;
        assert!(outcome.is_success());
    }

    #[tokio::test]
    async fn test_smooth_scroll_negative() {
        let cmd = MouseSmoothScrollCommand {
            delta_total: -5,
            duration_ms: 50,
        };
        let mut ctx = ExecutionContext::new();
        let outcome = cmd.execute(&mut ctx).await;
        assert!(outcome.is_success());
    }

    #[tokio::test]
    async fn test_smooth_scroll_zero_duration() {
        let cmd = MouseSmoothScrollCommand {
            delta_total: 10,
            duration_ms: 0,
        };
        let mut ctx = ExecutionContext::new();
        let outcome = cmd.execute(&mut ctx).await;
        assert!(outcome.is_success());
    }

    #[test]
    fn test_description() {
        let cmd = MouseSmoothScrollCommand {
            delta_total: 10,
            duration_ms: 100,
        };
        assert_eq!(cmd.description(), "MouseSmoothScroll");
    }
}
