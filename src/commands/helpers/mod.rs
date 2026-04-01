//! Shared helper functions for CLI commands.
//!
//! This module contains common utilities used across multiple commands,
//! reducing code duplication.

mod display;
mod resolution;
mod selection;
mod state;

pub use display::*;
pub use resolution::*;
pub use selection::*;
pub use state::*;

use adi_ecosystem::OwnershipResult;
use clap::ValueEnum;
use serde::{Deserialize, Serialize};

/// Scope for ownership operations (accept/transfer).
///
/// Determines which contracts are included in ownership operations.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, ValueEnum, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OwnershipScope {
    /// Ecosystem-level contracts only (Governance, ValidatorTimelock, etc.)
    Ecosystem,

    /// Chain-level contracts only (Chain Governance, Chain ChainAdmin)
    Chain,

    /// All contracts (ecosystem + chain) - default behavior
    #[default]
    All,
}

impl std::fmt::Display for OwnershipScope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ecosystem => write!(f, "ecosystem"),
            Self::Chain => write!(f, "chain"),
            Self::All => write!(f, "all"),
        }
    }
}

/// Result of chain selection from config.
///
/// Distinguishes between selecting an existing chain (with config defaults)
/// versus creating a new chain that isn't in the config file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChainSelection {
    /// User selected an existing chain defined in config.
    /// The chain's defaults from `ecosystem.chains[]` should be used.
    Existing(String),

    /// User wants to create a new chain not in config.
    /// Command should use default values or prompt for configuration.
    New(String),
}

impl ChainSelection {
    /// Get the chain name regardless of selection type.
    #[must_use]
    pub fn name(&self) -> &str {
        match self {
            Self::Existing(name) | Self::New(name) => name,
        }
    }
}

/// Category of ownership result for display purposes.
pub enum ResultCategory<'a> {
    /// Transaction was successful and has a hash.
    SuccessWithTx(String),
    /// Success without a transaction hash.
    SuccessNoTx,
    /// Operation was skipped with reason.
    Skipped(&'a str),
    /// Operation failed with error.
    Failed(&'a str),
}

/// Categorize an ownership result for display.
pub fn categorize_result(result: &OwnershipResult) -> ResultCategory<'_> {
    if result.success {
        match &result.tx_hash {
            Some(tx) => ResultCategory::SuccessWithTx(tx.to_string()),
            None => ResultCategory::SuccessNoTx,
        }
    } else {
        match &result.error {
            Some(e) if e.starts_with("Skipped: ") => {
                ResultCategory::Skipped(e.strip_prefix("Skipped: ").unwrap_or(e))
            }
            Some(e) => ResultCategory::Failed(e),
            None => ResultCategory::Failed("unknown error"),
        }
    }
}
