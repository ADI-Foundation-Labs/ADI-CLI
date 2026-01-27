//! Wrapper for the zkstack CLI.
//!
//! This module provides `ZkstackCli`, which wraps the zkstack command-line tool
//! for ZkSync ecosystem and chain management operations. The wrapper handles:
//!
//! - Command construction with proper arguments
//! - Async execution via `tokio::process::Command`
//! - Output capture and parsing
//! - Error handling with context
//!
//! # Example
//!
//! ```rust,ignore
//! use adi_cli::external::ZkstackCli;
//!
//! #[tokio::main]
//! async fn main() -> eyre::Result<()> {
//!     let zkstack = ZkstackCli::new();
//!
//!     // Check version
//!     let version = zkstack.version().await?;
//!     println!("zkstack version: {}", version);
//!
//!     Ok(())
//! }
//! ```

use crate::error::{Result, WrapErr};
use std::ffi::OsStr;
use std::process::Stdio;
use tokio::process::Command;

/// Output from a zkstack command execution.
#[derive(Debug, Clone)]
pub struct CommandOutput {
    /// Exit code from the command (0 = success).
    pub exit_code: i32,
    /// Standard output from the command.
    pub stdout: String,
    /// Standard error from the command.
    pub stderr: String,
}

impl CommandOutput {
    /// Returns true if the command succeeded (exit code 0).
    pub fn success(&self) -> bool {
        self.exit_code == 0
    }
}

/// Wrapper for the zkstack CLI tool.
///
/// Provides typed methods for common zkstack operations including
/// ecosystem creation, initialization, and chain management.
///
/// # Example
///
/// ```rust,ignore
/// use adi_cli::external::ZkstackCli;
///
/// let zkstack = ZkstackCli::new();
///
/// // Execute a custom command
/// let output = zkstack.execute(&["ecosystem", "create", "--help"]).await?;
/// if output.success() {
///     println!("{}", output.stdout);
/// }
/// ```
// Note: Currently unused as commands are implemented in later phases (US1-US6)
#[allow(dead_code)]
pub struct ZkstackCli {
    /// Path to the zkstack binary. Defaults to "zkstack".
    binary_path: String,
}

#[allow(dead_code)]
impl ZkstackCli {
    /// Creates a new ZkstackCli instance with the default binary path.
    pub fn new() -> Self {
        Self {
            binary_path: "zkstack".to_string(),
        }
    }

    /// Creates a new ZkstackCli instance with a custom binary path.
    ///
    /// # Arguments
    ///
    /// * `binary_path` - Path to the zkstack binary.
    pub fn with_binary_path(binary_path: impl Into<String>) -> Self {
        Self {
            binary_path: binary_path.into(),
        }
    }

    /// Returns the path to the zkstack binary.
    pub fn binary_path(&self) -> &str {
        &self.binary_path
    }

    /// Executes a zkstack command with the given arguments.
    ///
    /// # Arguments
    ///
    /// * `args` - Command-line arguments to pass to zkstack.
    ///
    /// # Returns
    ///
    /// A `CommandOutput` containing the exit code, stdout, and stderr.
    ///
    /// # Errors
    ///
    /// Returns an error if the command fails to spawn or execute.
    pub async fn execute<I, S>(&self, args: I) -> Result<CommandOutput>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let output = Command::new(&self.binary_path)
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .wrap_err_with(|| format!("Failed to execute zkstack command: {}", self.binary_path))?;

        let exit_code = output.status.code().unwrap_or(-1);
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        Ok(CommandOutput {
            exit_code,
            stdout,
            stderr,
        })
    }

    /// Gets the zkstack version.
    ///
    /// # Returns
    ///
    /// The version string from `zkstack --version`.
    ///
    /// # Errors
    ///
    /// Returns an error if the command fails or version cannot be parsed.
    pub async fn version(&self) -> Result<String> {
        let output = self.execute(["--version"]).await?;

        if !output.success() {
            return Err(eyre::eyre!(
                "zkstack --version failed with exit code {}: {}",
                output.exit_code,
                output.stderr
            ));
        }

        // Parse version from output (format: "zkstack X.Y.Z" or similar)
        Ok(output.stdout.trim().to_string())
    }

    /// Checks if zkstack is available and returns version info.
    ///
    /// # Returns
    ///
    /// `Ok(version)` if zkstack is available, or an error if not found.
    pub async fn check_available(&self) -> Result<String> {
        self.version()
            .await
            .wrap_err("zkstack CLI not found. Ensure it is installed and available in PATH.")
    }
}

impl Default for ZkstackCli {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_output_success() {
        let output = CommandOutput {
            exit_code: 0,
            stdout: "success".to_string(),
            stderr: String::new(),
        };
        assert!(output.success());
    }

    #[test]
    fn test_command_output_failure() {
        let output = CommandOutput {
            exit_code: 1,
            stdout: String::new(),
            stderr: "error".to_string(),
        };
        assert!(!output.success());
    }

    #[test]
    fn test_zkstack_cli_default_path() {
        let zkstack = ZkstackCli::new();
        assert_eq!(zkstack.binary_path(), "zkstack");
    }

    #[test]
    fn test_zkstack_cli_custom_path() {
        let zkstack = ZkstackCli::with_binary_path("/custom/path/zkstack");
        assert_eq!(zkstack.binary_path(), "/custom/path/zkstack");
    }
}
