//! Type definitions for ownership acceptance.
//!
//! This module contains all struct and enum definitions used
//! for ownership acceptance operations.

use alloy_primitives::{Address, Bytes, B256};
use alloy_sol_types::sol;

// Define contract interfaces
sol! {
    /// Standard Ownable2Step acceptOwnership function.
    #[allow(missing_docs)]
    function acceptOwnership() external;

    /// Transfer ownership to new address (Ownable/Ownable2Step pattern).
    #[allow(missing_docs)]
    function transferOwnership(address newOwner) external;

    /// Read pending owner for Ownable2Step contracts.
    #[allow(missing_docs)]
    function pendingOwner() external view returns (address);

    /// Read current owner.
    #[allow(missing_docs)]
    function owner() external view returns (address);

    /// Get bridged token beacon address from NativeTokenVault.
    #[allow(missing_docs)]
    function bridgedTokenBeacon() external view returns (address);

    /// ChainAdmin multicall interface.
    #[allow(missing_docs)]
    function multicall(
        (address, uint256, bytes)[] calls,
        bool requireSuccess
    ) external;

    /// Governance Call struct for operations.
    #[allow(missing_docs)]
    struct Call {
        address target;
        uint256 value;
        bytes data;
    }

    /// Governance Operation struct.
    #[allow(missing_docs)]
    struct Operation {
        Call[] calls;
        bytes32 predecessor;
        bytes32 salt;
    }

    /// Governance scheduleTransparent function.
    #[allow(missing_docs)]
    function scheduleTransparent(Operation operation, uint256 delay) external;

    /// Governance execute function.
    #[allow(missing_docs)]
    function execute(Operation operation) external payable;

    /// Governance minDelay getter.
    #[allow(missing_docs)]
    function minDelay() external view returns (uint256);
}

/// Contract requiring ownership acceptance.
#[derive(Debug, Clone)]
pub struct OwnershipContract {
    /// Contract name for logging.
    pub name: &'static str,
    /// Contract address.
    pub address: Address,
    /// Ownership acceptance method.
    pub method: OwnershipMethod,
}

/// Method for accepting ownership.
#[derive(Debug, Clone)]
pub enum OwnershipMethod {
    /// Direct acceptOwnership() call to the contract.
    Direct,
    /// Via multicall through chain_admin contract.
    ViaMulticall {
        /// ChainAdmin contract address.
        chain_admin: Address,
    },
}

/// Result of ownership acceptance for a single contract.
#[derive(Debug)]
pub struct OwnershipResult {
    /// Contract name.
    pub name: String,
    /// Whether acceptance succeeded.
    pub success: bool,
    /// Transaction hash if successful.
    pub tx_hash: Option<B256>,
    /// Error message if failed.
    pub error: Option<String>,
}

impl OwnershipResult {
    /// Create a successful result.
    pub fn success(name: &str, tx_hash: B256) -> Self {
        Self {
            name: name.to_string(),
            success: true,
            tx_hash: Some(tx_hash),
            error: None,
        }
    }

    /// Create a failed result.
    pub fn failure(name: &str, error: String) -> Self {
        Self {
            name: name.to_string(),
            success: false,
            tx_hash: None,
            error: Some(error),
        }
    }

    /// Create a skipped result (contract address not configured).
    pub fn skipped(name: &str, reason: &str) -> Self {
        Self {
            name: name.to_string(),
            success: false,
            tx_hash: None,
            error: Some(format!("Skipped: {}", reason)),
        }
    }
}

/// Summary of ownership acceptance operation.
#[derive(Debug)]
pub struct OwnershipSummary {
    /// Results for each contract.
    pub results: Vec<OwnershipResult>,
}

impl OwnershipSummary {
    /// Create a new summary from results.
    pub fn new(results: Vec<OwnershipResult>) -> Self {
        Self { results }
    }

    /// Returns the number of successful acceptances.
    pub fn successful_count(&self) -> usize {
        self.results.iter().filter(|r| r.success).count()
    }

    /// Returns the number of skipped contracts.
    pub fn skipped_count(&self) -> usize {
        self.results
            .iter()
            .filter(|r| !r.success && r.error.as_ref().is_some_and(|e| e.starts_with("Skipped:")))
            .count()
    }

