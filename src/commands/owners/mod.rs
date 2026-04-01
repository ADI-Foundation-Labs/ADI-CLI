//! Display L1 contract owners with wallet name mapping.

mod address_map;
mod display;
mod orchestrate;
mod queries;

use adi_types::normalize_rpc_url;
use alloy_primitives::Address;
use alloy_provider::ProviderBuilder;
use alloy_sol_types::sol;
use clap::Args;
use serde::{Deserialize, Serialize};
use url::Url;

use cliclack::progress_bar;

use crate::commands::helpers::{
    create_state_manager_with_context, resolve_ecosystem_name, resolve_rpc_url,
};
use crate::context::Context;
use crate::error::{Result, WrapErr};
use crate::ui;

use address_map::{
    add_chain_contract_addresses, add_operator_addresses, add_wallet_addresses,
    build_known_address_map,
};
use display::display_ownership_results;
use orchestrate::{query_chain_owners, query_ecosystem_owners};

// Define contract interfaces for owner queries
sol! {
    #[allow(missing_docs)]
    function owner() external view returns (address);

    #[allow(missing_docs)]
    function pendingOwner() external view returns (address);

    // Diamond Proxy admin interface (NOT Ownable2Step pattern)
    #[allow(missing_docs)]
    function getAdmin() external view returns (address);

    #[allow(missing_docs)]
    function getPendingAdmin() external view returns (address);
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

impl ContractOwnership {
    /// Create a "not configured" entry (no address available).
    fn not_configured(name: &'static str) -> Self {
        Self {
            name,
            address: None,
            owner: OwnerQueryResult::NotConfigured,
            pending_owner: OwnerQueryResult::NotConfigured,
        }
    }
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
    let state_manager = create_state_manager_with_context(&ecosystem_name, context)?;

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

    // Load ecosystem metadata to get default_chain
    let eco_metadata = state_manager
        .ecosystem()
        .metadata()
        .await
        .wrap_err("Failed to load ecosystem metadata")?;

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

    // Determine chain to query
    let chain_to_display = args.chain.as_ref().unwrap_or(&eco_metadata.default_chain);
    let chain_ops = state_manager.chain(chain_to_display);

    // Check chain state before starting progress bar
    let chain_exists = chain_ops.exists().await.wrap_err("Failed to check chain")?;
    let chain_contracts_exist = if chain_exists {
        chain_ops
            .contracts_exist()
            .await
            .wrap_err("Failed to check chain contracts")?
    } else {
        false
    };

    // Calculate total contracts to query
    // Ecosystem: 14 standard contracts + 1 Message Root Proxy = 15
    // Chain: 5 ownable + 1 Diamond Proxy + 1 ConsensusRegistry (if configured) = 6-7
    let ecosystem_count = 15u64;
    let chain_count = if chain_contracts_exist { 7u64 } else { 0u64 };
    let pb = progress_bar(ecosystem_count + chain_count);
    pb.start("Querying contract owners...");

    // Query ecosystem contract owners
    let ownerships = query_ecosystem_owners(&provider, &contracts, &pb).await;

    // Query chain contracts (if they exist) before stopping progress bar
    let chain_query_result = if chain_contracts_exist {
        let chain_contracts = chain_ops
            .contracts()
            .await
            .wrap_err("Failed to load chain contracts")?;

        let chain_wallets = chain_ops.wallets().await.ok();
        let chain_operators = chain_ops.operators().await.ok();
        let mut combined_map = known_map.clone();

        if let Some(ref cw) = chain_wallets {
            add_wallet_addresses(&mut combined_map, cw);
        }
        if let Some(ref ops) = chain_operators {
            add_operator_addresses(&mut combined_map, ops);
        }
        add_chain_contract_addresses(&mut combined_map, &chain_contracts);

        let chain_ownerships = query_chain_owners(&provider, &chain_contracts, &pb).await;
        Some((chain_ownerships, combined_map))
    } else {
        None
    };

    pb.stop("Ownership queries complete");

    // Now display all results after progress bar is stopped
    display_ownership_results("Ecosystem Contract Owners", &ownerships, &known_map)?;

    if !chain_exists {
        ui::warning(format!("Chain '{}' not found.", chain_to_display))?;
    } else if !chain_contracts_exist {
        ui::info(format!(
            "No contracts deployed for chain '{}'.",
            chain_to_display
        ))?;
    } else if let Some((chain_ownerships, combined_map)) = chain_query_result {
        display_ownership_results(
            &format!("Chain '{}' Contract Owners", chain_to_display),
            &chain_ownerships,
            &combined_map,
        )?;
    }

    // Display explanatory note about contract coverage
    ui::note(
        "Note",
        "Only contracts with ownership functions are shown above.\n\
         Permissionless contracts (DA validators, diamond facets, verifier components,\n\
         implementation contracts, upgrade contracts) have no owner by design.",
    )?;

    ui::outro("")?;
    Ok(())
}

/// Check if contract has a pending owner transfer.
fn has_pending_transfer(result: &OwnerQueryResult) -> bool {
    matches!(result, OwnerQueryResult::Ok(addr) if *addr != Address::ZERO)
}
