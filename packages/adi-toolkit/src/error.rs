//! Error types for toolkit operations.

use thiserror::Error;

/// Result type alias using ToolkitError.
pub type Result<T> = std::result::Result<T, ToolkitError>;

/// Errors that can occur during toolkit operations.
#[derive(Error, Debug)]
pub enum ToolkitError {
    /// Docker operation failed.
    #[error("Docker error: {0}")]
    Docker(#[from] adi_docker::DockerError),

    /// Command execution failed.
    #[error("Command failed with exit code {exit_code}: {message}")]
    CommandFailed {
        /// Exit code.
        exit_code: i64,
        /// Error message.
        message: String,
    },

    /// Invalid protocol version.
    #[error("Invalid protocol version: {0}")]
    InvalidVersion(String),
}