    /// Returns the number of failed acceptances (excludes skipped).
    pub fn failed_count(&self) -> usize {
        self.results
            .iter()
            .filter(|r| !r.success && r.error.as_ref().is_none_or(|e| !e.starts_with("Skipped:")))
            .count()
    }

    /// Returns true if at least one acceptance succeeded.
    pub fn has_successes(&self) -> bool {
        self.successful_count() > 0
    }

    /// Returns true if all acceptances succeeded.
    pub fn all_succeeded(&self) -> bool {
        self.results.iter().all(|r| r.success)
    }
}

/// Ownership state for a contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OwnershipState {
    /// Ownership transfer pending - needs acceptOwnership().
    Pending,
    /// Ownership already accepted - owner is governor.
    Accepted,
    /// Ownership transfer not initiated - owner is not governor, no pending owner.
    NotTransferred,
}

/// Status of a contract's ownership.
#[derive(Debug, Clone)]
pub struct OwnershipStatus {
    /// Contract name.
    pub name: &'static str,
    /// Contract address (None if not configured).
    pub address: Option<Address>,
    /// Current ownership state.
    pub state: OwnershipState,
}

/// Summary of ownership status check.
#[derive(Debug)]
pub struct OwnershipStatusSummary {
    /// Status for each contract.
    pub statuses: Vec<OwnershipStatus>,
}

impl OwnershipStatusSummary {
    /// Returns the number of contracts with pending ownership.
    pub fn pending_count(&self) -> usize {
        self.statuses
            .iter()
            .filter(|s| s.state == OwnershipState::Pending)
            .count()
    }

    /// Returns the number of contracts not configured.
    pub fn not_configured_count(&self) -> usize {
        self.statuses.iter().filter(|s| s.address.is_none()).count()
    }

    /// Returns the number of contracts already accepted.
    pub fn already_accepted_count(&self) -> usize {
        self.statuses
            .iter()
            .filter(|s| s.state == OwnershipState::Accepted)
            .count()
    }

    /// Returns the number of contracts where ownership was not transferred.
    pub fn not_transferred_count(&self) -> usize {
        self.statuses
            .iter()
            .filter(|s| s.state == OwnershipState::NotTransferred)
            .count()
    }
}

/// Transaction calldata for ownership acceptance.
///
/// Contains all information needed to submit a transaction
/// through external tooling (e.g., multisig, Safe).
#[derive(Debug, Clone)]
pub struct CalldataEntry {
    /// Contract name.
    pub name: &'static str,
    /// Target contract address (where to send tx).
    pub to: Address,
    /// Encoded calldata.
    pub calldata: Bytes,
    /// Human-readable description.
    pub description: String,
}

impl CalldataEntry {
    /// Create a new calldata entry.
    pub fn new(name: &'static str, to: Address, calldata: Bytes, description: String) -> Self {
        Self {
            name,
            to,
            calldata,
            description,
        }
    }
}

/// Collection of calldata entries for batch submission.
#[derive(Debug, Default)]
pub struct CalldataOutput {
    /// Calldata entries for each contract.
    pub entries: Vec<CalldataEntry>,
}

impl CalldataOutput {
    /// Create a new empty calldata output.
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Add an entry to the output.
    pub fn push(&mut self, entry: CalldataEntry) {
        self.entries.push(entry);
    }

    /// Returns the number of entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns true if there are no entries.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ownership_result_success() {
        let result = OwnershipResult::success("Test", B256::ZERO);
        assert!(result.success);
        assert!(result.tx_hash.is_some());
        assert!(result.error.is_none());
    }

    #[test]
    fn test_ownership_result_failure() {
        let result = OwnershipResult::failure("Test", "error".to_string());
        assert!(!result.success);
        assert!(result.tx_hash.is_none());
        assert!(result.error.is_some());
    }

    #[test]
    fn test_ownership_summary() {
        let results = vec![
            OwnershipResult::success("A", B256::ZERO),
            OwnershipResult::failure("B", "error".to_string()),
            OwnershipResult::success("C", B256::ZERO),
        ];
        let summary = OwnershipSummary::new(results);
        assert_eq!(summary.successful_count(), 2);
        assert_eq!(summary.failed_count(), 1);
        assert!(summary.has_successes());
        assert!(!summary.all_succeeded());
    }
}
