//! Error types for ecosystem operations.

use std::path::PathBuf;
use thiserror::Error;

/// Result type alias using EcosystemError.
pub type Result<T> = std::result::Result<T, EcosystemError>;

/// Errors that can occur during ecosystem operations.
#[derive(Error, Debug)]
pub enum EcosystemError {
    /// Required file is missing.
    #[error("Required file is missing: {0}")]
    MissingFile(PathBuf),

    /// Invalid configuration.
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    /// Ecosystem already exists.
    #[error("Ecosystem '{0}' already exists")]
    AlreadyExists(String),
}
