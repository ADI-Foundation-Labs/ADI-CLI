//! Data Availability (DA) configuration for ZkSync chains.
//!
//! This module provides functions for configuring DA mode on chains,
//! specifically for L3 deployments that need calldata-based pubdata
//! instead of EIP-4844 blobs.

use crate::error::{EcosystemError, Result};
use crate::signer::create_signer;
use adi_types::{normalize_rpc_url, Logger};
use alloy_network::{EthereumWallet, TransactionBuilder};
use alloy_primitives::{Address, Bytes, B256, U256};
use alloy_provider::{Provider, ProviderBuilder};
use alloy_rpc_types::TransactionRequest;
use alloy_sol_types::{sol, SolCall};
use console::Style;
use secrecy::SecretString;
use tokio::time::{timeout, Duration};

use adi_types::TX_TIMEOUT_SECONDS;

// Define the contract interfaces using alloy's sol! macro
sol! {
    /// Set DA validator pair on Diamond proxy.
    /// Called through ChainAdmin multicall.
    ///
    /// The second argument is the chain's `L2DACommitmentScheme` (an on-chain
    /// per-chain setting), NOT the per-batch transport flag. Encoded as `uint8`.
    #[allow(missing_docs)]
    function setDAValidatorPair(address l1DAValidator, uint8 l2DACommitmentScheme) external;

    /// ChainAdmin multicall interface.
    #[allow(missing_docs)]
    function multicall(
        (address, uint256, bytes)[] calls,
        bool requireSuccess
    ) external;
}

/// On-chain L2 DA commitment scheme, stored per-chain via `setDAValidatorPair`.
///
/// These values correspond to the `L2DACommitmentScheme` enum in the ZKsync
/// contracts (`common/Config.sol`). This is the commitment *scheme* the chain
/// expects — distinct from the per-batch transport flag (`PubdataSource {
/// Calldata=0, Blob=1 }` in `DAUtils.sol`) that the server prefixes onto each
/// batch's operator DA input. In particular, posting pubdata as **calldata** is
/// still `BlobsAndPubdataKeccak256` (3): the rollup `CalldataDA` validator uses
/// scheme 3 for both blob and calldata transports; the calldata-vs-blob choice
/// is the transport flag, not the scheme. `PubdataKeccak256` (2) is reserved for
/// custom/external DA and is not used by the standard rollup/gateway validators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum L2DACommitmentScheme {
    /// No DA — Validium mode.
    EmptyNoDa = 1,
    /// Keccak of pubdata only — for custom/external DA (not on-chain pubdata).
    PubdataKeccak256 = 2,
    /// Rollup scheme for on-chain pubdata via blobs **or** calldata.
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
/// * `commitment_scheme` - The chain's L2 DA commitment scheme.
///
/// # Returns
///
/// ABI-encoded calldata for the multicall transaction.
#[must_use]
pub fn build_set_da_validator_pair_multicall_calldata(
    diamond_proxy: Address,
    l1_da_validator: Address,
    commitment_scheme: L2DACommitmentScheme,
) -> Bytes {
    // Build inner call to setDAValidatorPair
    let inner_call = setDAValidatorPairCall {
        l1DAValidator: l1_da_validator,
        l2DACommitmentScheme: commitment_scheme as u8,
    };
    let inner_calldata = Bytes::from(inner_call.abi_encode());

    // Build outer multicall: [(diamond_proxy, 0, calldata)]
    let multicall_call = multicallCall {
        calls: vec![(diamond_proxy, U256::ZERO, inner_calldata)],
        requireSuccess: true,
    };

    Bytes::from(multicall_call.abi_encode())
}

/// Arguments for setting a chain's DA validator pair.
pub struct DaValidatorPairConfig<'a> {
    /// Settlement layer RPC endpoint URL.
    pub rpc_url: &'a str,
    /// ChainAdmin contract address.
    pub chain_admin: Address,
    /// Diamond proxy contract address.
    pub diamond_proxy: Address,
    /// L1 DA validator contract address.
    pub l1_da_validator: Address,
    /// The chain's L2 DA commitment scheme.
    pub commitment_scheme: L2DACommitmentScheme,
    /// Governor private key for signing transactions.
    pub governor_key: &'a SecretString,
    /// Gas price multiplier percentage.
    pub gas_multiplier: Option<u64>,
}

