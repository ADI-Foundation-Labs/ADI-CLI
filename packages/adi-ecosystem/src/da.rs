//! Data Availability (DA) configuration for ZkSync chains.
//!
//! This module provides functions for configuring DA mode on chains,
//! specifically for L3 deployments that need calldata-based pubdata
//! instead of EIP-4844 blobs.

use crate::error::{EcosystemError, Result};
use adi_types::{normalize_rpc_url, Logger};
use alloy_network::{EthereumWallet, TransactionBuilder};
use alloy_primitives::{Address, Bytes, B256, U256};
use alloy_provider::{Provider, ProviderBuilder};
use alloy_rpc_types::TransactionRequest;
use alloy_signer_local::PrivateKeySigner;
use alloy_sol_types::{sol, SolCall};
use console::Style;
use secrecy::{ExposeSecret, SecretString};

// Define the contract interfaces using alloy's sol! macro
sol! {
    /// Set DA validator pair on Diamond proxy.
    /// Called through ChainAdmin multicall.
    #[allow(missing_docs)]
    function setDAValidatorPair(address l1DAValidator, uint8 pubdataSource) external;

    /// ChainAdmin multicall interface.
    #[allow(missing_docs)]
    function multicall(
        (address, uint256, bytes)[] calls,
        bool requireSuccess
    ) external;
}

/// Pubdata source modes for DA configuration.
///
/// These values correspond to the `PubdataSource` enum in the ZkSync contracts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PubdataSource {
    /// No DA - Validium mode.
    EmptyNoDa = 1,
    /// Calldata-based pubdata (no blobs). Used for L3 chains.
    PubdataKeccak256 = 2,
    /// Blobs and pubdata keccak256 (Era v0.29 default).
    BlobsAndPubdataKeccak256 = 3,
    /// ZKsync OS with blobs.
    BlobsZksyncOs = 4,
}

/// Build calldata for `setDAValidatorPair` via ChainAdmin multicall.
///
/// # Arguments
///
/// * `diamond_proxy` - The Diamond proxy contract address.
/// * `l1_da_validator` - The L1 DA validator contract address.
/// * `pubdata_source` - The pubdata source mode.
///
/// # Returns
///
/// ABI-encoded calldata for the multicall transaction.
#[must_use]
pub fn build_set_da_validator_pair_multicall_calldata(
    diamond_proxy: Address,
    l1_da_validator: Address,
    pubdata_source: PubdataSource,
) -> Bytes {
    // Build inner call to setDAValidatorPair
    let inner_call = setDAValidatorPairCall {
        l1DAValidator: l1_da_validator,
        pubdataSource: pubdata_source as u8,
    };
    let inner_calldata = Bytes::from(inner_call.abi_encode());

    // Build outer multicall: [(diamond_proxy, 0, calldata)]
    let multicall_call = multicallCall {
        calls: vec![(diamond_proxy, U256::ZERO, inner_calldata)],
        requireSuccess: true,
    };

    Bytes::from(multicall_call.abi_encode())
}

