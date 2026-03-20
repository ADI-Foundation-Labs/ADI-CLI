//! Governance transaction execution.
//!
//! Sends scheduleTransparent and execute transactions to the governance contract.

use alloy_network::EthereumWallet;
use alloy_primitives::{Address, Bytes, B256, U256};
use alloy_provider::{Provider, ProviderBuilder};
use alloy_rpc_types::TransactionRequest;
use alloy_signer_local::PrivateKeySigner;
use secrecy::ExposeSecret;

use crate::error::{Result, UpgradeError};

/// Result of governance execution.
#[derive(Debug)]
pub struct GovernanceResult {
    /// Transaction hash of scheduleTransparent call.
    pub schedule_tx_hash: B256,
    /// Transaction hash of execute call.
    pub execute_tx_hash: B256,
}

/// Execute governance transactions (scheduleTransparent + execute).
///
/// Creates a signing provider from the governor's private key and sends
/// both transactions sequentially, waiting for confirmation.
pub async fn execute_governance<P: Provider + Clone>(
    provider: &P,
    governor_key: &secrecy::SecretString,
    governance_contract: Address,
    schedule_calldata: Bytes,
    execute_calldata: Bytes,
) -> Result<GovernanceResult> {
    log::info!(
        "Executing governance transactions on {}",
        governance_contract
    );

    // Create signer from governor key
    let key_str = governor_key.expose_secret();
    let key_hex = key_str.strip_prefix("0x").unwrap_or(key_str);
    let key_bytes: [u8; 32] = hex::decode(key_hex)
        .map_err(|e| UpgradeError::GovernanceFailed(format!("Invalid governor key hex: {e}")))?
        .try_into()
        .map_err(|_| UpgradeError::GovernanceFailed("Governor key must be 32 bytes".into()))?;

    let signer = PrivateKeySigner::from_bytes(&key_bytes.into())
        .map_err(|e| UpgradeError::GovernanceFailed(format!("Invalid governor key: {e}")))?;

    let wallet = EthereumWallet::from(signer);

    // Create signing provider by wrapping the existing provider
    let signing_provider = ProviderBuilder::new()
        .wallet(wallet)
        .connect_provider(provider.clone());

    // Send scheduleTransparent
    log::info!("Sending scheduleTransparent transaction...");
    let schedule_tx = TransactionRequest::default()
        .to(governance_contract)
        .input(schedule_calldata.into())
        .value(U256::ZERO);

    let schedule_pending = signing_provider
        .send_transaction(schedule_tx)
        .await
        .map_err(|e| {
            UpgradeError::GovernanceFailed(format!("scheduleTransparent tx failed: {e}"))
        })?;

    let schedule_receipt = schedule_pending.get_receipt().await.map_err(|e| {
        UpgradeError::GovernanceFailed(format!("scheduleTransparent receipt failed: {e}"))
    })?;

    let schedule_tx_hash = schedule_receipt.transaction_hash;
    log::info!("scheduleTransparent tx: {}", schedule_tx_hash);

    // Send execute
    log::info!("Sending execute transaction...");
    let execute_tx = TransactionRequest::default()
        .to(governance_contract)
        .input(execute_calldata.into())
        .value(U256::ZERO);

    let execute_pending = signing_provider
        .send_transaction(execute_tx)
        .await
        .map_err(|e| UpgradeError::GovernanceFailed(format!("execute tx failed: {e}")))?;

    let execute_receipt = execute_pending
        .get_receipt()
        .await
        .map_err(|e| UpgradeError::GovernanceFailed(format!("execute receipt failed: {e}")))?;

    let execute_tx_hash = execute_receipt.transaction_hash;
    log::info!("execute tx: {}", execute_tx_hash);

    Ok(GovernanceResult {
        schedule_tx_hash,
        execute_tx_hash,
    })
}
