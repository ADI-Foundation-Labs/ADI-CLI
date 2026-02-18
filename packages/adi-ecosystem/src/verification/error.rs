//! Error types for contract verification.

use super::types::ExplorerType;
use thiserror::Error;

/// Verification-specific errors.
#[derive(Debug, Error)]
pub enum VerificationError {
    /// Explorer API error.
    #[error("Explorer API error: {0}")]
    ExplorerApi(String),

    /// Rate limited by explorer API.
    #[error("Rate limited by explorer API")]
    RateLimited,

    /// Forge verification command failed.
    #[error("Forge verification failed: {0}")]
    ForgeFailed(String),

    /// Contract not found in state.
    #[error("Contract not found in state: {0}")]
    ContractNotFound(String),

    /// API key required for explorer.
    #[error("API key required for {0} explorer")]
    ApiKeyRequired(ExplorerType),

    /// Explorer URL required for custom explorer.
    #[error("Explorer URL required for custom explorer type")]
    ExplorerUrlRequired,

    /// Constructor arguments not available.
    #[error("Constructor args not available for {0}")]
    MissingConstructorArgs(String),

    /// Invalid explorer configuration.
    #[error("Invalid explorer configuration: {0}")]
    InvalidConfig(String),

    /// HTTP request failed.
    #[error("HTTP request failed: {0}")]
    HttpError(String),

    /// JSON parsing error.
    #[error("Failed to parse JSON response: {0}")]
    JsonParse(String),

    /// Chain ID not found.
    #[error("Chain ID not found for network")]
    ChainIdNotFound,
}

impl From<reqwest::Error> for VerificationError {
    fn from(err: reqwest::Error) -> Self {
        Self::HttpError(err.to_string())
    }
}
