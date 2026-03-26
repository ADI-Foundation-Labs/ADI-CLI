//! Error types for upgrade operations.

use thiserror::Error;

/// Result type alias using UpgradeError.
pub type Result<T> = std::result::Result<T, UpgradeError>;

/// Errors that can occur during upgrade operations.
#[derive(Error, Debug)]
pub enum UpgradeError {
    /// Unsupported protocol version.
    #[error("Unsupported protocol version: {0}")]
    UnsupportedVersion(String),

    /// Configuration error.
    #[error("Configuration error: {0}")]
    Config(String),

    /// Simulation failed.
    #[error("Simulation failed: {0}")]
    SimulationFailed(String),

    /// Broadcast failed.
    #[error("Broadcast failed: {0}")]
    BroadcastFailed(String),

    /// Bytecode validation failed.
    #[error("Bytecode validation failed: {0}")]
    ValidationFailed(String),

    /// Governance transaction failed.
    #[error("Governance transaction failed: {0}")]
    GovernanceFailed(String),
}
