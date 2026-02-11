//! UI utilities for CLI output.
//!
//! This module provides the CLI logger implementation using cliclack
//! for user-facing output and `log::debug!` for debug messages.

use adi_types::Logger;
use std::sync::Arc;

// Re-export logging functions
pub use cliclack::log::{error, info, success, warning};

// Re-export interactive components
pub use cliclack::{confirm, input, intro, note, outro, outro_cancel};

/// CLI logger using cliclack for user-facing output.
///
/// - `debug()` uses `log::debug!` (shown only with -d flag)
/// - `info()` uses `cliclack::log::info` (shows `●` symbol)
/// - `warning()` uses `cliclack::log::warning` (shows `▲` symbol)
/// - `success()` uses `cliclack::log::success` (shows `◆` symbol)
/// - `error()` uses `cliclack::log::error`
#[derive(Debug, Default, Clone, Copy)]
pub struct CliLogger;

impl Logger for CliLogger {
    fn debug(&self, message: &str) {
        log::debug!("{}", message);
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

/// Create a shared CLI logger instance.
pub fn cli_logger() -> Arc<dyn Logger> {
    Arc::new(CliLogger)
}
