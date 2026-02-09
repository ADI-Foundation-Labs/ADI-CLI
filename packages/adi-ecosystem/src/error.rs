//! Error types for ecosystem operations.

use std::path::PathBuf;
use thiserror::Error;

/// Result type alias using EcosystemError.
pub type Result<T> = std::result::Result<T, EcosystemError>;

/// Errors that can occur during ecosystem operations.
#[derive(Error, Debug)]
pub enum EcosystemError {
    /// Required file is missing.
    #[error("Required file is missing: {}", .0.display())]
    MissingFile(PathBuf),

    /// Invalid configuration.
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    /// Ecosystem already exists.
    #[error("Ecosystem '{0}' already exists")]
    AlreadyExists(String),

    /// Required contract address is missing after deployment.
    #[error("Missing contract address: {0}")]
    MissingContract(String),

    /// Transaction failed during deployment.
    #[error("Transaction failed: {reason}")]
    TransactionFailed {
        /// Reason for failure.
        reason: String,
    },
}
