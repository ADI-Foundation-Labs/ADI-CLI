//! Build known address maps from wallets, operators, and contracts.

use adi_types::{ChainContracts, EcosystemContracts, Operators, Wallets};
use alloy_primitives::Address;
use std::collections::HashMap;

/// Build a map from known addresses (wallets + contracts) to their names.
pub(super) fn build_known_address_map(
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
        if let Some(addr) = ctm.l1_wrapped_base_token_store {
            map.insert(addr, "L1 Wrapped Base Token Store");
        }
    }
    if let Some(ref core) = contracts.core_ecosystem_contracts {
        if let Some(addr) = core.transparent_proxy_admin_addr {
            map.insert(addr, "Transparent Proxy Admin");
        }
        if let Some(addr) = core.stm_deployment_tracker_proxy_addr {
            map.insert(addr, "STM Deployment Tracker");
        }
        if let Some(addr) = core.message_root_proxy_addr {
            map.insert(addr, "Message Root Proxy");
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
pub(super) fn add_wallet_addresses(map: &mut HashMap<Address, &'static str>, wallets: &Wallets) {
    if let Some(w) = &wallets.deployer {
        map.insert(w.address, "deployer");
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
}

/// Add operator override addresses to the map (from CLI/config).
pub(super) fn add_operator_addresses(
    map: &mut HashMap<Address, &'static str>,
    operators: &Operators,
) {
    if let Some(addr) = operators.operator {
        map.insert(addr, "operator (override)");
    }
    if let Some(addr) = operators.prove_operator {
        map.insert(addr, "prove_operator (override)");
    }
    if let Some(addr) = operators.execute_operator {
        map.insert(addr, "execute_operator (override)");
    }
}

/// Add chain contract addresses to the map.
pub(super) fn add_chain_contract_addresses(
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
    // L2 contracts
    if let Some(ref l2) = contracts.l2 {
        if let Some(addr) = l2.consensus_registry {
            map.insert(addr, "ConsensusRegistry");
        }
    }
}
