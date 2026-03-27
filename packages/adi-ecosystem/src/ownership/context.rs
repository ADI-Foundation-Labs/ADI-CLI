//! Provider context for ownership operations.
//!
//! Extracts the shared provider/signer/nonce/gas setup boilerplate
//! used by accept, transfer, and collect ownership functions.

use crate::signer::create_signer;
use adi_types::{normalize_rpc_url, Logger};
use alloy_network::EthereumWallet;
use alloy_primitives::Address;
use alloy_provider::{Provider, ProviderBuilder};
use secrecy::SecretString;

use super::types::{OwnershipResult, OwnershipSummary};

/// Shared context for ownership transaction operations.
pub(super) struct SigningContext<P> {
    /// Signing provider.
    pub provider: P,
    /// Governor address derived from the signing key.
    pub governor_address: Address,
    /// Settlement layer chain ID.
    pub chain_id: u64,
    /// Current transaction nonce.
    pub nonce: u64,
    /// Gas price to use for transactions.
    pub gas_price: u128,
}

/// Build a signing context for ownership operations.
///
/// On failure, returns an `OwnershipSummary` with a single failure result
/// so callers can return early without boilerplate error handling.
pub(super) async fn build_signing_context(
    rpc_url: &str,
    governor_key: &SecretString,
    gas_multiplier: Option<u64>,
    logger: &dyn Logger,
) -> Result<SigningContext<impl alloy_provider::Provider + Clone>, OwnershipSummary> {
    let signer = create_signer(governor_key).map_err(|e| {
        logger.error(&format!("Failed to create signer: {}", e));
        OwnershipSummary::new(vec![OwnershipResult::failure(
            "all",
            format!("Failed to create signer: {}", e),
        )])
    })?;
    let governor_address = signer.address();

    let wallet = EthereumWallet::from(signer);
    let normalized_rpc = normalize_rpc_url(rpc_url);
    let url: url::Url = normalized_rpc.parse().map_err(|e| {
        logger.error(&format!("Invalid RPC URL: {}", e));
        OwnershipSummary::new(vec![OwnershipResult::failure(
            "all",
            format!("Invalid RPC URL: {}", e),
        )])
    })?;
    let provider = ProviderBuilder::new().wallet(wallet).connect_http(url);

    let chain_id = provider.get_chain_id().await.map_err(|e| {
        logger.error(&format!("Failed to get chain ID: {}", e));
        OwnershipSummary::new(vec![OwnershipResult::failure(
            "all",
            format!("Failed to get chain ID: {}", e),
        )])
    })?;

    let nonce = provider
        .get_transaction_count(governor_address)
        .await
        .map_err(|e| {
            logger.error(&format!("Failed to get nonce: {}", e));
            OwnershipSummary::new(vec![OwnershipResult::failure(
                "all",
                format!("Failed to get nonce: {}", e),
            )])
        })?;

    let estimated = provider.get_gas_price().await.map_err(|e| {
        logger.error(&format!("Failed to get gas price: {}", e));
        OwnershipSummary::new(vec![OwnershipResult::failure(
            "all",
            format!("Failed to get gas price: {}", e),
        )])
    })?;
    let gas_price = gas_multiplier.map_or(estimated, |m| estimated * u128::from(m) / 100);

    Ok(SigningContext {
        provider,
        governor_address,
        chain_id,
        nonce,
        gas_price,
    })
}
