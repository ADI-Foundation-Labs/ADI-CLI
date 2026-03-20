//! ABI encoding for governance calldata.
//!
//! Encodes stage1 calls into scheduleTransparent and execute calldatas.

use alloy_primitives::{Bytes, B256, U256};
use alloy_sol_types::{sol, SolCall, SolType};

use crate::error::{Result, UpgradeError};

// Define the Governance contract types.
sol! {
    /// A single call in a governance operation.
    struct Call {
        address target;
        uint256 value;
        bytes data;
    }

    /// A governance operation containing multiple calls.
    struct Operation {
        Call[] calls;
        bytes32 predecessor;
        bytes32 salt;
    }

    /// Schedule a transparent governance operation.
    function scheduleTransparent(Operation calldata _operation, uint256 _delay) external;

    /// Execute a governance operation.
    function execute(Operation calldata _operation) external;
}

/// Encoded governance calldatas.
#[derive(Debug, Clone)]
pub struct GovernanceCalldata {
    /// scheduleTransparent calldata (4-byte selector + ABI-encoded args).
    pub schedule_transparent: Bytes,
    /// execute calldata (4-byte selector + ABI-encoded args).
    pub execute: Bytes,
}

/// Encode stage1 calls hex into governance calldatas.
///
/// Takes the raw ABI-encoded `Call[]` from forge script output,
/// wraps it into an `Operation` with zero predecessor/salt,
/// and produces scheduleTransparent + execute calldatas.
pub fn encode_governance_calls(stage1_calls_hex: &str) -> Result<GovernanceCalldata> {
    let hex_clean = stage1_calls_hex.trim().trim_start_matches("0x");
    let bytes = hex::decode(hex_clean)
        .map_err(|e| UpgradeError::GovernanceFailed(format!("Invalid hex in stage1_calls: {e}")))?;

    // Decode the ABI-encoded Call[]
    let calls = <sol!(Call[])>::abi_decode(&bytes)
        .map_err(|e| UpgradeError::GovernanceFailed(format!("Failed to ABI-decode Call[]: {e}")))?;

    log::info!("Decoded {} governance calls", calls.len());

    // Build Operation with zero predecessor and salt
    let operation = Operation {
        calls,
        predecessor: B256::ZERO,
        salt: B256::ZERO,
    };

    // Encode scheduleTransparent(operation, 0)
    let schedule_call = scheduleTransparentCall {
        _operation: operation.clone(),
        _delay: U256::ZERO,
    };
    let schedule_transparent = Bytes::from(schedule_call.abi_encode());

    // Encode execute(operation)
    let execute_call = executeCall {
        _operation: operation,
    };
    let execute = Bytes::from(execute_call.abi_encode());

    Ok(GovernanceCalldata {
        schedule_transparent,
        execute,
    })
}