/// Set a chain's DA validator pair (`setDAValidatorPair`) via ChainAdmin.
///
/// Pairs the given L1 DA validator with the chain's L2 DA commitment scheme.
/// Used for non-default DA modes: calldata (rollup, blobs/calldata transport)
/// and Validium (no DA). Works for chains settling on L1 (L2) or on a gateway
/// (L3) — the commitment scheme is the same; only the validator address differs.
///
/// # Arguments
///
/// * `config` - DA configuration including RPC, addresses, and keys.
/// * `logger` - Logger for debug/info/warning output.
///
/// # Returns
///
/// Transaction hash on success.
///
/// # Errors
///
/// Returns error if transaction fails or required addresses are invalid.
pub async fn configure_da_validator_pair(
    config: DaValidatorPairConfig<'_>,
    logger: &dyn Logger,
) -> Result<B256> {
    logger.debug(&format!(
        "Setting DA validator pair via chain_admin: {}",
        config.chain_admin
    ));

    // Create signer from governor key
    let signer = create_signer(config.governor_key)?;
    let governor_address = signer.address();
    logger.debug(&format!("Governor address: {}", governor_address));

    // Create signing provider
    let wallet = EthereumWallet::from(signer);
    let normalized_rpc = normalize_rpc_url(config.rpc_url);
    let url: url::Url = normalized_rpc.parse().map_err(|e| {
        EcosystemError::InvalidConfig(format!("Invalid RPC URL '{}': {}", config.rpc_url, e))
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
    let gas_price = config
        .gas_multiplier
        .map_or(estimated, |m| estimated * u128::from(m) / 100);
    logger.debug(&format!("Using gas price: {} wei", gas_price));

    // Build calldata for setDAValidatorPair via multicall
    let calldata = build_set_da_validator_pair_multicall_calldata(
        config.diamond_proxy,
        config.l1_da_validator,
        config.commitment_scheme,
    );

    let green = Style::new().green();
    let mode_name = match config.commitment_scheme {
        L2DACommitmentScheme::EmptyNoDa => "Validium mode (no DA)",
        L2DACommitmentScheme::PubdataKeccak256 => "external DA mode (pubdata keccak256)",
        L2DACommitmentScheme::BlobsAndPubdataKeccak256 => "rollup mode (blobs/calldata pubdata)",
        L2DACommitmentScheme::BlobsZksyncOs => "ZKsync OS blobs mode",
    };

    let spinner = cliclack::spinner();
    spinner.start(format!(
        "Setting DA validator pair to {} ({})",
        mode_name,
        green.apply_to(config.l1_da_validator)
    ));

    // Build transaction to chain_admin
    let tx = TransactionRequest::default()
        .with_from(governor_address)
        .with_to(config.chain_admin)
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

    // Wait for confirmation with timeout
    let receipt = timeout(Duration::from_secs(TX_TIMEOUT_SECONDS), pending.get_receipt())
        .await
        .map_err(|_| {
            spinner.error("Transaction not mined within timeout window");
            EcosystemError::TransactionFailed {
                reason: "Transaction not mined within timeout window: setDAValidatorPair".to_string(),
            }
        })?
        .map_err(|e| {
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
        "DA validator pair set to {} -> Confirmed in block {} (gas: {})",
        mode_name,
        green.apply_to(receipt.block_number.unwrap_or_default()),
        receipt.gas_used
    ));

    Ok(tx_hash)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_commitment_scheme_values() {
        assert_eq!(L2DACommitmentScheme::EmptyNoDa as u8, 1);
        assert_eq!(L2DACommitmentScheme::PubdataKeccak256 as u8, 2);
        assert_eq!(L2DACommitmentScheme::BlobsAndPubdataKeccak256 as u8, 3);
        assert_eq!(L2DACommitmentScheme::BlobsZksyncOs as u8, 4);
    }

    #[test]
    fn test_build_calldata_not_empty() {
        let diamond_proxy = Address::ZERO;
        let l1_da_validator = Address::ZERO;

        let calldata = build_set_da_validator_pair_multicall_calldata(
            diamond_proxy,
            l1_da_validator,
            L2DACommitmentScheme::BlobsAndPubdataKeccak256,
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

        let calldata_rollup = build_set_da_validator_pair_multicall_calldata(
            diamond_proxy,
            l1_da_validator,
            L2DACommitmentScheme::BlobsAndPubdataKeccak256,
        );

        let calldata_blobs = build_set_da_validator_pair_multicall_calldata(
            diamond_proxy,
            l1_da_validator,
            L2DACommitmentScheme::BlobsZksyncOs,
        );

        // Different commitment schemes should produce different calldata
        assert_ne!(calldata_rollup, calldata_blobs);
    }

    #[test]
    fn test_build_calldata_validium() {
        let diamond_proxy = Address::ZERO;
        let l1_da_validator = Address::ZERO;

        let calldata = build_set_da_validator_pair_multicall_calldata(
            diamond_proxy,
            l1_da_validator,
            L2DACommitmentScheme::EmptyNoDa,
        );

        // Validium mode (1) should be present in calldata
        // The last byte of the inner call is the commitment scheme
        assert!(calldata.iter().any(|&b| b == 1));
    }
}
