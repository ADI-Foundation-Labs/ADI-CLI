//! Display L1 contract owners with wallet name mapping.

use adi_types::{normalize_rpc_url, ChainContracts, EcosystemContracts, Wallets};
use alloy_primitives::Address;
use alloy_provider::{Provider, ProviderBuilder};
use alloy_rpc_types::TransactionRequest;
use alloy_sol_types::{sol, SolCall};
use clap::Args;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use url::Url;

use crate::commands::helpers::{
    create_state_manager_with_context, resolve_ecosystem_name, resolve_rpc_url,
};
use crate::context::Context;
use crate::error::{Result, WrapErr};
use crate::ui;

// Define contract interfaces for owner queries
sol! {
    #[allow(missing_docs)]
    function owner() external view returns (address);

    #[allow(missing_docs)]
    function pendingOwner() external view returns (address);
}

/// Arguments for `owners` command.
#[derive(Clone, Args, Debug, Serialize, Deserialize)]
pub struct OwnersArgs {
    /// Ecosystem name (falls back to config file if not provided).
    #[arg(long, help = "Ecosystem name (falls back to config if not provided)")]
    pub ecosystem_name: Option<String>,

    /// Settlement layer JSON-RPC URL.
    #[arg(long, env = "ADI_RPC_URL", help = "Settlement layer RPC URL")]
    pub rpc_url: Option<Url>,

    /// Chain name to display chain-level contract owners.
    #[arg(long, help = "Chain name to display chain-level contract owners")]
    pub chain: Option<String>,
}

/// Query result for owner/pendingOwner calls.
#[derive(Clone, Debug)]
enum OwnerQueryResult {
    /// Successfully queried owner address.
    Ok(Address),
    /// Query failed with error message.
    Err(String),
    /// Contract not configured (no address).
    NotConfigured,
}

/// Contract ownership information.
struct ContractOwnership {
    name: &'static str,
    address: Option<Address>,
    owner: OwnerQueryResult,
    pending_owner: OwnerQueryResult,
}

/// Execute the owners command.
pub async fn run(args: &OwnersArgs, context: &Context) -> Result<()> {
    ui::intro("ADI Contract Owners")?;

    // Resolve configuration
    let ecosystem_name = resolve_ecosystem_name(args.ecosystem_name.as_ref(), context.config())?;
    let rpc_url = resolve_rpc_url(args.rpc_url.as_ref(), context.config())?;

    ui::info(format!("Ecosystem: {}", ui::green(&ecosystem_name)))?;
    ui::info(format!("RPC: {}", ui::green(&rpc_url)))?;

    // Create state manager
    let state_manager = create_state_manager_with_context(&ecosystem_name, context);

    // Check if ecosystem exists
    if !state_manager
        .exists()
        .await
        .wrap_err("Failed to check ecosystem")?
    {
        ui::warning(format!(
            "Ecosystem '{}' not found.\nRun 'adi init' first to initialize the ecosystem.",
            ecosystem_name
        ))?;
        ui::outro("")?;
        return Ok(());
    }

    // Load ecosystem wallets for address mapping
    let wallets = state_manager
        .ecosystem()
        .wallets()
        .await
        .wrap_err("Failed to load ecosystem wallets")?;

    // Check if contracts exist
    if !state_manager
        .ecosystem()
        .contracts_exist()
        .await
        .wrap_err("Failed to check contracts")?
    {
        ui::info("No contracts deployed yet. Run 'adi deploy' to deploy.")?;
        ui::outro("")?;
        return Ok(());
    }

    // Load contracts
    let contracts = state_manager
        .ecosystem()
        .contracts()
        .await
        .wrap_err("Failed to load ecosystem contracts")?;

    // Build combined known address map (wallets + contracts)
    let known_map = build_known_address_map(&wallets, &contracts);

    // Create provider
    let normalized_url = normalize_rpc_url(rpc_url.as_str());
    let url: Url = normalized_url.parse().wrap_err("Invalid RPC URL")?;
    let provider = ProviderBuilder::new().connect_http(url);

    // Query ecosystem contract owners
    let ownerships = query_ecosystem_owners(&provider, &contracts).await;

    // Display results
    display_ownership_results("Ecosystem Contract Owners", &ownerships, &known_map)?;

    // If chain is specified, also query chain contracts
    if let Some(ref chain_name) = args.chain {
        let chain_ops = state_manager.chain(chain_name);

        if !chain_ops.exists().await.wrap_err("Failed to check chain")? {
            ui::warning(format!("Chain '{}' not found.", chain_name))?;
        } else if !chain_ops
            .contracts_exist()
            .await
            .wrap_err("Failed to check chain contracts")?
        {
            ui::info(format!("No contracts deployed for chain '{}'.", chain_name))?;
        } else {
            let chain_contracts = chain_ops
                .contracts()
                .await
                .wrap_err("Failed to load chain contracts")?;

            // Load chain wallets and merge with ecosystem known addresses
            let chain_wallets = chain_ops.wallets().await.ok();
            let mut combined_map = known_map.clone();

            // Add chain wallet addresses
            if let Some(ref cw) = chain_wallets {
                add_wallet_addresses(&mut combined_map, cw);
            }

            // Add chain contract addresses
            add_chain_contract_addresses(&mut combined_map, &chain_contracts);

            let chain_ownerships = query_chain_owners(&provider, &chain_contracts).await;
            display_ownership_results(
                &format!("Chain '{}' Contract Owners", chain_name),
                &chain_ownerships,
                &combined_map,
            )?;
        }
    }

    ui::outro("")?;
    Ok(())
}

