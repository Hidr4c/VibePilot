//! Command pattern for peripheral action execution.
//!
//! Provides a trait-based command system where each action type (click, type,
//! wait, scroll, key combination) is encapsulated as a concrete command.
//!
//! # Architecture
//!
//! - `Command` trait defines the interface for all action commands.
//! - `CommandOutcome` represents the result of command execution.
//! - Concrete commands: `ClickCommand`, `TypeCommand`, `WaitCommand`,
//!   `ScrollCommand`, `KeyCombinationCommand`.
//!
//! # Examples
//!
//! ```ignore
//! use vibe_pilot_rust::commands::{ClickCommand, Command};
//!
//! let click = ClickCommand::new(0.5, 0.5);
//! let outcome = click.execute(&mut context).await;
//! ```

pub mod command;
pub mod click_command;
pub mod type_command;
pub mod wait_command;
pub mod scroll_command;
pub mod key_command;
pub mod type_with_delay;
pub mod smooth_scroll;
pub mod key_hold_release;

pub use command::{Command, CommandOutcome, ExecutionContext};
pub use click_command::ClickCommand;
pub use type_command::TypeCommand;
pub use wait_command::WaitCommand;
pub use scroll_command::ScrollCommand;
pub use key_command::KeyCombinationCommand;
pub use type_with_delay::TypeWithDelayCommand;
pub use smooth_scroll::MouseSmoothScrollCommand;
pub use key_hold_release::{KeyHoldCommand, KeyReleaseCommand};
