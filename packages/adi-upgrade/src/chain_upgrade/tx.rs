//! Transaction helpers for chain upgrades.
//!
//! Sends raw transactions and protocol-specific calls to chain contracts.

use alloy_primitives::{Address, Bytes, U256};
use alloy_provider::Provider;
use alloy_rpc_types::TransactionRequest;

use crate::error::{Result, UpgradeError};
use crate::signing::build_signing_provider;

/// Convert semver Version to protocol version uint256.
///
/// Formula: `(major << 40) | (minor << 32) | patch`
///
/// # Examples
///
/// - v0.30.0 -> `0x1e00000000`
/// - v0.30.1 -> `0x1e00000001`
pub fn version_to_protocol_uint(version: &semver::Version) -> U256 {
    let major = U256::from(version.major);
    let minor = U256::from(version.minor);
    let patch = U256::from(version.patch);

    (major << 40) | (minor << 32) | patch
}

/// Send a raw transaction to a contract address.
pub(crate) async fn send_chain_tx<P: Provider + Clone>(
    provider: &P,
    signer_key: &secrecy::SecretString,
    to: Address,
    calldata: Bytes,
    label: &str,
) -> Result<alloy_primitives::B256> {
    let signing_provider = build_signing_provider(provider, signer_key)?;

    log::info!("Sending {label} tx to {to}...");

    let tx = TransactionRequest::default()
        .to(to)
        .input(calldata.into())
        .value(U256::ZERO);

    let pending = signing_provider
        .send_transaction(tx)
        .await
        .map_err(|e| UpgradeError::GovernanceFailed(format!("{label} tx failed: {e}")))?;

    let receipt = pending
        .get_receipt()
        .await
        .map_err(|e| UpgradeError::GovernanceFailed(format!("{label} receipt failed: {e}")))?;

    log::info!("{label} tx: {}", receipt.transaction_hash);
    Ok(receipt.transaction_hash)
}

/// Call `ChainAdmin.setUpgradeTimestamp(uint256 protocolVersion, uint256 upgradeTimestamp)`.
pub(crate) async fn set_upgrade_timestamp<P: Provider + Clone>(
    provider: &P,
    signer_key: &secrecy::SecretString,
    chain_admin: Address,
    protocol_version: &semver::Version,
) -> Result<alloy_primitives::B256> {
    use alloy_sol_types::SolCall;

    alloy_sol_types::sol! {
        function setUpgradeTimestamp(uint256 protocolVersion, uint256 upgradeTimestamp) external;
    }

    let version_uint = version_to_protocol_uint(protocol_version);

    let timestamp = U256::from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| UpgradeError::Config(format!("Failed to get timestamp: {e}")))?
            .as_secs()
            + 1,
    );

    log::info!(
        "Setting upgrade timestamp: version={}, timestamp={}",
        version_uint,
        timestamp
    );

    let calldata = setUpgradeTimestampCall {
        protocolVersion: version_uint,
        upgradeTimestamp: timestamp,
    }
    .abi_encode();

    send_chain_tx(
        provider,
        signer_key,
        chain_admin,
        Bytes::from(calldata),
        "setUpgradeTimestamp",
    )
    .await
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_version_to_protocol_uint_v0_30_0() {
        let version = semver::Version::new(0, 30, 0);
        let result = version_to_protocol_uint(&version);
        // (0 << 40) | (30 << 32) | 0 = 30 * 2^32 = 0x1e00000000
        assert_eq!(result, U256::from(0x1e00000000u64));
    }

    #[test]
    fn test_version_to_protocol_uint_v0_30_1() {
        let version = semver::Version::new(0, 30, 1);
        let result = version_to_protocol_uint(&version);
        assert_eq!(result, U256::from(0x1e00000001u64));
    }

    #[test]
    fn test_version_to_protocol_uint_v1_0_0() {
        let version = semver::Version::new(1, 0, 0);
        let result = version_to_protocol_uint(&version);
        // (1 << 40) = 0x10000000000
        assert_eq!(result, U256::from(0x10000000000u64));
    }
}
