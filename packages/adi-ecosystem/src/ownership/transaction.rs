//! Transaction helpers for ownership acceptance.
//!
//! This module provides utilities for sending transactions and
//! creating signers from private keys.

use crate::error::{EcosystemError, Result};
use alloy_network::TransactionBuilder;
use alloy_primitives::{Address, Bytes, B256};
use alloy_provider::Provider;
use alloy_rpc_types::TransactionRequest;
use alloy_signer_local::PrivateKeySigner;
use secrecy::{ExposeSecret, SecretString};

/// Transaction result with hash and block number.
pub struct TxResult {
    /// Transaction hash.
    pub tx_hash: B256,
    /// Block number where transaction was confirmed.
    pub block_number: u64,
}

/// Send an ownership transaction and wait for confirmation (no spinner).
pub async fn send_ownership_tx<P>(
    provider: &P,
    to: Address,
    calldata: Bytes,
    from: Address,
    chain_id: u64,
    nonce: u64,
    gas_price: u128,
) -> Result<TxResult>
where
    P: Provider + Clone,
{
    let tx = TransactionRequest::default()
        .with_from(from)
        .with_to(to)
        .with_input(calldata)
        .with_nonce(nonce)
        .with_gas_limit(200_000) // Conservative gas limit for ownership calls
        .with_gas_price(gas_price)
        .with_chain_id(chain_id);

    let pending =
        provider
            .send_transaction(tx)
            .await
            .map_err(|e| EcosystemError::TransactionFailed {
                reason: format!("Failed to send tx: {}", e),
            })?;

    let tx_hash = *pending.tx_hash();

    let receipt = pending
        .get_receipt()
        .await
        .map_err(|e| EcosystemError::TransactionFailed {
            reason: format!("Failed to get receipt: {}", e),
        })?;

    if !receipt.status() {
        return Err(EcosystemError::TransactionFailed {
            reason: format!("Transaction {} reverted", tx_hash),
        });
    }

    Ok(TxResult {
        tx_hash,
        block_number: receipt.block_number.unwrap_or_default(),
    })
}

/// Create a signer from a private key.
pub fn create_signer(key: &SecretString) -> Result<PrivateKeySigner> {
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