/// Build a map from known addresses (wallets + contracts) to their names.
fn build_known_address_map(
    wallets: &Wallets,
    contracts: &EcosystemContracts,
) -> HashMap<Address, &'static str> {
    let mut map = HashMap::new();

    // Add wallet addresses
    add_wallet_addresses(&mut map, wallets);

    // Add ecosystem contract addresses
    if let Some(addr) = contracts.governance_addr() {
        map.insert(addr, "Governance");
    }
    if let Some(addr) = contracts.chain_admin_addr() {
        map.insert(addr, "Chain Admin");
    }
    if let Some(addr) = contracts.validator_timelock_addr() {
        map.insert(addr, "Validator Timelock");
    }
    if let Some(addr) = contracts.bridgehub_addr() {
        map.insert(addr, "Bridgehub");
    }
    if let Some(addr) = contracts.native_token_vault_addr() {
        map.insert(addr, "Native Token Vault");
    }
    if let Some(addr) = contracts.server_notifier_addr() {
        map.insert(addr, "Server Notifier");
    }
    if let Some(addr) = contracts.verifier_addr() {
        map.insert(addr, "Verifier");
    }
    if let Some(addr) = contracts.l1_rollup_da_manager_addr() {
        map.insert(addr, "Rollup DA Manager");
    }

    // Nested contract addresses
    if let Some(ref ctm) = contracts.zksync_os_ctm {
        if let Some(addr) = ctm.state_transition_proxy_addr {
            map.insert(addr, "State Transition (CTM)");
        }
    }
    if let Some(ref core) = contracts.core_ecosystem_contracts {
        if let Some(addr) = core.transparent_proxy_admin_addr {
            map.insert(addr, "Transparent Proxy Admin");
        }
        if let Some(addr) = core.stm_deployment_tracker_proxy_addr {
            map.insert(addr, "STM Deployment Tracker");
        }
    }
    if let Some(ref bridges) = contracts.bridges {
        if let Some(addr) = bridges.l1_nullifier_addr {
            map.insert(addr, "L1 Nullifier");
        }
        if let Some(ref erc20) = bridges.erc20 {
            if let Some(addr) = erc20.l1_address {
                map.insert(addr, "ERC20 Bridge");
            }
        }
        if let Some(ref shared) = bridges.shared {
            if let Some(addr) = shared.l1_address {
                map.insert(addr, "Shared Bridge");
            }
        }
    }

    map
}

