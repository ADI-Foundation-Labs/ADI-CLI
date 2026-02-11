//! UI utilities for CLI output.
//!
//! This module re-exports cliclack functions for cleaner imports.
//! Use `log::debug!` for debug-level logging (cliclack has no debug level).

// Re-export logging functions
pub use cliclack::log::{error, info, success, warning};

// Re-export interactive components
pub use cliclack::{confirm, input, intro, note, outro, outro_cancel};
