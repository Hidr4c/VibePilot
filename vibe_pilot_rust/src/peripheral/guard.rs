use std::sync::Arc;

/// Trait defining security policies for typed commands.
pub trait CommandGuard: Send + Sync {
    /// Validates if a command string is safe to run.
    /// Returns Ok(()) if safe, or Err(message) describing the security violation.
    fn validate_command(&self, command: &str) -> Result<(), String>;
}

/// Default implementation checking for known destructive command patterns.
pub struct DefaultCommandGuard;

impl DefaultCommandGuard {
    /// Creates a new DefaultCommandGuard.
    pub fn new() -> Self {
        Self
    }
}

impl CommandGuard for DefaultCommandGuard {
    fn validate_command(&self, command: &str) -> Result<(), String> {
        if crate::config::contains_dangerous_command(command) {
            Err(format!(
                "Security Alert: Executing this command is blocked or requires explicit approval: '{}'",
                command
            ))
        } else {
            Ok(())
        }
    }
}

/// Factory to build CommandGuard instances following the IoC pattern.
pub struct CommandGuardFactory;

impl CommandGuardFactory {
    /// Creates the default CommandGuard implementation.
    pub fn create() -> Arc<dyn CommandGuard> {
        Arc::new(DefaultCommandGuard::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_command_guard() {
        let guard = DefaultCommandGuard::new();
        // Safe commands
        assert!(guard.validate_command("cargo test").is_ok());
        assert!(guard.validate_command("echo hello").is_ok());

        // Dangerous commands
        assert!(guard.validate_command("rm -rf /").is_err());
        assert!(guard.validate_command("format C:").is_err());
    }

    #[test]
    fn test_factory_creation() {
        let guard = CommandGuardFactory::create();
        assert!(guard.validate_command("ls").is_ok());
        assert!(guard.validate_command("rm -rf /").is_err());
    }
}