/// Add wallet addresses to the map.
fn add_wallet_addresses(map: &mut HashMap<Address, &'static str>, wallets: &Wallets) {
    if let Some(w) = &wallets.deployer {
        map.insert(w.address, "deployer");
    }
    if let Some(w) = &wallets.operator {
        map.insert(w.address, "operator");
    }
    if let Some(w) = &wallets.blob_operator {
        map.insert(w.address, "blob_operator");
    }
    if let Some(w) = &wallets.prove_operator {
        map.insert(w.address, "prove_operator");
    }
    if let Some(w) = &wallets.execute_operator {
        map.insert(w.address, "execute_operator");
    }
    if let Some(w) = &wallets.fee_account {
        map.insert(w.address, "fee_account");
    }
    if let Some(w) = &wallets.governor {
        map.insert(w.address, "governor");
    }
    if let Some(w) = &wallets.token_multiplier_setter {
        map.insert(w.address, "token_multiplier_setter");
    }
}

/// Add chain contract addresses to the map.
fn add_chain_contract_addresses(
    map: &mut HashMap<Address, &'static str>,
    contracts: &ChainContracts,
) {
    if let Some(addr) = contracts.governance_addr() {
        map.insert(addr, "Chain Governance");
    }
    if let Some(addr) = contracts.chain_admin_addr() {
        map.insert(addr, "Chain Admin");
    }
    if let Some(addr) = contracts.diamond_proxy_addr() {
        map.insert(addr, "Diamond Proxy");
    }
    if let Some(ref l1) = contracts.l1 {
        if let Some(addr) = l1.chain_proxy_admin_addr {
            map.insert(addr, "Chain Proxy Admin");
        }
        if let Some(addr) = l1.verifier_addr {
            map.insert(addr, "Chain Verifier");
        }
        if let Some(addr) = l1.validator_timelock_addr {
            map.insert(addr, "Chain Validator Timelock");
        }
    }
}

/// Query owner() on a contract, returning detailed error on failure.
async fn query_owner<P: Provider + Clone>(
    provider: &P,
    contract_address: Address,
    contract_name: &str,
) -> OwnerQueryResult {
    let calldata = ownerCall {}.abi_encode();
    let tx = TransactionRequest::default()
        .to(contract_address)
        .input(calldata.into());

    match provider.call(tx).await {
        Ok(result) => {
            if let Some(addr_bytes) = result.get(12..32) {
                OwnerQueryResult::Ok(Address::from_slice(addr_bytes))
            } else {
                let err = format!("invalid response length: {} bytes", result.len());
                log::debug!("Query owner() failed for {}: {}", contract_name, err);
                OwnerQueryResult::Err(err)
            }
        }
        Err(e) => {
            let err = format_rpc_error(&e);
            log::debug!("Query owner() failed for {}: {}", contract_name, err);
            OwnerQueryResult::Err(err)
        }
    }
}

/// Query pendingOwner() on a contract, returning detailed error on failure.
async fn query_pending_owner<P: Provider + Clone>(
    provider: &P,
    contract_address: Address,
    contract_name: &str,
) -> OwnerQueryResult {
    let calldata = pendingOwnerCall {}.abi_encode();
    let tx = TransactionRequest::default()
        .to(contract_address)
        .input(calldata.into());

    match provider.call(tx).await {
        Ok(result) => {
            if let Some(addr_bytes) = result.get(12..32) {
                OwnerQueryResult::Ok(Address::from_slice(addr_bytes))
            } else {
                let err = format!("invalid response length: {} bytes", result.len());
                log::debug!("Query pendingOwner() failed for {}: {}", contract_name, err);
                OwnerQueryResult::Err(err)
            }
        }
        Err(e) => {
            // pendingOwner() not implemented is common, treat as "not set"
            let err = format_rpc_error(&e);
            log::debug!("Query pendingOwner() failed for {}: {}", contract_name, err);
            OwnerQueryResult::Err(err)
        }
    }
}

/// Format RPC error to a short, readable message.
fn format_rpc_error(e: &impl std::fmt::Display) -> String {
    let full = e.to_string();
    // Extract just the meaningful part from verbose RPC errors
    if full.contains("execution reverted") {
        "execution reverted".to_string()
    } else if full.contains("invalid opcode") {
        "invalid opcode".to_string()
    } else if full.contains("out of gas") {
        "out of gas".to_string()
    } else if let Some(start) = full.find("message:") {
        let end = start.saturating_add(50);
        full.get(start..end).unwrap_or(&full).to_string()
    } else {
        // Truncate long errors
        let truncated: String = full.chars().take(60).collect();
        if full.len() > 60 {
            format!("{}...", truncated)
        } else {
            truncated
        }
    }
}

