//! Type definitions for contract verification.
//!
//! This module contains all struct and enum definitions used
//! for smart contract verification on block explorers.

use alloy_primitives::Address;
use clap::ValueEnum;
use serde::{Deserialize, Serialize};

/// Block explorer type for verification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, ValueEnum, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExplorerType {
    /// Etherscan-compatible API.
    #[default]
    Etherscan,
    /// Blockscout API.
    Blockscout,
    /// Custom API URL.
    Custom,
}

impl std::fmt::Display for ExplorerType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Etherscan => write!(f, "etherscan"),
            Self::Blockscout => write!(f, "blockscout"),
            Self::Custom => write!(f, "custom"),
        }
    }
}

impl std::str::FromStr for ExplorerType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "etherscan" => Ok(Self::Etherscan),
            "blockscout" => Ok(Self::Blockscout),
            "custom" => Ok(Self::Custom),
            _ => Err(format!("unknown explorer type: {}", s)),
        }
    }
}

impl ExplorerType {
    /// Get the verifier name for forge verify-contract command.
    ///
    /// Always returns "custom" to ensure `--verifier-url` is respected.
    /// Forge ignores `--verifier-url` for non-custom verifiers (etherscan, blockscout)
    /// and tries to look up URLs by chain ID, which fails for unknown chains.
    pub fn forge_verifier_name(&self) -> &'static str {
        // Always use "custom" to ensure --verifier-url is respected
        "custom"
    }
}

/// Verification status from explorer API.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerificationStatus {
    /// Contract is verified on the explorer.
    Verified,
    /// Contract is not verified.
    NotVerified,
    /// Verification is pending (submitted but not confirmed).
    Pending,
    /// Unable to determine status.
    Unknown(String),
}

impl std::fmt::Display for VerificationStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Verified => write!(f, "Verified"),
            Self::NotVerified => write!(f, "Not Verified"),
            Self::Pending => write!(f, "Pending"),
            Self::Unknown(msg) => write!(f, "Unknown: {}", msg),
        }
    }
}

/// Result of a single contract verification attempt.
#[derive(Debug, Clone)]
pub struct VerificationResult {
    /// Contract name.
    pub name: String,
    /// Contract address.
    pub address: Address,
    /// Verification outcome.
    pub outcome: VerificationOutcome,
}

impl VerificationResult {
    /// Create a new verification result.
    #[must_use]
    pub fn new(name: &str, address: Address, outcome: VerificationOutcome) -> Self {
        Self {
            name: name.to_string(),
            address,
            outcome,
        }
    }

    /// Create an already verified result.
    #[must_use]
    pub fn already_verified(name: &str, address: Address) -> Self {
        Self::new(name, address, VerificationOutcome::AlreadyVerified)
    }

    /// Create a submitted result.
    #[must_use]
    pub fn submitted(name: &str, address: Address, guid: String) -> Self {
        Self::new(name, address, VerificationOutcome::Submitted { guid })
    }

    /// Create a confirmed result.
    #[must_use]
    pub fn confirmed(name: &str, address: Address) -> Self {
        Self::new(name, address, VerificationOutcome::Confirmed)
    }

    /// Create a failed result.
    #[must_use]
    pub fn failed(name: &str, address: Address, reason: String) -> Self {
        Self::new(name, address, VerificationOutcome::Failed { reason })
    }

    /// Create a skipped result.
    #[must_use]
    pub fn skipped(name: &str, address: Address, reason: String) -> Self {
        Self::new(name, address, VerificationOutcome::Skipped { reason })
    }

    /// Returns true if verification succeeded or was already verified.
    pub fn is_success(&self) -> bool {
        matches!(
            self.outcome,
            VerificationOutcome::AlreadyVerified
                | VerificationOutcome::Submitted { .. }
                | VerificationOutcome::Confirmed
        )
    }
}

/// Outcome of a verification attempt.
#[derive(Debug, Clone)]
pub enum VerificationOutcome {
    /// Already verified, skipped.
    AlreadyVerified,
    /// Successfully submitted for verification.
    Submitted {
        /// Verification GUID from explorer.
        guid: String,
    },
    /// Verification confirmed.
    Confirmed,
    /// Verification failed.
    Failed {
        /// Failure reason.
        reason: String,
    },
    /// Skipped (no address configured or filtered out).
    Skipped {
        /// Skip reason.
        reason: String,
    },
}

impl std::fmt::Display for VerificationOutcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AlreadyVerified => write!(f, "Already Verified"),
            Self::Submitted { guid } => write!(f, "Submitted (GUID: {})", guid),
            Self::Confirmed => write!(f, "Confirmed"),
            Self::Failed { reason } => write!(f, "Failed: {}", reason),
            Self::Skipped { reason } => write!(f, "Skipped: {}", reason),
        }
    }
}

