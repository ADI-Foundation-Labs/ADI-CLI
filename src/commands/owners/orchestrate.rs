//! Orchestrate ownership queries for ecosystem and chain contracts.

use adi_types::{ChainContracts, EcosystemContracts};
use alloy_primitives::Address;
use alloy_provider::Provider;
use cliclack::ProgressBar;

use super::queries::{
    query_admin, query_owner, query_pending_admin, query_pending_owner, query_proxy_admin,
};
use super::{ContractOwnership, OwnerQueryResult};

/// Query ownership for ecosystem contracts.
pub(super) async fn query_ecosystem_owners<P: Provider + Clone>(
    provider: &P,
    contracts: &EcosystemContracts,
    pb: &ProgressBar,
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

    let message_root_proxy_addr = contracts
        .core_ecosystem_contracts
        .as_ref()
        .and_then(|c| c.message_root_proxy_addr);

    let l1_wrapped_base_token_store_addr = contracts
        .zksync_os_ctm
        .as_ref()
        .and_then(|c| c.l1_wrapped_base_token_store);

    // List of contracts to query (only those with owner())
    // NOTE: The following contracts are permissionless and have no owner():
    // - ERC20 Bridge (legacy, stateless)
    // - DA Validators (Rollup, Avail, Validium) - permissionless by design
    // - Diamond Facets (Admin, Executor, Mailbox, Getters) - used via Diamond Proxy
    // - Verifier components (Fflonk, Plonk) - individual verifiers
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
        (
            "L1 Wrapped Base Token Store",
            l1_wrapped_base_token_store_addr,
        ),
        // Bridge contracts
        ("L1 Nullifier", l1_nullifier_addr),
        ("Shared Bridge", shared_bridge_addr),
        // Admin contracts
        ("Transparent Proxy Admin", transparent_proxy_admin_addr),
        ("STM Deployment Tracker", stm_tracker_addr),
    ];

    for (name, address) in contract_list {
        results.push(query_ownable(provider, name, address).await);
        pb.inc(1);
    }

    // Message Root Proxy uses EIP-1967 Transparent Proxy pattern (admin in storage slot)
    results.push(
        query_proxy(
            provider,
            "Message Root Proxy (Admin)",
            message_root_proxy_addr,
        )
        .await,
    );
    pb.inc(1);

    results
}

/// Query ownership for chain contracts.
///
/// Note: Diamond Proxy uses a custom admin pattern (getAdmin/getPendingAdmin)
/// instead of the standard Ownable2Step (owner/pendingOwner).
pub(super) async fn query_chain_owners<P: Provider + Clone>(
    provider: &P,
    contracts: &ChainContracts,
    pb: &ProgressBar,
) -> Vec<ContractOwnership> {
    let mut results = Vec::new();

    let chain_proxy_admin_addr = contracts.l1.as_ref().and_then(|l| l.chain_proxy_admin_addr);
    let chain_verifier_addr = contracts.l1.as_ref().and_then(|l| l.verifier_addr);
    let chain_validator_timelock_addr = contracts
        .l1
        .as_ref()
        .and_then(|l| l.validator_timelock_addr);

    // Standard Ownable2Step contracts (owner/pendingOwner)
    let ownable_contracts: Vec<(&'static str, Option<Address>)> = vec![
        ("Chain Governance", contracts.governance_addr()),
        ("Chain Admin", contracts.chain_admin_addr()),
        ("Chain Proxy Admin", chain_proxy_admin_addr),
        ("Chain Verifier", chain_verifier_addr),
        ("Chain Validator Timelock", chain_validator_timelock_addr),
    ];

    for (name, address) in ownable_contracts {
        results.push(query_ownable(provider, name, address).await);
        pb.inc(1);
    }

    // Diamond Proxy uses getAdmin()/getPendingAdmin() instead of owner()/pendingOwner()
    let diamond_proxy_addr = contracts.diamond_proxy_addr();
    results.push(query_admin_pair(provider, "Diamond Proxy (Admin)", diamond_proxy_addr).await);
    pb.inc(1);

    // L2 ConsensusRegistry - placeholder (requires L2 RPC URL)
    let consensus_addr = contracts.l2.as_ref().and_then(|l2| l2.consensus_registry);
    if consensus_addr.is_some() {
        results.push(ContractOwnership {
            name: "ConsensusRegistry (L2)",
            address: consensus_addr,
            owner: OwnerQueryResult::Err("L2 contract - requires --l2-rpc-url".to_string()),
            pending_owner: OwnerQueryResult::Err("L2 contract".to_string()),
        });
        pb.inc(1);
    }

    results
}

// ============================================================================
// Query helpers — flatten the Option<Address> → NotConfigured pattern
// ============================================================================

/// Query owner()/pendingOwner() for an Ownable2Step contract.
async fn query_ownable<P: Provider + Clone>(
    provider: &P,
    name: &'static str,
    address: Option<Address>,
) -> ContractOwnership {
    let Some(addr) = address else {
        return ContractOwnership::not_configured(name);
    };
    ContractOwnership {
        name,
        address: Some(addr),
        owner: query_owner(provider, addr, name).await,
        pending_owner: query_pending_owner(provider, addr, name).await,
    }
}

/// Query getAdmin()/getPendingAdmin() for a Diamond Proxy contract.
async fn query_admin_pair<P: Provider + Clone>(
    provider: &P,
    name: &'static str,
    address: Option<Address>,
) -> ContractOwnership {
    let Some(addr) = address else {
        return ContractOwnership::not_configured(name);
    };
    ContractOwnership {
        name,
        address: Some(addr),
        owner: query_admin(provider, addr, name).await,
        pending_owner: query_pending_admin(provider, addr, name).await,
    }
}

/// Query EIP-1967 proxy admin (no pending concept).
async fn query_proxy<P: Provider + Clone>(
    provider: &P,
    name: &'static str,
    address: Option<Address>,
) -> ContractOwnership {
    let Some(addr) = address else {
        return ContractOwnership::not_configured(name);
    };
    ContractOwnership {
        name,
        address: Some(addr),
        owner: query_proxy_admin(provider, addr, name).await,
        pending_owner: OwnerQueryResult::Err("not applicable".to_string()),
    }
}