/// Query ownership for ecosystem contracts.
async fn query_ecosystem_owners<P: Provider + Clone>(
    provider: &P,
    contracts: &EcosystemContracts,
) -> Vec<ContractOwnership> {
    let mut results = Vec::new();

    // Extract addresses from nested structures
    let l1_nullifier_addr = contracts.bridges.as_ref().and_then(|b| b.l1_nullifier_addr);

    let state_transition_addr = contracts
        .zksync_os_ctm
        .as_ref()
        .and_then(|c| c.state_transition_proxy_addr);

    let stm_tracker_addr = contracts
        .core_ecosystem_contracts
        .as_ref()
        .and_then(|c| c.stm_deployment_tracker_proxy_addr);

    let transparent_proxy_admin_addr = contracts
        .core_ecosystem_contracts
        .as_ref()
        .and_then(|c| c.transparent_proxy_admin_addr);

    let shared_bridge_addr = contracts
        .bridges
        .as_ref()
        .and_then(|b| b.shared.as_ref())
        .and_then(|s| s.l1_address);

    // List of contracts to query (only those with owner())
    // NOTE: The following contracts are permissionless and have no owner():
    // - ERC20 Bridge (legacy, stateless)
    // - DA Validators (Rollup, Avail, Validium) - permissionless by design
    let contract_list: Vec<(&'static str, Option<Address>)> = vec![
        // Governance contracts
        ("Governance", contracts.governance_addr()),
        ("Chain Admin", contracts.chain_admin_addr()),
        ("Validator Timelock", contracts.validator_timelock_addr()),
        // Core infrastructure
        ("State Transition (CTM)", state_transition_addr),
        ("Bridgehub", contracts.bridgehub_addr()),
        ("Native Token Vault", contracts.native_token_vault_addr()),
        // Operational contracts
        ("Server Notifier", contracts.server_notifier_addr()),
        ("Verifier", contracts.verifier_addr()),
        ("Rollup DA Manager", contracts.l1_rollup_da_manager_addr()),
        // Bridge contracts
        ("L1 Nullifier", l1_nullifier_addr),
        ("Shared Bridge", shared_bridge_addr),
        // Admin contracts
        ("Transparent Proxy Admin", transparent_proxy_admin_addr),
        ("STM Deployment Tracker", stm_tracker_addr),
    ];

    for (name, address) in contract_list {
        let (owner, pending_owner) = if let Some(addr) = address {
            let owner = query_owner(provider, addr, name).await;
            let pending = query_pending_owner(provider, addr, name).await;
            (owner, pending)
        } else {
            (
                OwnerQueryResult::NotConfigured,
                OwnerQueryResult::NotConfigured,
            )
        };

        results.push(ContractOwnership {
            name,
            address,
            owner,
            pending_owner,
        });
    }

    results
}

/// Query ownership for chain contracts.
async fn query_chain_owners<P: Provider + Clone>(
    provider: &P,
    contracts: &ChainContracts,
) -> Vec<ContractOwnership> {
    let mut results = Vec::new();

    let chain_proxy_admin_addr = contracts.l1.as_ref().and_then(|l| l.chain_proxy_admin_addr);
    let chain_verifier_addr = contracts.l1.as_ref().and_then(|l| l.verifier_addr);
    let chain_validator_timelock_addr = contracts
        .l1
        .as_ref()
        .and_then(|l| l.validator_timelock_addr);

    let contract_list: Vec<(&'static str, Option<Address>)> = vec![
        ("Chain Governance", contracts.governance_addr()),
        ("Chain Admin", contracts.chain_admin_addr()),
        ("Diamond Proxy", contracts.diamond_proxy_addr()),
        ("Chain Proxy Admin", chain_proxy_admin_addr),
        ("Chain Verifier", chain_verifier_addr),
        ("Chain Validator Timelock", chain_validator_timelock_addr),
    ];

    for (name, address) in contract_list {
        let (owner, pending_owner) = if let Some(addr) = address {
            let owner = query_owner(provider, addr, name).await;
            let pending = query_pending_owner(provider, addr, name).await;
            (owner, pending)
        } else {
            (
                OwnerQueryResult::NotConfigured,
                OwnerQueryResult::NotConfigured,
            )
        };

        results.push(ContractOwnership {
            name,
            address,
            owner,
            pending_owner,
        });
    }

    results
}

