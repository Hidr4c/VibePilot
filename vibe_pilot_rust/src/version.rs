//! Version information for VibePilot.
//!
//! This module provides version info that is set at compile time via Cargo build scripts.

/// Returns the application version string (e.g., "1.0.0" or "dev").
pub fn get_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// Returns the git hash if available, or "unknown".
pub fn get_git_hash() -> &'static str {
    option_env!("GIT_HASH").unwrap_or("unknown")
}

/// Returns the build date if available, or "unknown".
pub fn get_build_date() -> &'static str {
    option_env!("BUILD_DATE").unwrap_or("unknown")
}

/// Returns all version info as a formatted string.
pub fn get_version_info() -> String {
    format!(
        "VibePilot v{} (git: {}, built: {})",
        get_version(),
        get_git_hash(),
        get_build_date(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_version() {
        let version = get_version();
        assert!(!version.is_empty());
        assert!(version.len() > 0);
    }

    #[test]
    fn test_get_git_hash() {
        let hash = get_git_hash();
        assert!(!hash.is_empty());
    }

    #[test]
    fn test_get_build_date() {
        let date = get_build_date();
        assert!(!date.is_empty());
    }

    #[test]
    fn test_get_version_info() {
        let info = get_version_info();
        assert!(info.contains("VibePilot"));
        assert!(info.contains("v"));
    }
}