/// Configure L3 DA mode (calldata-based pubdata).
///
/// This function sends a transaction to disable blobs and use calldata-based
/// pubdata instead. Required for L3 chains deploying on L2 settlement layers
/// that don't support EIP-4844 blobs.
///
/// # Arguments
///
/// * `rpc_url` - Settlement layer RPC endpoint URL.
/// * `chain_admin` - ChainAdmin contract address.
/// * `diamond_proxy` - Diamond proxy contract address.
/// * `l1_da_validator` - RollupL1DAValidator contract address.
/// * `governor_key` - Governor private key for signing transactions.
/// * `gas_multiplier` - Gas price multiplier percentage. None to use raw estimate.
/// * `logger` - Logger for debug/info/warning output.
///
/// # Returns
///
/// Transaction hash on success.
///
/// # Errors
///
/// Returns error if transaction fails or required addresses are invalid.
pub async fn configure_l3_da(
    rpc_url: &str,
    chain_admin: Address,
    diamond_proxy: Address,
    l1_da_validator: Address,
    governor_key: &SecretString,
    gas_multiplier: Option<u64>,
    logger: &dyn Logger,
) -> Result<B256> {
    logger.debug(&format!(
        "Configuring L3 DA mode via chain_admin: {}",
        chain_admin
    ));

    // Create signer from governor key
    let signer = create_signer(governor_key)?;
    let governor_address = signer.address();
    logger.debug(&format!("Governor address: {}", governor_address));

    // Create signing provider
    let wallet = EthereumWallet::from(signer);
    let normalized_rpc = normalize_rpc_url(rpc_url);
    let url: url::Url = normalized_rpc.parse().map_err(|e| {
        EcosystemError::InvalidConfig(format!("Invalid RPC URL '{}': {}", rpc_url, e))
    })?;
    let provider = ProviderBuilder::new().wallet(wallet).connect_http(url);

    // Get chain ID and nonce
    let chain_id =
        provider
            .get_chain_id()
            .await
            .map_err(|e| EcosystemError::TransactionFailed {
                reason: format!("Failed to get chain ID: {}", e),
            })?;

    let nonce = provider
        .get_transaction_count(governor_address)
        .await
        .map_err(|e| EcosystemError::TransactionFailed {
            reason: format!("Failed to get nonce: {}", e),
        })?;

    // Estimate gas price and apply multiplier if provided
    let estimated =
        provider
            .get_gas_price()
            .await
            .map_err(|e| EcosystemError::TransactionFailed {
                reason: format!("Failed to get gas price: {}", e),
            })?;
    let gas_price = gas_multiplier.map_or(estimated, |m| estimated * u128::from(m) / 100);
    logger.debug(&format!("Using gas price: {} wei", gas_price));

    // Build calldata for setDAValidatorPair via multicall
    let calldata = build_set_da_validator_pair_multicall_calldata(
        diamond_proxy,
        l1_da_validator,
        PubdataSource::BlobsAndPubdataKeccak256,
    );

    let green = Style::new().green();
    let spinner = cliclack::spinner();
    spinner.start(format!(
        "Setting DA validator pair to calldata mode ({})",
        green.apply_to(l1_da_validator)
    ));

    // Build transaction to chain_admin
    let tx = TransactionRequest::default()
        .with_from(governor_address)
        .with_to(chain_admin)
        .with_input(calldata)
        .with_nonce(nonce)
        .with_gas_limit(100_000) // Conservative gas limit
        .with_gas_price(gas_price)
        .with_chain_id(chain_id);

    // Send transaction
    let pending = provider.send_transaction(tx).await.map_err(|e| {
        spinner.error(format!("Failed to send tx: {}", e));
        EcosystemError::TransactionFailed {
            reason: format!("Failed to send setDAValidatorPair tx: {}", e),
        }
    })?;

    let tx_hash = *pending.tx_hash();

    // Wait for confirmation
    let receipt = pending.get_receipt().await.map_err(|e| {
        spinner.error(format!("Confirmation failed: {}", e));
        EcosystemError::TransactionFailed {
            reason: format!("Failed to confirm setDAValidatorPair tx: {}", e),
        }
    })?;

    if !receipt.status() {
        spinner.error("Transaction reverted");
        return Err(EcosystemError::TransactionFailed {
            reason: format!("Transaction {} reverted", tx_hash),
        });
    }

    spinner.stop(format!(
        "DA validator pair set to calldata mode -> Confirmed in block {} (gas: {})",
        green.apply_to(receipt.block_number.unwrap_or_default()),
        receipt.gas_used
    ));

    Ok(tx_hash)
}

/// Create a signer from a private key.
fn create_signer(key: &SecretString) -> Result<PrivateKeySigner> {
    let key_str = key.expose_secret();

    // Strip 0x prefix if present
    let key_hex = key_str.strip_prefix("0x").unwrap_or(key_str);

    let key_bytes: [u8; 32] = hex::decode(key_hex)
        .map_err(|e| EcosystemError::InvalidConfig(format!("Invalid private key hex: {}", e)))?
        .try_into()
        .map_err(|_| EcosystemError::InvalidConfig("Private key must be 32 bytes".to_string()))?;

    PrivateKeySigner::from_bytes(&key_bytes.into())
        .map_err(|e| EcosystemError::InvalidConfig(format!("Invalid private key: {}", e)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pubdata_source_values() {
        assert_eq!(PubdataSource::EmptyNoDa as u8, 1);
        assert_eq!(PubdataSource::PubdataKeccak256 as u8, 2);
        assert_eq!(PubdataSource::BlobsAndPubdataKeccak256 as u8, 3);
        assert_eq!(PubdataSource::BlobsZksyncOs as u8, 4);
    }

    #[test]
    fn test_build_calldata_not_empty() {
        let diamond_proxy = Address::ZERO;
        let l1_da_validator = Address::ZERO;

        let calldata = build_set_da_validator_pair_multicall_calldata(
            diamond_proxy,
            l1_da_validator,
            PubdataSource::PubdataKeccak256,
        );

        // Calldata should not be empty
        assert!(!calldata.is_empty());
        // Should start with multicall selector (first 4 bytes)
        assert!(calldata.len() >= 4);
    }

    #[test]
    fn test_build_calldata_different_sources() {
        let diamond_proxy = Address::ZERO;
        let l1_da_validator = Address::ZERO;

        let calldata_keccak = build_set_da_validator_pair_multicall_calldata(
            diamond_proxy,
            l1_da_validator,
            PubdataSource::PubdataKeccak256,
        );

        let calldata_blobs = build_set_da_validator_pair_multicall_calldata(
            diamond_proxy,
            l1_da_validator,
            PubdataSource::BlobsZksyncOs,
        );

        // Different pubdata sources should produce different calldata
        assert_ne!(calldata_keccak, calldata_blobs);
    }
}