/// Check if contract has a pending owner transfer.
fn has_pending_transfer(result: &OwnerQueryResult) -> bool {
    matches!(result, OwnerQueryResult::Ok(addr) if *addr != Address::ZERO)
}

/// Display ownership results in formatted output.
fn display_ownership_results(
    title: &str,
    ownerships: &[ContractOwnership],
    known_map: &HashMap<Address, &'static str>,
) -> Result<()> {
    let mut lines = Vec::new();

    // Count pending transfers for summary
    let pending_count = ownerships
        .iter()
        .filter(|o| has_pending_transfer(&o.pending_owner))
        .count();

    for ownership in ownerships {
        match ownership.address {
            Some(addr) => {
                // Highlight contracts with pending transfers
                let has_pending = has_pending_transfer(&ownership.pending_owner);
                let name_line = if has_pending {
                    format!(
                        "{} {}: {}",
                        ui::yellow("[PENDING]"),
                        ownership.name,
                        ui::green(addr)
                    )
                } else {
                    format!("{}: {}", ownership.name, ui::green(addr))
                };
                lines.push(name_line);

                // Owner line
                let owner_display = format_owner_result(&ownership.owner, known_map);
                lines.push(format!("  Owner: {}", owner_display));

                // Pending owner line
                let pending_display = format_pending_result(&ownership.pending_owner, known_map);
                lines.push(format!("  Pending: {}", pending_display));

                lines.push(String::new());
            }
            None => {
                lines.push(format!(
                    "{}: {}",
                    ownership.name,
                    ui::cyan("not configured")
                ));
                lines.push(String::new());
            }
        }
    }

    // Remove trailing empty line
    if lines.last().is_some_and(|l| l.is_empty()) {
        lines.pop();
    }

    // Add summary if there are pending transfers
    let title_with_summary = if pending_count > 0 {
        format!("{} ({} pending)", title, ui::yellow(pending_count))
    } else {
        title.to_string()
    };

    ui::note(&title_with_summary, lines.join("\n"))?;
    Ok(())
}

/// Format owner query result with known address mapping.
fn format_owner_result(
    result: &OwnerQueryResult,
    known_map: &HashMap<Address, &'static str>,
) -> String {
    match result {
        OwnerQueryResult::Ok(addr) => {
            let role = known_map
                .get(addr)
                .map(|r| format!(" ({})", r))
                .unwrap_or_default();
            format!("{}{}", ui::green(addr), ui::cyan(role))
        }
        OwnerQueryResult::Err(e) => ui::yellow(format!("query failed: {}", e)).to_string(),
        OwnerQueryResult::NotConfigured => ui::cyan("not configured").to_string(),
    }
}

/// Format pending owner query result.
/// Active pending transfers are highlighted in yellow for visibility.
fn format_pending_result(
    result: &OwnerQueryResult,
    known_map: &HashMap<Address, &'static str>,
) -> String {
    match result {
        OwnerQueryResult::Ok(addr) if *addr == Address::ZERO => ui::cyan("not set").to_string(),
        OwnerQueryResult::Ok(addr) => {
            // Highlight pending transfers in yellow for visibility
            let role = known_map
                .get(addr)
                .map(|r| format!(" ({})", r))
                .unwrap_or_default();
            format!("{}{}", ui::yellow(addr), ui::cyan(role))
        }
        OwnerQueryResult::Err(_) => {
            // pendingOwner() not implemented is common, show as "not set"
            ui::cyan("not set").to_string()
        }
        OwnerQueryResult::NotConfigured => ui::cyan("not configured").to_string(),
    }
}
