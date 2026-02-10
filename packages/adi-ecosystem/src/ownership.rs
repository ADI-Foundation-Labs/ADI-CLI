//! Ownership acceptance for contracts with pending ownership transfers.
//!
//! This module handles accepting ownership for contracts that use:
//! - Ownable2Step pattern (`acceptOwnership()`)
//! - Multicall pattern (via ChainAdmin)
//! - Governance pattern (via scheduleTransparent + execute)
//!
//! # Contracts Handled
//!
//! After ecosystem deployment, the following contracts may have pending ownership:
//! - Server Notifier (via multicall through chain_admin)
//! - Validator Timelock (direct acceptOwnership)
//! - Verifier (direct acceptOwnership)
//! - Governance (direct acceptOwnership)
//! - RollupDA Manager (via Governance scheduleTransparent + execute)

use crate::error::{EcosystemError, Result};
use adi_types::{ChainContracts, EcosystemContracts};
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

    /// Read pending owner for Ownable2Step contracts.
    #[allow(missing_docs)]
    function pendingOwner() external view returns (address);

    /// Read current owner.
    #[allow(missing_docs)]
    function owner() external view returns (address);

    /// ChainAdmin multicall interface.
    #[allow(missing_docs)]
    function multicall(
        (address, uint256, bytes)[] calls,
        bool requireSuccess
    ) external;

    /// Governance Call struct for operations.
    #[allow(missing_docs)]
    struct Call {
        address target;
        uint256 value;
        bytes data;
    }

    /// Governance Operation struct.
    #[allow(missing_docs)]
    struct Operation {
        Call[] calls;
        bytes32 predecessor;
        bytes32 salt;
    }

    /// Governance scheduleTransparent function.
    #[allow(missing_docs)]
    function scheduleTransparent(Operation operation, uint256 delay) external;

    /// Governance execute function.
    #[allow(missing_docs)]
    function execute(Operation operation) external payable;

    /// Governance minDelay getter.
    #[allow(missing_docs)]
    function minDelay() external view returns (uint256);
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

/// Ownership state for a contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OwnershipState {
    /// Ownership transfer pending - needs acceptOwnership().
    Pending,
    /// Ownership already accepted - owner is governor.
    Accepted,
    /// Ownership transfer not initiated - owner is not governor, no pending owner.
    NotTransferred,
}

/// Status of a contract's ownership.
#[derive(Debug, Clone)]
pub struct OwnershipStatus {
    /// Contract name.
    pub name: &'static str,
    /// Contract address (None if not configured).
    pub address: Option<Address>,
    /// Current ownership state.
    pub state: OwnershipState,
}

/// Summary of ownership status check.
#[derive(Debug)]
pub struct OwnershipStatusSummary {
    /// Status for each contract.
    pub statuses: Vec<OwnershipStatus>,
}

impl OwnershipStatusSummary {
    /// Returns the number of contracts with pending ownership.
    pub fn pending_count(&self) -> usize {
        self.statuses
            .iter()
            .filter(|s| s.state == OwnershipState::Pending)
            .count()
    }

    /// Returns the number of contracts not configured.
    pub fn not_configured_count(&self) -> usize {
        self.statuses.iter().filter(|s| s.address.is_none()).count()
    }

    /// Returns the number of contracts already accepted.
    pub fn already_accepted_count(&self) -> usize {
        self.statuses
            .iter()
            .filter(|s| s.state == OwnershipState::Accepted)
            .count()
    }

    /// Returns the number of contracts where ownership was not transferred.
    pub fn not_transferred_count(&self) -> usize {
        self.statuses
            .iter()
            .filter(|s| s.state == OwnershipState::NotTransferred)
            .count()
    }
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

    /// Returns the number of skipped contracts.
    pub fn skipped_count(&self) -> usize {
        self.results
            .iter()
            .filter(|r| {
                !r.success
                    && r.error
                        .as_ref()
                        .is_some_and(|e| e.starts_with("Skipped:"))
            })
            .count()
    }

