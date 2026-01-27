//! Logging utilities for adi-cli.
//!
//! Standard logging uses the `log` crate macros (info!, warn!, error!, debug!).
//! This module provides a success! macro for user-facing success messages.

/// Print a success message with green formatting.
///
/// Success messages are user-facing output (not diagnostic logging),
/// so they bypass the log system and print directly to stdout.
///
/// # Example
///
/// ```ignore
/// use crate::success;
/// success!("Deployment completed successfully");
/// ```
#[macro_export]
macro_rules! success {
    ($($arg:tt)*) => {{
        use colored::Colorize;
        println!("{}", format!($($arg)*).green());
    }};
}
