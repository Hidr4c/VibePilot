//! Unified error types for VibePilot.
//!
//! This module defines `VibeError` as the central error type across all
//! VibePilot modules, replacing ad-hoc `String` error propagation.
//!
//! # Design
//!
//! `VibeError` wraps the most common failure modes:
//! - `IO` — file system / I/O errors
//! - `Serialization` — JSON / MessagePack / deserialization failures
//! - `Encryption` — AES-GCM / DPAPI failures
//! - `Llm` — LLM API / HTTP errors
//! - `Image` — image encoding / decoding / pixel access failures
//! - `Input` — peripheral input simulation failures
//! - `Generic` — catch-all for custom error messages
//!
//! # Examples
//!
//! ```no_run
//! use vibe_pilot_rust::error::VibeError;
//!
//! fn do_work() -> Result<(), VibeError> {
//!     // ...
//!     Err(VibeError::Llm("API timeout".to_string()))
//! }
//! ```

use std::fmt;

/// Unified error type for VibePilot.
#[derive(Debug, Clone, PartialEq)]
pub enum VibeError {
    /// File system or I/O error.
    Io(String),
    /// Serialization or deserialization error.
    Serialization(String),
    /// Encryption or decryption error.
    Encryption(String),
    /// LLM API or HTTP error.
    Llm(String),
    /// Image encoding / decoding / pixel access error.
    Image(String),
    /// Peripheral input simulation error.
    Input(String),
    /// Generic catch-all error.
    Generic(String),
}

impl VibeError {
    /// Returns a human-readable string description.
    pub fn message(&self) -> &str {
        match self {
            VibeError::Io(msg)
            | VibeError::Serialization(msg)
            | VibeError::Encryption(msg)
            | VibeError::Llm(msg)
            | VibeError::Image(msg)
            | VibeError::Input(msg)
            | VibeError::Generic(msg) => msg,
        }
    }

    /// Returns the error category.
    pub fn category(&self) -> &str {
        match self {
            VibeError::Io(_) => "IO",
            VibeError::Serialization(_) => "Serialization",
            VibeError::Encryption(_) => "Encryption",
            VibeError::Llm(_) => "LLM",
            VibeError::Image(_) => "Image",
            VibeError::Input(_) => "Input",
            VibeError::Generic(_) => "Generic",
        }
    }
}

impl fmt::Display for VibeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.category(), self.message())
    }
}

impl std::error::Error for VibeError {}

impl From<std::io::Error> for VibeError {
    fn from(e: std::io::Error) -> Self {
        VibeError::Io(e.to_string())
    }
}

impl From<serde_json::Error> for VibeError {
    fn from(e: serde_json::Error) -> Self {
        VibeError::Serialization(e.to_string())
    }
}

impl From<aes_gcm::Error> for VibeError {
    fn from(e: aes_gcm::Error) -> Self {
        VibeError::Encryption(e.to_string())
    }
}

impl From<reqwest::Error> for VibeError {
    fn from(e: reqwest::Error) -> Self {
        VibeError::Llm(e.to_string())
    }
}

impl From<image::ImageError> for VibeError {
    fn from(e: image::ImageError) -> Self {
        VibeError::Image(e.to_string())
    }
}

impl From<arboard::Error> for VibeError {
    fn from(e: arboard::Error) -> Self {
        VibeError::Input(format!("Clipboard: {}", e))
    }
}

impl From<String> for VibeError {
    fn from(s: String) -> Self {
        VibeError::Generic(s)
    }
}

impl From<&str> for VibeError {
    fn from(s: &str) -> Self {
        VibeError::Generic(s.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vibe_error_categories() {
        assert_eq!(VibeError::Io("test".into()).category(), "IO");
        assert_eq!(VibeError::Serialization("test".into()).category(), "Serialization");
        assert_eq!(VibeError::Encryption("test".into()).category(), "Encryption");
        assert_eq!(VibeError::Llm("test".into()).category(), "LLM");
        assert_eq!(VibeError::Image("test".into()).category(), "Image");
        assert_eq!(VibeError::Input("test".into()).category(), "Input");
        assert_eq!(VibeError::Generic("test".into()).category(), "Generic");
    }

    #[test]
    fn test_vibe_error_display() {
        let err = VibeError::Llm("API timeout".into());
        assert!(format!("{}", err).contains("LLM"));
        assert!(format!("{}", err).contains("API timeout"));
    }

    #[test]
    fn test_vibe_error_message() {
        let err = VibeError::Generic("custom error".into());
        assert_eq!(err.message(), "custom error");
    }

    #[test]
    fn test_vibe_error_from_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let vibe_err: VibeError = io_err.into();
        assert!(matches!(vibe_err, VibeError::Io(_)));
    }

    #[test]
    fn test_vibe_error_from_string() {
        let s: VibeError = "test".into();
        assert!(matches!(s, VibeError::Generic(ref msg) if msg == "test"));
    }

    #[test]
    fn test_vibe_error_equality() {
        let e1 = VibeError::Llm("timeout".into());
        let e2 = VibeError::Llm("timeout".into());
        let e3 = VibeError::Llm("network error".into());
        assert_eq!(e1, e2);
        assert_ne!(e1, e3);
    }

    #[test]
    fn test_vibe_error_from_json() {
        let json_res: Result<serde_json::Value, _> = serde_json::from_str("{invalid");
        let json_err = json_res.unwrap_err();
        let vibe_err: VibeError = json_err.into();
        assert!(matches!(vibe_err, VibeError::Serialization(_)));
    }

    #[test]
    fn test_vibe_error_from_aes_gcm() {
        let aes_err = aes_gcm::Error;
        let vibe_err: VibeError = aes_err.into();
        assert!(matches!(vibe_err, VibeError::Encryption(_)));
    }

    #[test]
    fn test_vibe_error_from_arboard() {
        let arb_err = arboard::Error::ClipboardNotSupported;
        let vibe_err: VibeError = arb_err.into();
        assert!(matches!(vibe_err, VibeError::Input(_)));
    }

    #[test]
    fn test_vibe_error_from_string_type() {
        let s = "string error".to_string();
        let vibe_err: VibeError = s.into();
        assert!(matches!(vibe_err, VibeError::Generic(ref msg) if msg == "string error"));
    }

    #[test]
    fn test_vibe_error_std_error_trait() {
        let err = VibeError::Generic("std error".into());
        let std_err: Box<dyn std::error::Error> = Box::new(err);
        assert_eq!(std_err.to_string(), "[Generic] std error");
    }
}