    /// Returns the number of failed acceptances (excludes skipped).
    pub fn failed_count(&self) -> usize {
        self.results
            .iter()
            .filter(|r| {
                !r.success
                    && r.error
                        .as_ref()
                        .is_none_or(|e| !e.starts_with("Skipped:"))
            })
            .count()
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

/// Check ownership status for all ecosystem contracts.
///
/// This function queries `pendingOwner()` on each contract to determine
/// which ones have pending ownership transfers that need to be accepted.
///
/// # Arguments
///
/// * `rpc_url` - Settlement layer RPC endpoint URL.
/// * `contracts` - Ecosystem contracts containing addresses.
/// * `governor_address` - Governor address to check as pending owner.
///
/// # Returns
///
/// Summary of ownership status for all contracts.
pub async fn check_ecosystem_ownership_status(
    rpc_url: &str,
    contracts: &EcosystemContracts,
    governor_address: Address,
) -> Result<OwnershipStatusSummary> {
    let url: url::Url = rpc_url
        .parse()
        .map_err(|e| EcosystemError::InvalidConfig(format!("Invalid RPC URL: {}", e)))?;
    let provider = ProviderBuilder::new().connect_http(url);

    let mut statuses = Vec::new();

    // Get chain_admin for contracts that are owned by it
    let chain_admin = contracts.chain_admin_addr();

    // Check Server Notifier (owned by ChainAdmin, not governor)
    let server_notifier_addr = contracts.server_notifier_addr();
    let state = match (server_notifier_addr, chain_admin) {
        (Some(addr), Some(ca)) => check_ownership_state(&provider, addr, ca).await,
        _ => OwnershipState::NotTransferred,
    };
    statuses.push(OwnershipStatus {
        name: "Server Notifier",
        address: server_notifier_addr,
        state,
    });

    // Check Validator Timelock
    let timelock_addr = contracts.validator_timelock_addr();
    let state = if let Some(addr) = timelock_addr {
        check_ownership_state(&provider, addr, governor_address).await
    } else {
        OwnershipState::NotTransferred
    };
    statuses.push(OwnershipStatus {
        name: "Validator Timelock",
        address: timelock_addr,
        state,
    });

    // Check Verifier
    let verifier_addr = contracts.verifier_addr();
    let state = if let Some(addr) = verifier_addr {
        check_ownership_state(&provider, addr, governor_address).await
    } else {
        OwnershipState::NotTransferred
    };
    statuses.push(OwnershipStatus {
        name: "Verifier",
        address: verifier_addr,
        state,
    });

    // Check Governance
    let governance_addr = contracts.governance_addr();
    let state = if let Some(addr) = governance_addr {
        check_ownership_state(&provider, addr, governor_address).await
    } else {
        OwnershipState::NotTransferred
    };
    statuses.push(OwnershipStatus {
        name: "Governance",
        address: governance_addr,
        state,
    });

    // Check Rollup DA Manager (pending owner should be governance, not governor)
    let da_manager_addr = contracts.l1_rollup_da_manager_addr();
    let state = if let (Some(da_addr), Some(gov_addr)) = (da_manager_addr, governance_addr) {
        check_ownership_state(&provider, da_addr, gov_addr).await
    } else {
        OwnershipState::NotTransferred
    };
    statuses.push(OwnershipStatus {
        name: "Rollup DA Manager",
        address: da_manager_addr,
        state,
    });

    Ok(OwnershipStatusSummary { statuses })
}

/// Check ownership status for chain contracts.
///
/// # Arguments
///
/// * `rpc_url` - Settlement layer RPC endpoint URL.
/// * `contracts` - Chain contracts containing addresses.
/// * `governor_address` - Governor address to check as pending owner.
///
/// # Returns
///
/// Summary of ownership status for chain contracts.
pub async fn check_chain_ownership_status(
    rpc_url: &str,
    contracts: &ChainContracts,
    governor_address: Address,
) -> Result<OwnershipStatusSummary> {
    let url: url::Url = rpc_url
        .parse()
        .map_err(|e| EcosystemError::InvalidConfig(format!("Invalid RPC URL: {}", e)))?;
    let provider = ProviderBuilder::new().connect_http(url);

    let mut statuses = Vec::new();

    // Check Chain Admin
    let chain_admin_addr = contracts.chain_admin_addr();
    let state = if let Some(addr) = chain_admin_addr {
        check_ownership_state(&provider, addr, governor_address).await
    } else {
        OwnershipState::NotTransferred
    };
    statuses.push(OwnershipStatus {
        name: "Chain Admin",
        address: chain_admin_addr,
        state,
    });

    Ok(OwnershipStatusSummary { statuses })
}

/// Call pendingOwner() on a contract and return the result.
async fn call_pending_owner<P>(provider: &P, contract_address: Address) -> Option<Address>
where
    P: Provider + Clone,
{
    let calldata = pendingOwnerCall {}.abi_encode();
    let tx = alloy_rpc_types::TransactionRequest::default()
        .to(contract_address)
        .input(calldata.into());

    match provider.call(tx).await {
        Ok(result) => result.get(12..32).map(Address::from_slice),
        Err(e) => {
            log::debug!(
                "Failed to call pendingOwner for {}: {}",
                contract_address,
                e
            );
            None
        }
    }
}

/// Call owner() on a contract and return the result.
async fn call_owner<P>(provider: &P, contract_address: Address) -> Option<Address>
where
    P: Provider + Clone,
{
    let calldata = ownerCall {}.abi_encode();
    let tx = alloy_rpc_types::TransactionRequest::default()
        .to(contract_address)
        .input(calldata.into());

    match provider.call(tx).await {
        Ok(result) => result.get(12..32).map(Address::from_slice),
        Err(e) => {
            log::debug!("Failed to call owner for {}: {}", contract_address, e);
            None
        }
    }
}

/// Check the ownership state of a contract.
///
/// Returns:
/// - `Pending` if governor is the pending owner (needs acceptOwnership)
/// - `Accepted` if governor is already the owner
/// - `NotTransferred` if ownership was never transferred to governor
async fn check_ownership_state<P>(
    provider: &P,
    contract_address: Address,
    governor_address: Address,
) -> OwnershipState
where
    P: Provider + Clone,
{
    // Check if governor is pending owner
    let pending_owner = call_pending_owner(provider, contract_address).await;
    if pending_owner == Some(governor_address) {
        return OwnershipState::Pending;
    }

    // Check if governor is already owner
    let current_owner = call_owner(provider, contract_address).await;
    if current_owner == Some(governor_address) {
        return OwnershipState::Accepted;
    }

    // Neither pending nor owner - ownership transfer not initiated
    OwnershipState::NotTransferred
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

    // 4. Governance (direct)
    log::info!("{}", "Processing Governance...".cyan());
    let result = accept_governance(
        &provider,
        contracts,
        governor_address,
        chain_id,
        &mut nonce,
        gas_price,
    )
    .await;
    results.push(result);

    // 5. Rollup DA Manager (via governance acceptOwner)
    log::info!("{}", "Processing Rollup DA Manager...".cyan());
    let result = accept_rollup_da_manager(
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

/// Accept ownership for chain-level contracts.
///
/// This function attempts to accept ownership for chain-specific contracts:
/// - Chain Admin (direct acceptOwnership)
///
/// # Arguments
///
/// * `rpc_url` - Settlement layer RPC endpoint URL.
/// * `contracts` - Chain contracts containing addresses.
/// * `governor_key` - Governor private key for signing transactions.
/// * `gas_price_wei` - Optional gas price in wei (estimated if not provided).
///
/// # Returns
///
/// Summary of all ownership acceptance attempts.
pub async fn accept_chain_ownership(
    rpc_url: &str,
    contracts: &ChainContracts,
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

    // 1. Chain Admin (direct)
    log::info!("{}", "Processing Chain Admin...".cyan());
    let result = accept_chain_admin(
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

/// Accept ownership for Chain Admin contract.
async fn accept_chain_admin<P>(
    provider: &P,
    contracts: &ChainContracts,
    governor: Address,
    chain_id: u64,
    nonce: &mut u64,
    gas_price: u128,
) -> OwnershipResult
where
    P: Provider + Clone,
{
    let chain_admin = match contracts.chain_admin_addr() {
        Some(addr) => addr,
        None => {
            return OwnershipResult::skipped("Chain Admin", "chain_admin_addr not configured");
        }
    };

    // Check if ownership acceptance is needed
    match check_ownership_state(provider, chain_admin, governor).await {
        OwnershipState::Accepted => {
            log::info!(
                "  {} Chain Admin: {}",
                "✓".cyan(),
                "ownership already accepted".cyan()
            );
            return OwnershipResult::skipped("Chain Admin", "ownership already accepted");
        }
        OwnershipState::NotTransferred => {
            log::info!(
                "  {} Chain Admin: {}",
                "⚠".yellow(),
                "ownership not transferred".yellow()
            );
            return OwnershipResult::skipped("Chain Admin", "ownership not transferred");
        }
        OwnershipState::Pending => {}
    }

    let calldata = build_accept_ownership_calldata();

    match send_ownership_tx(
        provider, chain_admin, calldata, governor, chain_id, *nonce, gas_price,
    )
    .await
    {
        Ok(tx_hash) => {
            log::info!(
                "  {} Chain Admin ownership accepted: {}",
                "✓".green(),
                tx_hash.to_string().green()
            );
            *nonce += 1;
            OwnershipResult::success("Chain Admin", tx_hash)
        }
        Err(e) => {
            log::warn!(
                "  {} Chain Admin ownership failed: {}",
                "✗".yellow(),
                e.to_string().yellow()
            );
            OwnershipResult::failure("Chain Admin", e.to_string())
        }
    }
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

    // Check if ownership acceptance is needed
    // Note: Server Notifier is owned by ChainAdmin, not governor
    match check_ownership_state(provider, server_notifier, chain_admin_addr).await {
        OwnershipState::Accepted => {
            log::info!(
                "  {} Server Notifier: {}",
                "✓".cyan(),
                "ownership already accepted".cyan()
            );
            return OwnershipResult::skipped("Server Notifier", "ownership already accepted");
        }
        OwnershipState::NotTransferred => {
            log::info!(
                "  {} Server Notifier: {}",
                "⚠".yellow(),
                "ownership not transferred".yellow()
            );
            return OwnershipResult::skipped("Server Notifier", "ownership not transferred");
        }
        OwnershipState::Pending => {}
    }

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

    // Check if ownership acceptance is needed
    match check_ownership_state(provider, timelock, governor).await {
        OwnershipState::Accepted => {
            log::info!(
                "  {} Validator Timelock: {}",
                "✓".cyan(),
                "ownership already accepted".cyan()
            );
            return OwnershipResult::skipped("Validator Timelock", "ownership already accepted");
        }
        OwnershipState::NotTransferred => {
            log::info!(
                "  {} Validator Timelock: {}",
                "⚠".yellow(),
                "ownership not transferred".yellow()
            );
            return OwnershipResult::skipped("Validator Timelock", "ownership not transferred");
        }
        OwnershipState::Pending => {}
    }

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

    // Check if ownership acceptance is needed
    match check_ownership_state(provider, verifier, governor).await {
        OwnershipState::Accepted => {
            log::info!(
                "  {} Verifier: {}",
                "✓".cyan(),
                "ownership already accepted".cyan()
            );
            return OwnershipResult::skipped("Verifier", "ownership already accepted");
        }
        OwnershipState::NotTransferred => {
            log::info!(
                "  {} Verifier: {}",
                "⚠".yellow(),
                "ownership not transferred".yellow()
            );
            return OwnershipResult::skipped("Verifier", "ownership not transferred");
        }
        OwnershipState::Pending => {}
    }

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

/// Accept ownership for Governance contract.
async fn accept_governance<P>(
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
    let governance = match contracts.governance_addr() {
        Some(addr) => addr,
        None => {
            return OwnershipResult::skipped("Governance", "governance_addr not configured");
        }
    };

    // Check if ownership acceptance is needed
    match check_ownership_state(provider, governance, governor).await {
        OwnershipState::Accepted => {
            log::info!(
                "  {} Governance: {}",
                "✓".cyan(),
                "ownership already accepted".cyan()
            );
            return OwnershipResult::skipped("Governance", "ownership already accepted");
        }
        OwnershipState::NotTransferred => {
            log::info!(
                "  {} Governance: {}",
                "⚠".yellow(),
                "ownership not transferred".yellow()
            );
            return OwnershipResult::skipped("Governance", "ownership not transferred");
        }
        OwnershipState::Pending => {}
    }

    let calldata = build_accept_ownership_calldata();

    match send_ownership_tx(
        provider, governance, calldata, governor, chain_id, *nonce, gas_price,
    )
    .await
    {
        Ok(tx_hash) => {
            log::info!(
                "  {} Governance ownership accepted: {}",
                "✓".green(),
                tx_hash.to_string().green()
            );
            *nonce += 1;
            OwnershipResult::success("Governance", tx_hash)
        }
        Err(e) => {
            log::warn!(
                "  {} Governance ownership failed: {}",
                "✗".yellow(),
                e.to_string().yellow()
            );
            OwnershipResult::failure("Governance", e.to_string())
        }
    }
}

/// Accept ownership for Rollup DA Manager via Governance.
///
/// This uses the Governance timelock pattern:
/// 1. Call scheduleTransparent(operation, 0) to schedule the acceptOwnership call
/// 2. Call execute(operation) to execute the scheduled operation
///
/// The operation contains a Call to the DA Manager's acceptOwnership() function.
async fn accept_rollup_da_manager<P>(
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
    let da_manager = match contracts.l1_rollup_da_manager_addr() {
        Some(addr) => addr,
        None => {
            return OwnershipResult::skipped(
                "Rollup DA Manager",
                "l1_rollup_da_manager not configured",
            );
        }
    };

    let governance = match contracts.governance_addr() {
        Some(addr) => addr,
        None => {
            return OwnershipResult::skipped(
                "Rollup DA Manager",
                "governance_addr not configured (required for Governance timelock)",
            );
        }
    };

    // Check if ownership acceptance is needed by checking pendingOwner on DA Manager
    // Note: for DA Manager, the expected pending owner is the governance contract
    match check_ownership_state(provider, da_manager, governance).await {
        OwnershipState::Accepted => {
            log::info!(
                "  {} Rollup DA Manager: {}",
                "✓".cyan(),
                "ownership already accepted".cyan()
            );
            return OwnershipResult::skipped("Rollup DA Manager", "ownership already accepted");
        }
        OwnershipState::NotTransferred => {
            log::info!(
                "  {} Rollup DA Manager: {}",
                "⚠".yellow(),
                "ownership not transferred".yellow()
            );
            return OwnershipResult::skipped("Rollup DA Manager", "ownership not transferred");
        }
        OwnershipState::Pending => {}
    }

    // Generate a unique salt for this operation (using current nonce as entropy)
    let salt = B256::from(U256::from(*nonce));

    // Step 1: Schedule the operation via Governance
    log::info!("    Scheduling operation via Governance timelock...");
    let schedule_calldata = build_governance_schedule_calldata(da_manager, salt);

    match send_ownership_tx(
        provider,
        governance,
        schedule_calldata,
        governor,
        chain_id,
        *nonce,
        gas_price,
    )
    .await
    {
        Ok(tx_hash) => {
            log::info!(
                "    {} Scheduled: {}",
                "✓".green(),
                tx_hash.to_string().green()
            );
            *nonce += 1;
        }
        Err(e) => {
            log::warn!(
                "  {} Rollup DA Manager schedule failed: {}",
                "✗".yellow(),
                e.to_string().yellow()
            );
            return OwnershipResult::failure(
                "Rollup DA Manager",
                format!("Schedule failed: {}", e),
            );
        }
    }

    // Step 2: Execute the scheduled operation
    log::info!("    Executing scheduled operation...");
    let execute_calldata = build_governance_execute_calldata(da_manager, salt);

    match send_ownership_tx(
        provider,
        governance,
        execute_calldata,
        governor,
        chain_id,
        *nonce,
        gas_price,
    )
    .await
    {
        Ok(tx_hash) => {
            log::info!(
                "  {} Rollup DA Manager ownership accepted: {}",
                "✓".green(),
                tx_hash.to_string().green()
            );
            *nonce += 1;
            OwnershipResult::success("Rollup DA Manager", tx_hash)
        }
        Err(e) => {
            log::warn!(
                "  {} Rollup DA Manager execute failed: {}",
                "✗".yellow(),
                e.to_string().yellow()
            );
            OwnershipResult::failure("Rollup DA Manager", format!("Execute failed: {}", e))
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
