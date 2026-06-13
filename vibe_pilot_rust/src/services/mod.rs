//! Shared services extracted from VibePilotApp.
//!
//! These modules encapsulate cross-cutting concerns:
//! - `config`: Configuration and profile management
//! - `llm`: LLM API operations
//! - `migrator`: Storage directory migration

pub mod config;
pub mod llm;
pub mod migrator;
pub mod wait_manager;
pub mod wait_manager_factory;
pub mod api;
pub mod calibration;
pub mod accessibility;

pub use wait_manager_factory::WaitManagerFactory;
pub use calibration::{AutoCalibrator, VisualDiffAutoCalibrator, AutoCalibratorFactory};
pub use accessibility::{AccessibilityParser, Win32AccessibilityParser, AccessibilityParserFactory};
