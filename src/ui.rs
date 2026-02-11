//! UI utilities for CLI output.
//!
//! This module provides the CLI logger implementation using cliclack
//! for user-facing output with consistent visual styling.

use adi_types::Logger;
use std::sync::Arc;

// Re-export logging functions
pub use cliclack::log::{error, info, success, warning};

// Re-export interactive components
pub use cliclack::{confirm, input, intro, note, outro, outro_cancel};

/// CLI logger using cliclack for user-facing output.
///
/// - `debug()` uses `cliclack::log::remark` (shown only when debug_enabled is true)
/// - `info()` uses `cliclack::log::info` (shows `●` symbol)
/// - `warning()` uses `cliclack::log::warning` (shows `▲` symbol)
/// - `success()` uses `cliclack::log::success` (shows `◆` symbol)
/// - `error()` uses `cliclack::log::error`
#[derive(Debug, Clone, Copy, Default)]
pub struct CliLogger {
    debug_enabled: bool,
}

impl CliLogger {
    /// Create a new CLI logger.
    ///
    /// # Arguments
    ///
    /// * `debug_enabled` - Whether to show debug messages.
    #[must_use]
    pub fn new(debug_enabled: bool) -> Self {
        Self { debug_enabled }
    }
}

impl Logger for CliLogger {
    fn debug(&self, message: &str) {
        if self.debug_enabled {
            let _ = cliclack::log::remark(message);
        }
    }

    fn info(&self, message: &str) {
        let _ = cliclack::log::info(message);
    }

    fn warning(&self, message: &str) {
        let _ = cliclack::log::warning(message);
    }

    fn success(&self, message: &str) {
        let _ = cliclack::log::success(message);
    }

    fn error(&self, message: &str) {
        let _ = cliclack::log::error(message);
    }
}

/// Create a shared CLI logger instance with configurable debug.
///
/// # Arguments
///
/// * `debug_enabled` - Whether to show debug messages.
pub fn cli_logger_with_debug(debug_enabled: bool) -> Arc<dyn Logger> {
    Arc::new(CliLogger::new(debug_enabled))
}
