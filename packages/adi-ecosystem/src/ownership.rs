//! Ownership acceptance for contracts with pending ownership transfers.
//!
//! This module handles accepting ownership for contracts that use:
//! - Ownable2Step pattern (`acceptOwnership()`)
//! - Multicall pattern (via ChainAdmin)
//!
//! # Contracts Handled
//!
//! After ecosystem deployment, the following contracts may have pending ownership:
//! - Server Notifier (via multicall through chain_admin)
//! - RollupDA Manager (via governance acceptOwner)
//! - Validator Timelock (direct acceptOwnership)
//! - Verifier (direct acceptOwnership)

use crate::error::{EcosystemError, Result};
use adi_types::EcosystemContracts;
use alloy_network::{EthereumWallet, TransactionBuilder};
use alloy_primitives::{Address, Bytes, B256, U256};
use alloy_provider::{Provider, ProviderBuilder};
use alloy_rpc_types::TransactionRequest;
use alloy_signer_local::PrivateKeySigner;
use alloy_sol_types::{sol, SolCall};
use colored::Colorize;
use secrecy::{ExposeSecret, SecretString};

// Define contract interfaces
sol! {
    /// Standard Ownable2Step acceptOwnership function.
    #[allow(missing_docs)]
    function acceptOwnership() external;

    /// ChainAdmin multicall interface.
    #[allow(missing_docs)]
    function multicall(
        (address, uint256, bytes)[] calls,
        bool requireSuccess
    ) external;
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
    fn success(name: &str, tx_hash: B256) -> Self {
        Self {
            name: name.to_string(),
            success: true,
            tx_hash: Some(tx_hash),
            error: None,
        }
    }

    /// Create a failed result.
    fn failure(name: &str, error: String) -> Self {
        Self {
            name: name.to_string(),
            success: false,
            tx_hash: None,
            error: Some(error),
        }
    }

    /// Create a skipped result (contract address not configured).
    fn skipped(name: &str, reason: &str) -> Self {
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

    /// Returns the number of failed acceptances.
    pub fn failed_count(&self) -> usize {
        self.results.iter().filter(|r| !r.success).count()
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

/// Build calldata for acceptOwnership() call.
#[must_use]
pub fn build_accept_ownership_calldata() -> Bytes {
    let call = acceptOwnershipCall {};
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

/// Accept ownership for all pending contracts.
///
/// This function attempts to accept ownership for:
/// - Server Notifier (via multicall)
/// - Validator Timelock (direct)
/// - Verifier (direct)
///
/// Note: RollupDA Manager requires forge script and is handled separately.
///
/// # Arguments
///
/// * `rpc_url` - Settlement layer RPC endpoint URL.
/// * `contracts` - Ecosystem contracts containing addresses.
/// * `governor_key` - Governor private key for signing transactions.
/// * `gas_price_wei` - Optional gas price in wei (estimated if not provided).
///
/// # Returns
///
/// Summary of all ownership acceptance attempts.
pub async fn accept_all_ownership(
    rpc_url: &str,
    contracts: &EcosystemContracts,
    governor_key: &SecretString,
    gas_price_wei: Option<u128>,
) -> OwnershipSummary {
    let mut results = Vec::new();

    // Create signer from governor key
    let signer = match create_signer(governor_key) {
        Ok(s) => s,
        Err(e) => {
            log::error!("Failed to create signer: {}", e);
            results.push(OwnershipResult::failure(
                "all",
                format!("Failed to create signer: {}", e),
            ));
            return OwnershipSummary::new(results);
        }
    };
    let governor_address = signer.address();

    // Create signing provider
    let wallet = EthereumWallet::from(signer);
    let url: url::Url = match rpc_url.parse() {
        Ok(u) => u,
        Err(e) => {
            log::error!("Invalid RPC URL: {}", e);
            results.push(OwnershipResult::failure(
                "all",
                format!("Invalid RPC URL: {}", e),
            ));
            return OwnershipSummary::new(results);
        }
    };
    let provider = ProviderBuilder::new().wallet(wallet).connect_http(url);

    // Get chain ID
    let chain_id = match provider.get_chain_id().await {
        Ok(id) => id,
        Err(e) => {
            log::error!("Failed to get chain ID: {}", e);
            results.push(OwnershipResult::failure(
                "all",
                format!("Failed to get chain ID: {}", e),
            ));
            return OwnershipSummary::new(results);
        }
    };

    // Get initial nonce
    let mut nonce = match provider.get_transaction_count(governor_address).await {
        Ok(n) => n,
        Err(e) => {
            log::error!("Failed to get nonce: {}", e);
            results.push(OwnershipResult::failure(
                "all",
                format!("Failed to get nonce: {}", e),
            ));
            return OwnershipSummary::new(results);
        }
    };

    // Get gas price if not provided
    let gas_price = match gas_price_wei {
        Some(price) => price,
        None => match provider.get_gas_price().await {
            Ok(p) => p,
            Err(e) => {
                log::error!("Failed to get gas price: {}", e);
                results.push(OwnershipResult::failure(
                    "all",
                    format!("Failed to get gas price: {}", e),
                ));
                return OwnershipSummary::new(results);
            }
        },
    };

    // Get chain admin for multicall operations
    let chain_admin = contracts.chain_admin_addr();

    // 1. Server Notifier (via multicall)
    log::info!("{}", "Processing Server Notifier...".cyan());
    let result = accept_server_notifier(
        &provider,
        contracts,
        chain_admin,
        governor_address,
        chain_id,
        &mut nonce,
        gas_price,
    )
    .await;
    results.push(result);

    // 2. Validator Timelock (direct)
    log::info!("{}", "Processing Validator Timelock...".cyan());
    let result = accept_validator_timelock(
        &provider,
        contracts,
        governor_address,
        chain_id,
        &mut nonce,
        gas_price,
    )
    .await;
    results.push(result);

    // 3. Verifier (direct)
    log::info!("{}", "Processing Verifier...".cyan());
    let result = accept_verifier(
        &provider,
        contracts,
        governor_address,
        chain_id,
        &mut nonce,
        gas_price,
    )
    .await;
    results.push(result);

    OwnershipSummary::new(results)
}

/// Accept ownership for Server Notifier via multicall.
async fn accept_server_notifier<P>(
    provider: &P,
    contracts: &EcosystemContracts,
    chain_admin: Option<Address>,
    governor: Address,
    chain_id: u64,
    nonce: &mut u64,
    gas_price: u128,
) -> OwnershipResult
where
    P: Provider + Clone,
{
    let server_notifier = match contracts.server_notifier_addr() {
        Some(addr) => addr,
        None => {
            return OwnershipResult::skipped(
                "Server Notifier",
                "server_notifier_proxy_addr not configured",
            );
        }
    };

    let chain_admin_addr = match chain_admin {
        Some(addr) => addr,
        None => {
            return OwnershipResult::skipped("Server Notifier", "chain_admin_addr not configured");
        }
    };

    let calldata = build_accept_ownership_multicall_calldata(server_notifier);

    match send_ownership_tx(
        provider,
        chain_admin_addr,
        calldata,
        governor,
        chain_id,
        *nonce,
        gas_price,
    )
    .await
    {
        Ok(tx_hash) => {
            log::info!(
                "  {} Server Notifier ownership accepted: {}",
                "✓".green(),
                tx_hash.to_string().green()
            );
            *nonce += 1;
            OwnershipResult::success("Server Notifier", tx_hash)
        }
        Err(e) => {
            log::warn!(
                "  {} Server Notifier ownership failed: {}",
                "✗".yellow(),
                e.to_string().yellow()
            );
            OwnershipResult::failure("Server Notifier", e.to_string())
        }
    }
}

/// Accept ownership for Validator Timelock.
async fn accept_validator_timelock<P>(
    provider: &P,
    contracts: &EcosystemContracts,
    governor: Address,
    chain_id: u64,
    nonce: &mut u64,
    gas_price: u128,
) -> OwnershipResult
where
    P: Provider + Clone,
{
    let timelock = match contracts.validator_timelock_addr() {
        Some(addr) => addr,
        None => {
            return OwnershipResult::skipped(
                "Validator Timelock",
                "validator_timelock_addr not configured",
            );
        }
    };

    let calldata = build_accept_ownership_calldata();

    match send_ownership_tx(
        provider, timelock, calldata, governor, chain_id, *nonce, gas_price,
    )
    .await
    {
        Ok(tx_hash) => {
            log::info!(
                "  {} Validator Timelock ownership accepted: {}",
                "✓".green(),
                tx_hash.to_string().green()
            );
            *nonce += 1;
            OwnershipResult::success("Validator Timelock", tx_hash)
        }
        Err(e) => {
            log::warn!(
                "  {} Validator Timelock ownership failed: {}",
                "✗".yellow(),
                e.to_string().yellow()
            );
            OwnershipResult::failure("Validator Timelock", e.to_string())
        }
    }
}

/// Accept ownership for Verifier.
async fn accept_verifier<P>(
    provider: &P,
    contracts: &EcosystemContracts,
    governor: Address,
    chain_id: u64,
    nonce: &mut u64,
    gas_price: u128,
) -> OwnershipResult
where
    P: Provider + Clone,
{
    let verifier = match contracts.verifier_addr() {
        Some(addr) => addr,
        None => {
            return OwnershipResult::skipped("Verifier", "verifier_addr not configured");
        }
    };

    let calldata = build_accept_ownership_calldata();

    match send_ownership_tx(
        provider, verifier, calldata, governor, chain_id, *nonce, gas_price,
    )
    .await
    {
        Ok(tx_hash) => {
            log::info!(
                "  {} Verifier ownership accepted: {}",
                "✓".green(),
                tx_hash.to_string().green()
            );
            *nonce += 1;
            OwnershipResult::success("Verifier", tx_hash)
        }
        Err(e) => {
            log::warn!(
                "  {} Verifier ownership failed: {}",
                "✗".yellow(),
                e.to_string().yellow()
            );
            OwnershipResult::failure("Verifier", e.to_string())
        }
    }
}

/// Send an ownership acceptance transaction.
async fn send_ownership_tx<P>(
    provider: &P,
    to: Address,
    calldata: Bytes,
    from: Address,
    chain_id: u64,
    nonce: u64,
    gas_price: u128,
) -> Result<B256>
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

    // Wait for confirmation
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
    fn test_build_accept_ownership_calldata() {
        let calldata = build_accept_ownership_calldata();
        // acceptOwnership() selector is 0x79ba5097
        assert!(!calldata.is_empty());
        assert!(calldata.len() >= 4);
    }

    #[test]
    fn test_build_multicall_calldata() {
        let target = Address::ZERO;
        let calldata = build_accept_ownership_multicall_calldata(target);
        // Should contain multicall selector
        assert!(!calldata.is_empty());
        assert!(calldata.len() >= 4);
    }

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