/// Summary of verification operation.
#[derive(Debug, Default)]
pub struct VerificationSummary {
    /// Results for each contract.
    pub results: Vec<VerificationResult>,
}

impl VerificationSummary {
    /// Create a new summary from results.
    #[must_use]
    pub fn new(results: Vec<VerificationResult>) -> Self {
        Self { results }
    }

    /// Returns the number of contracts already verified.
    pub fn already_verified_count(&self) -> usize {
        self.results
            .iter()
            .filter(|r| matches!(r.outcome, VerificationOutcome::AlreadyVerified))
            .count()
    }

    /// Returns the number of successfully submitted verifications.
    pub fn submitted_count(&self) -> usize {
        self.results
            .iter()
            .filter(|r| matches!(r.outcome, VerificationOutcome::Submitted { .. }))
            .count()
    }

    /// Returns the number of confirmed verifications.
    pub fn confirmed_count(&self) -> usize {
        self.results
            .iter()
            .filter(|r| matches!(r.outcome, VerificationOutcome::Confirmed))
            .count()
    }

    /// Returns the number of skipped contracts.
    pub fn skipped_count(&self) -> usize {
        self.results
            .iter()
            .filter(|r| matches!(r.outcome, VerificationOutcome::Skipped { .. }))
            .count()
    }

    /// Returns the number of failed verifications.
    pub fn failed_count(&self) -> usize {
        self.results
            .iter()
            .filter(|r| matches!(r.outcome, VerificationOutcome::Failed { .. }))
            .count()
    }

    /// Returns true if all contracts are verified (including already verified).
    pub fn all_verified(&self) -> bool {
        self.results.iter().all(|r| {
            matches!(
                r.outcome,
                VerificationOutcome::AlreadyVerified | VerificationOutcome::Confirmed
            )
        })
    }

    /// Returns true if there are contracts needing verification.
    pub fn needs_verification(&self) -> bool {
        self.results
            .iter()
            .any(|r| matches!(r.outcome, VerificationOutcome::Skipped { .. }))
            || self.failed_count() > 0
    }
}

/// Status check result for a contract.
#[derive(Debug, Clone)]
pub struct ContractVerificationStatus {
    /// Contract name.
    pub name: String,
    /// Contract address.
    pub address: Address,
    /// Verification status.
    pub status: VerificationStatus,
}

impl ContractVerificationStatus {
    /// Create a new status entry.
    #[must_use]
    pub fn new(name: &str, address: Address, status: VerificationStatus) -> Self {
        Self {
            name: name.to_string(),
            address,
            status,
        }
    }

    /// Returns true if the contract is verified.
    pub fn is_verified(&self) -> bool {
        matches!(self.status, VerificationStatus::Verified)
    }
}

/// Summary of verification status check.
#[derive(Debug, Default)]
pub struct VerificationStatusSummary {
    /// Status for each contract.
    pub statuses: Vec<ContractVerificationStatus>,
}

impl VerificationStatusSummary {
    /// Create a new summary.
    #[must_use]
    pub fn new(statuses: Vec<ContractVerificationStatus>) -> Self {
        Self { statuses }
    }

    /// Returns the number of verified contracts.
    pub fn verified_count(&self) -> usize {
        self.statuses.iter().filter(|s| s.is_verified()).count()
    }

    /// Returns the number of unverified contracts.
    pub fn unverified_count(&self) -> usize {
        self.statuses
            .iter()
            .filter(|s| matches!(s.status, VerificationStatus::NotVerified))
            .count()
    }

    /// Returns true if all contracts are verified.
    pub fn all_verified(&self) -> bool {
        self.statuses.iter().all(|s| s.is_verified())
    }

    /// Returns contracts that need verification.
    pub fn needs_verification(&self) -> Vec<&ContractVerificationStatus> {
        self.statuses.iter().filter(|s| !s.is_verified()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verification_outcome_display() {
        assert_eq!(
            VerificationOutcome::AlreadyVerified.to_string(),
            "Already Verified"
        );
        assert_eq!(VerificationOutcome::Confirmed.to_string(), "Confirmed");
    }

    #[test]
    fn test_verification_summary() {
        let results = vec![
            VerificationResult::already_verified("A", Address::ZERO),
            VerificationResult::confirmed("B", Address::ZERO),
            VerificationResult::failed("C", Address::ZERO, "error".to_string()),
        ];
        let summary = VerificationSummary::new(results);
        assert_eq!(summary.already_verified_count(), 1);
        assert_eq!(summary.confirmed_count(), 1);
        assert_eq!(summary.failed_count(), 1);
        assert!(!summary.all_verified());
    }
}
