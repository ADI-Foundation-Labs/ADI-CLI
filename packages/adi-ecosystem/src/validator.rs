//! Validator role management for ZkSync chains.
//!
//! This module provides types and functions for assigning validator roles
//! to operator wallets via the ValidatorTimelock contract.

use alloy_primitives::{Address, Bytes, U256};
use alloy_sol_types::{sol, SolCall};

// Define the contract interfaces using alloy's sol! macro
sol! {
    /// ValidatorTimelock interface for addValidatorRoles.
    #[allow(missing_docs)]
    function addValidatorRoles(
        address diamondProxy,
        address operator,
        (bool, bool, bool, bool, bool) roles
    ) external;

    /// ChainAdmin multicall interface.
    #[allow(missing_docs)]
    function multicall(
        (address, uint256, bytes)[] calls,
        bool requireSuccess
    ) external;
}

/// Validator roles that can be assigned to an operator.
#[derive(Debug, Clone, Copy)]
pub struct ValidatorRoles {
    /// Can precommit batches (submit commitment before finalization).
    pub precommitter: bool,
    /// Can commit batches to L1.
    pub committer: bool,
    /// Can revert committed batches.
    pub reverter: bool,
    /// Can submit validity proofs.
    pub prover: bool,
    /// Can execute batches after proof verification.
    pub executor: bool,
}

impl ValidatorRoles {
    /// Create roles for the commit operator (precommitter, committer, reverter).
    ///
    /// This operator handles batch commitment and can revert if needed.
    #[must_use]
    pub fn commit_operator() -> Self {
        Self {
            precommitter: true,
            committer: true,
            reverter: true,
            prover: false,
            executor: false,
        }
    }

    /// Create roles for the prove operator (prover only).
    ///
    /// This operator submits validity proofs to L1.
    #[must_use]
    pub fn prove_operator() -> Self {
        Self {
            precommitter: false,
            committer: false,
            reverter: false,
            prover: true,
            executor: false,
        }
    }

    /// Create roles for the execute operator (executor only).
    ///
    /// This operator executes batches after proof verification.
    #[must_use]
    pub fn execute_operator() -> Self {
        Self {
            precommitter: false,
            committer: false,
            reverter: false,
            prover: false,
            executor: true,
        }
    }

    /// Convert to tuple format for ABI encoding.
    fn to_tuple(self) -> (bool, bool, bool, bool, bool) {
        (
            self.precommitter,
            self.committer,
            self.reverter,
            self.prover,
            self.executor,
        )
    }
}

/// Build calldata for adding validator roles via multicall.
///
/// This function builds the calldata for calling `multicall` on the ChainAdmin
/// contract, which internally calls `addValidatorRoles` on the ValidatorTimelock.
///
/// # Arguments
///
/// * `validator_timelock` - The ValidatorTimelock contract address.
/// * `diamond_proxy` - The Diamond proxy contract address.
/// * `operator` - The operator address to grant roles to.
/// * `roles` - The validator roles to assign.
///
/// # Returns
///
/// ABI-encoded calldata for the multicall transaction.
#[must_use]
pub fn build_add_validator_roles_calldata(
    validator_timelock: Address,
    diamond_proxy: Address,
    operator: Address,
    roles: ValidatorRoles,
) -> Bytes {
    // Build inner call to addValidatorRoles
    let inner_call = addValidatorRolesCall {
        diamondProxy: diamond_proxy,
        operator,
        roles: roles.to_tuple(),
    };
    let inner_calldata = Bytes::from(inner_call.abi_encode());

    // Build outer multicall: [(validator_timelock, 0, calldata)]
    let multicall_call = multicallCall {
        calls: vec![(validator_timelock, U256::ZERO, inner_calldata)],
        requireSuccess: true,
    };

    Bytes::from(multicall_call.abi_encode())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_commit_operator_roles() {
        let roles = ValidatorRoles::commit_operator();
        assert!(roles.precommitter);
        assert!(roles.committer);
        assert!(roles.reverter);
        assert!(!roles.prover);
        assert!(!roles.executor);
    }

    #[test]
    fn test_prove_operator_roles() {
        let roles = ValidatorRoles::prove_operator();
        assert!(!roles.precommitter);
        assert!(!roles.committer);
        assert!(!roles.reverter);
        assert!(roles.prover);
        assert!(!roles.executor);
    }

    #[test]
    fn test_execute_operator_roles() {
        let roles = ValidatorRoles::execute_operator();
        assert!(!roles.precommitter);
        assert!(!roles.committer);
        assert!(!roles.reverter);
        assert!(!roles.prover);
        assert!(roles.executor);
    }

    #[test]
    fn test_build_calldata_not_empty() {
        let validator_timelock = Address::ZERO;
        let diamond_proxy = Address::ZERO;
        let operator = Address::ZERO;
        let roles = ValidatorRoles::commit_operator();

        let calldata = build_add_validator_roles_calldata(
            validator_timelock,
            diamond_proxy,
            operator,
            roles,
        );

        // Calldata should not be empty
        assert!(!calldata.is_empty());
        // Should start with multicall selector (first 4 bytes)
        assert!(calldata.len() >= 4);
    }
}
