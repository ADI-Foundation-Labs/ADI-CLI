//! Logger trait for decoupled logging across ADI packages.
//!
//! This module provides a `Logger` trait that abstracts logging operations,
//! allowing packages to log without coupling to specific logging frameworks.
//!
//! # Example
//!
//! ```rust
//! use adi_types::{Logger, NoopLogger};
//! use std::sync::Arc;
//!
//! fn do_work(logger: &dyn Logger) {
//!     logger.debug("Starting work...");
//!     logger.info("Processing item");
//!     logger.success("Item processed");
//! }
//!
//! let logger = Arc::new(NoopLogger);
//! do_work(&*logger);
//! ```

use std::fmt;

/// Trait for logging messages at various levels.
///
/// Implementations can target different outputs:
/// - CLI: `cliclack` for user-facing output, `log::debug!` for debug
/// - Tests: No-op implementation
/// - Custom: Any other logging backend
///
/// All methods are infallible - logging failures should not affect business logic.
pub trait Logger: Send + Sync {
    /// Log a debug message (only shown with debug flag enabled).
    fn debug(&self, message: &str);

    /// Log an informational message.
    fn info(&self, message: &str);

    /// Log a warning message.
    fn warning(&self, message: &str);

    /// Log a success message.
    fn success(&self, message: &str);

    /// Log an error message.
    fn error(&self, message: &str);

    /// Log a debug message with formatted arguments.
    fn debug_fmt(&self, args: fmt::Arguments<'_>) {
        self.debug(&args.to_string());
    }

    /// Log an info message with formatted arguments.
    fn info_fmt(&self, args: fmt::Arguments<'_>) {
        self.info(&args.to_string());
    }

    /// Log a warning message with formatted arguments.
    fn warning_fmt(&self, args: fmt::Arguments<'_>) {
        self.warning(&args.to_string());
    }

    /// Log a success message with formatted arguments.
    fn success_fmt(&self, args: fmt::Arguments<'_>) {
        self.success(&args.to_string());
    }

    /// Log an error message with formatted arguments.
    fn error_fmt(&self, args: fmt::Arguments<'_>) {
        self.error(&args.to_string());
    }
}

/// No-op logger that discards all messages.
///
/// Useful for tests and SDK consumers who do not need logging output.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoopLogger;

impl Logger for NoopLogger {
    fn debug(&self, _message: &str) {}
    fn info(&self, _message: &str) {}
    fn warning(&self, _message: &str) {}
    fn success(&self, _message: &str) {}
    fn error(&self, _message: &str) {}
}

/// Logger implementation using the `log` crate.
///
/// Maps logger levels to `log` crate levels:
/// - `debug` -> `log::debug!`
/// - `info` -> `log::info!`
/// - `warning` -> `log::warn!`
/// - `success` -> `log::info!` (with prefix)
/// - `error` -> `log::error!`
#[derive(Debug, Default, Clone, Copy)]
pub struct LogCrateLogger;

impl Logger for LogCrateLogger {
    fn debug(&self, message: &str) {
        log::debug!("{}", message);
    }

    fn info(&self, message: &str) {
        log::info!("{}", message);
    }

    fn warning(&self, message: &str) {
        log::warn!("{}", message);
    }

    fn success(&self, message: &str) {
        log::info!("[SUCCESS] {}", message);
    }

    fn error(&self, message: &str) {
        log::error!("{}", message);
    }
}
