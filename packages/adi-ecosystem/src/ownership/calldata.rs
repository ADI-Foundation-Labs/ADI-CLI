//! Calldata builders for ownership acceptance transactions.
//!
//! This module provides functions to build encoded calldata for
//! various ownership acceptance patterns.

use super::types::{
    acceptOwnershipCall, executeCall, multicallCall, scheduleTransparentCall, transferOwnershipCall,
};
use super::types::{Call, Operation};
use alloy_primitives::{Address, Bytes, B256, U256};
use alloy_sol_types::SolCall;

/// Build calldata for acceptOwnership() call.
#[must_use]
pub fn build_accept_ownership_calldata() -> Bytes {
    let call = acceptOwnershipCall {};
    Bytes::from(call.abi_encode())
}

/// Build calldata for transferOwnership(newOwner) call.
#[must_use]
pub fn build_transfer_ownership_calldata(new_owner: Address) -> Bytes {
    let call = transferOwnershipCall {
        newOwner: new_owner,
    };
    Bytes::from(call.abi_encode())
}

/// Build calldata for acceptOwnership via multicall.
///
/// This wraps the acceptOwnership call in a multicall transaction
/// to be sent to the ChainAdmin contract.
#[must_use]
pub fn build_accept_ownership_multicall_calldata(target_contract: Address) -> Bytes {
    // Build inner call to acceptOwnership
    let inner_call = acceptOwnershipCall {};
    let inner_calldata = Bytes::from(inner_call.abi_encode());

    // Build outer multicall: [(target, 0, calldata)]
    let multicall_call = multicallCall {
        calls: vec![(target_contract, U256::ZERO, inner_calldata)],
        requireSuccess: true,
    };

    Bytes::from(multicall_call.abi_encode())
}

/// Build calldata for governance scheduleTransparent call.
///
/// Used to schedule an acceptOwnership operation on a target contract
/// through the Governance timelock.
#[must_use]
pub fn build_governance_schedule_calldata(target: Address, salt: B256) -> Bytes {
    // Build the inner acceptOwnership call
    let accept_call = acceptOwnershipCall {};
    let accept_calldata = Bytes::from(accept_call.abi_encode());

    // Create the Call struct
    let call = Call {
        target,
        value: U256::ZERO,
        data: accept_calldata,
    };

    // Create the Operation struct
    let operation = Operation {
        calls: vec![call],
        predecessor: B256::ZERO,
        salt,
    };

    // Build scheduleTransparent(operation, 0) - delay=0 for immediate execution
    let schedule_call = scheduleTransparentCall {
        operation,
        delay: U256::ZERO,
    };

    Bytes::from(schedule_call.abi_encode())
}

/// Build calldata for governance execute call.
///
/// Used to execute a previously scheduled operation.
#[must_use]
pub fn build_governance_execute_calldata(target: Address, salt: B256) -> Bytes {
    // Build the inner acceptOwnership call
    let accept_call = acceptOwnershipCall {};
    let accept_calldata = Bytes::from(accept_call.abi_encode());

    // Create the Call struct
    let call = Call {
        target,
        value: U256::ZERO,
        data: accept_calldata,
    };

    // Create the Operation struct (must match what was scheduled)
    let operation = Operation {
        calls: vec![call],
        predecessor: B256::ZERO,
        salt,
    };

    // Build execute(operation)
    let execute_call = executeCall { operation };

    Bytes::from(execute_call.abi_encode())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_accept_ownership_calldata() {
        let calldata = build_accept_ownership_calldata();
        // acceptOwnership() selector is 0x79ba5097
        assert!(!calldata.is_empty());
        assert!(calldata.len() >= 4);
    }

    #[test]
    fn test_build_transfer_ownership_calldata() {
        let new_owner = Address::ZERO;
        let calldata = build_transfer_ownership_calldata(new_owner);
        // transferOwnership(address) selector is 0xf2fde38b
        assert!(!calldata.is_empty());
        // 4 bytes selector + 32 bytes address
        assert!(calldata.len() >= 36);
    }

    #[test]
    fn test_build_multicall_calldata() {
        let target = Address::ZERO;
        let calldata = build_accept_ownership_multicall_calldata(target);
        // Should contain multicall selector
        assert!(!calldata.is_empty());
        assert!(calldata.len() >= 4);
    }
}
