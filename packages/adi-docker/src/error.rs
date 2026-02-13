//! Error types for Docker operations.

use thiserror::Error;

/// Result type alias using DockerError.
pub type Result<T> = std::result::Result<T, DockerError>;

/// Errors that can occur during Docker operations.
#[derive(Error, Debug)]
pub enum DockerError {
    /// Docker daemon is not running or not accessible.
    #[error("Docker daemon is not running. Please start Docker and try again: {0}")]
    DaemonNotRunning(String),

    /// Failed to connect to Docker daemon.
    #[error("Failed to connect to Docker: {0}")]
    ConnectionFailed(#[from] bollard::errors::Error),

    /// Image not found locally.
    #[error("Image not found locally: {image}")]
    ImageNotFound {
        /// The image that was not found.
        image: String,
    },

    /// Failed to pull image from registry.
    #[error("Failed to pull image {image}: {reason}")]
    PullFailed {
        /// The image that failed to pull.
        image: String,
        /// The reason for failure.
        reason: String,
    },

    /// Container creation failed.
    #[error("Failed to create container: {0}")]
    ContainerCreateFailed(String),

    /// Container execution failed.
    #[error("Container exited with code {exit_code}: {message}")]
    ContainerFailed {
        /// The exit code of the container.
        exit_code: i64,
        /// Error message.
        message: String,
    },

    /// Container timeout.
    #[error("Container operation timed out after {seconds} seconds")]
    Timeout {
        /// Timeout duration in seconds.
        seconds: u64,
    },

    /// Invalid configuration.
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    /// I/O error during stream operations.
    #[error("Stream I/O error: {0}")]
    StreamError(String),
}
