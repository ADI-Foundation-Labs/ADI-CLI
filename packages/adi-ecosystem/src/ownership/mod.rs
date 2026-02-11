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

mod calldata;
mod contracts;
mod status;
mod transaction;
mod transfer;
mod types;

// Re-export public types
pub use calldata::{
    build_accept_ownership_calldata, build_accept_ownership_multicall_calldata,
    build_governance_execute_calldata, build_governance_schedule_calldata,
    build_transfer_ownership_calldata,
};
pub use status::{check_chain_ownership_status, check_ecosystem_ownership_status};
pub use types::{
    OwnershipContract, OwnershipMethod, OwnershipResult, OwnershipState, OwnershipStatus,
    OwnershipStatusSummary, OwnershipSummary,
};

use adi_types::{ChainContracts, EcosystemContracts};
use alloy_network::EthereumWallet;
use alloy_primitives::Address;
use alloy_provider::{Provider, ProviderBuilder};
use colored::Colorize;
use secrecy::SecretString;

use contracts::{
    accept_chain_admin, accept_governance, accept_rollup_da_manager, accept_server_notifier,
    accept_validator_timelock, accept_verifier,
};
use transaction::create_signer;
use transfer::{
    transfer_bridged_token_beacon, transfer_chain_chain_admin, transfer_chain_governance,
    transfer_ecosystem_chain_admin, transfer_governance, transfer_validator_timelock,
};

/// Accept ownership for all pending contracts.
///
/// This function attempts to accept ownership for:
/// - Server Notifier (via multicall)
/// - Validator Timelock (direct)
/// - Verifier (direct)
/// - Governance (direct)
/// - Rollup DA Manager (via governance timelock)
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

/// Transfer ownership for all ecosystem contracts to a new owner.
///
/// This function transfers ownership for:
/// - Governance (Ownable2Step)
/// - Ecosystem Chain Admin (Ownable2Step)
/// - Bridged Token Beacon (Ownable - immediate transfer)
/// - Validator Timelock (Ownable2Step)
///
/// Note: For Ownable2Step contracts, the new owner must call acceptOwnership()
/// to complete the transfer. The Bridged Token Beacon uses standard Ownable,
/// so ownership transfers immediately.
///
/// # Arguments
///
/// * `rpc_url` - Settlement layer RPC endpoint URL.
/// * `contracts` - Ecosystem contracts containing addresses.
/// * `governor_key` - Governor private key for signing transactions.
/// * `new_owner` - Address to transfer ownership to.
/// * `gas_price_wei` - Optional gas price in wei (estimated if not provided).
///
/// # Returns
///
/// Summary of all ownership transfer attempts.
pub async fn transfer_all_ownership(
    rpc_url: &str,
    contracts: &EcosystemContracts,
    governor_key: &SecretString,
    new_owner: Address,
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

    log::info!(
        "Transferring ownership to: {}",
        new_owner.to_string().green()
    );

    // 1. Transfer Governance
    log::info!("{}", "Transferring Governance ownership...".cyan());
    let result = transfer_governance(
        &provider,
        contracts,
        governor_address,
        new_owner,
        chain_id,
        &mut nonce,
        gas_price,
    )
    .await;
    results.push(result);

    // 2. Transfer Ecosystem Chain Admin
    log::info!(
        "{}",
        "Transferring Ecosystem Chain Admin ownership...".cyan()
    );
    let result = transfer_ecosystem_chain_admin(
        &provider,
        contracts,
        governor_address,
        new_owner,
        chain_id,
        &mut nonce,
        gas_price,
    )
    .await;
    results.push(result);

    // 3. Transfer Bridged Token Beacon (Ownable - immediate transfer)
    log::info!(
        "{}",
        "Transferring Bridged Token Beacon ownership...".cyan()
    );
    let result = transfer_bridged_token_beacon(
        &provider,
        contracts,
        governor_address,
        new_owner,
        chain_id,
        &mut nonce,
        gas_price,
    )
    .await;
    results.push(result);

    // 4. Transfer Validator Timelock
    log::info!("{}", "Transferring Validator Timelock ownership...".cyan());
    let result = transfer_validator_timelock(
        &provider,
        contracts,
        governor_address,
        new_owner,
        chain_id,
        &mut nonce,
        gas_price,
    )
    .await;
    results.push(result);

    OwnershipSummary::new(results)
}

/// Transfer ownership for chain-level contracts to a new owner.
///
/// This function transfers ownership for:
/// - Chain Governance (Ownable2Step)
/// - Chain Chain Admin (Ownable2Step)
///
/// Note: For Ownable2Step contracts, the new owner must call acceptOwnership()
/// to complete the transfer.
///
/// # Arguments
///
/// * `rpc_url` - Settlement layer RPC endpoint URL.
/// * `contracts` - Chain contracts containing addresses.
/// * `governor_key` - Governor private key for signing transactions.
/// * `new_owner` - Address to transfer ownership to.
/// * `gas_price_wei` - Optional gas price in wei (estimated if not provided).
///
/// # Returns
///
/// Summary of all ownership transfer attempts.
pub async fn transfer_chain_ownership(
    rpc_url: &str,
    contracts: &ChainContracts,
    governor_key: &SecretString,
    new_owner: Address,
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

    log::info!(
        "Transferring chain ownership to: {}",
        new_owner.to_string().green()
    );

    // 1. Transfer Chain Governance
    log::info!("{}", "Transferring Chain Governance ownership...".cyan());
    let result = transfer_chain_governance(
        &provider,
        contracts,
        governor_address,
        new_owner,
        chain_id,
        &mut nonce,
        gas_price,
    )
    .await;
    results.push(result);

    // 2. Transfer Chain Chain Admin
    log::info!("{}", "Transferring Chain Chain Admin ownership...".cyan());
    let result = transfer_chain_chain_admin(
        &provider,
        contracts,
        governor_address,
        new_owner,
        chain_id,
        &mut nonce,
        gas_price,
    )
    .await;
    results.push(result);

    OwnershipSummary::new(results)
}
