//! Orchestrate ownership queries for ecosystem and chain contracts.

use adi_types::{ChainContracts, EcosystemContracts};
use alloy_primitives::Address;
use alloy_provider::Provider;
use cliclack::ProgressBar;
use futures_util::future::join_all;

use super::queries::{
    query_admin, query_bridged_token_beacon, query_owner, query_pending_admin, query_pending_owner,
    query_proxy_admin,
};
use super::{ContractOwnership, OwnerQueryResult};

/// Query ownership for ecosystem contracts.
pub(super) async fn query_ecosystem_owners<P: Provider + Clone>(
    provider: &P,
    contracts: &EcosystemContracts,
    pb: &ProgressBar,
) -> Vec<ContractOwnership> {
    // Resolve the Bridged Token Beacon address from the Native Token Vault
    // (it is not stored in config and must be read on-chain).
    let bridged_token_beacon_addr =
        query_bridged_token_beacon(provider, contracts.native_token_vault_addr()).await;

    // Extract addresses from nested structures
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
        ("Ecosystem Chain Admin", contracts.chain_admin_addr()),
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
        ("L1 Nullifier", contracts.l1_nullifier_addr()),
        ("Shared Bridge", shared_bridge_addr),
        ("Bridged Token Beacon", bridged_token_beacon_addr),
        // Admin contracts
        ("Transparent Proxy Admin", transparent_proxy_admin_addr),
        ("STM Deployment Tracker", stm_tracker_addr),
    ];

    // Query all contracts concurrently. Message Root Proxy uses the EIP-1967
    // admin slot rather than owner(), so it runs alongside in the same batch.
    let ownable_queries = join_all(contract_list.into_iter().map(|(name, address)| async move {
        let result = query_ownable(provider, name, address).await;
        pb.inc(1);
        result
    }));
    let message_root_query = async {
        let result = query_proxy(
            provider,
            "Message Root Proxy (Admin)",
            message_root_proxy_addr,
        )
        .await;
        pb.inc(1);
        result
    };

    let (mut results, message_root) = tokio::join!(ownable_queries, message_root_query);
    results.push(message_root);
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

    // Query all chain contracts concurrently. Diamond Proxy uses the custom
    // getAdmin()/getPendingAdmin() pair, so it runs alongside the Ownable batch.
    let ownable_queries = join_all(ownable_contracts.into_iter().map(
        |(name, address)| async move {
            let result = query_ownable(provider, name, address).await;
            pb.inc(1);
            result
        },
    ));
    let diamond_proxy_addr = contracts.diamond_proxy_addr();
    let diamond_query = async {
        let result = query_admin_pair(provider, "Diamond Proxy (Admin)", diamond_proxy_addr).await;
        pb.inc(1);
        result
    };

    let (mut results, diamond) = tokio::join!(ownable_queries, diamond_query);
    results.push(diamond);

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
    let (owner, pending_owner) = tokio::join!(
        query_owner(provider, addr, name),
        query_pending_owner(provider, addr, name),
    );
    ContractOwnership {
        name,
        address: Some(addr),
        owner,
        pending_owner,
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
    let (owner, pending_owner) = tokio::join!(
        query_admin(provider, addr, name),
        query_pending_admin(provider, addr, name),
    );
    ContractOwnership {
        name,
        address: Some(addr),
        owner,
        pending_owner,
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
