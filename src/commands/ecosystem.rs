//! Display ecosystem and chain information with deployed contracts.

use adi_state::StateManager;
use adi_types::{
    BaseToken, BatchCommitDataMode, BridgesConfig, ChainContracts, ChainEcosystemContracts,
    ChainL1Contracts, ChainL2Contracts, ChainMetadata, CoreEcosystemContracts, EcosystemContracts,
    EcosystemMetadata, InitialDeployments, L1Contracts, ProverMode, VmOption, ZkSyncOsCtm,
};
use alloy_primitives::{Address, B256};
use clap::Args;
use serde::{Deserialize, Serialize};

use crate::commands::helpers::{create_state_manager_with_context, resolve_ecosystem_name};
use crate::context::Context;
use crate::error::{Result, WrapErr};
use crate::ui;

/// Arguments for `ecosystem` command.
#[derive(Clone, Args, Debug, Serialize, Deserialize)]
pub struct EcosystemArgs {
    /// Ecosystem name (falls back to config file if not provided).
    #[arg(long, help = "Ecosystem name (falls back to config if not provided)")]
    pub ecosystem_name: Option<String>,

    /// Chain name to display chain-level information.
    #[arg(long, help = "Chain name to display chain-level information")]
    pub chain: Option<String>,
}

// ============================================================================
// Value formatting helpers
// ============================================================================

/// Format an optional address field with green color.
fn format_addr(name: &str, addr: Option<Address>) -> String {
    match addr {
        Some(a) => format!("{}: {}", name, ui::green(a)),
        None => format!("{}: {}", name, ui::cyan("not set")),
    }
}

/// Format an optional hash field with green color.
fn format_hash(name: &str, hash: Option<B256>) -> String {
    match hash {
        Some(h) => format!("{}: {}", name, ui::green(h)),
        None => format!("{}: {}", name, ui::cyan("not set")),
    }
}

/// Format a value with green color.
fn format_val<T: std::fmt::Display>(name: &str, val: T) -> String {
    format!("{}: {}", name, ui::green(val))
}

// ============================================================================
// Metadata formatting
// ============================================================================

/// Format ecosystem metadata for display.
fn format_ecosystem_metadata(
    meta: &EcosystemMetadata,
    deployments: Option<&InitialDeployments>,
) -> String {
    let mut lines = vec![
        format_val("L1 Network", &meta.l1_network),
        format_val("Era Chain ID", meta.era_chain_id),
        format_val("Prover Mode", format_prover_mode(&meta.prover_version)),
        format_val("Default Chain", &meta.default_chain),
    ];

    if let Some(dep) = deployments {
        lines.push(format_val(
            "Governance Min Delay",
            format!("{}s", dep.governance_min_delay),
        ));
    }

    lines.join("\n")
}

/// Format chain metadata for display.
fn format_chain_metadata(meta: &ChainMetadata) -> String {
    let base_token_display = format_base_token(&meta.base_token);

    [
        format_val("Chain ID", meta.chain_id),
        format_val("L1 Network", &meta.l1_network),
        format_val("Prover Mode", format_prover_mode(&meta.prover_version)),
        format_val("Base Token", base_token_display),
        format_val(
            "Batch Mode",
            format_batch_mode(&meta.l1_batch_commit_data_generator_mode),
        ),
        format_val("VM Option", format_vm_option(&meta.vm_option)),
        format_val("EVM Emulator", meta.evm_emulator),
    ]
    .join("\n")
}

/// Format prover mode for display.
fn format_prover_mode(mode: &ProverMode) -> &'static str {
    match mode {
        ProverMode::NoProofs => "NoProofs",
        ProverMode::Gpu => "GPU",
    }
}

/// Format batch commit data mode for display.
fn format_batch_mode(mode: &BatchCommitDataMode) -> &'static str {
    match mode {
        BatchCommitDataMode::Rollup => "Rollup",
        BatchCommitDataMode::Validium => "Validium",
    }
}

/// Format VM option for display.
fn format_vm_option(opt: &VmOption) -> &'static str {
    match opt {
        VmOption::ZKSyncOsVM => "ZKSyncOsVM",
        VmOption::Evm => "EVM",
    }
}

/// Format base token for display.
fn format_base_token(token: &BaseToken) -> String {
    if token.is_eth() {
        "ETH".to_string()
    } else {
        format!("{}", token.address)
    }
}

// ============================================================================
// Contract formatting (existing logic)
// ============================================================================

/// Format core ecosystem contracts section.
fn format_core(core: &CoreEcosystemContracts) -> Vec<String> {
    vec![
        format!(
            "  {}",
            format_addr("Bridgehub Proxy", core.bridgehub_proxy_addr)
        ),
        format!(
            "  {}",
            format_addr("Message Root Proxy", core.message_root_proxy_addr)
        ),
        format!(
            "  {}",
            format_addr("Proxy Admin", core.transparent_proxy_admin_addr)
        ),
        format!(
            "  {}",
            format_addr(
                "STM Deployment Tracker",
                core.stm_deployment_tracker_proxy_addr
            )
        ),
        format!(
            "  {}",
            format_addr("Native Token Vault", core.native_token_vault_addr)
        ),
    ]
}

/// Format bridges section.
fn format_bridges(bridges: &BridgesConfig) -> Vec<String> {
    let mut lines = Vec::new();
    if let Some(ref erc20) = bridges.erc20 {
        lines.push(format!("  {}", format_addr("ERC20 L1", erc20.l1_address)));
        lines.push(format!("  {}", format_addr("ERC20 L2", erc20.l2_address)));
    }
    if let Some(ref shared) = bridges.shared {
        lines.push(format!("  {}", format_addr("Shared L1", shared.l1_address)));
        lines.push(format!("  {}", format_addr("Shared L2", shared.l2_address)));
    }
    lines.push(format!(
        "  {}",
        format_addr("L1 Nullifier", bridges.l1_nullifier_addr)
    ));
    lines
}

/// Format L1 contracts section.
fn format_l1(l1: &L1Contracts) -> Vec<String> {
    vec![
        format!("  {}", format_addr("Governance", l1.governance_addr)),
        format!("  {}", format_addr("Chain Admin", l1.chain_admin_addr)),
        format!(
            "  {}",
            format_addr("Transaction Filterer", l1.transaction_filterer_addr)
        ),
    ]
}

/// Format ZkSync OS CTM section.
fn format_ctm(ctm: &ZkSyncOsCtm) -> Vec<String> {
    let mut lines = vec![
        format!("  {}", format_addr("Governance", ctm.governance)),
        format!("  {}", format_addr("Chain Admin", ctm.chain_admin)),
        format!("  {}", format_addr("Proxy Admin", ctm.proxy_admin)),
        format!(
            "  {}",
            format_addr("State Transition Proxy", ctm.state_transition_proxy_addr)
        ),
        format!(
            "  {}",
            format_addr("Validator Timelock", ctm.validator_timelock_addr)
        ),
        format!(
            "  {}",
            format_addr("Server Notifier", ctm.server_notifier_proxy_addr)
        ),
        format!("  {}", format_addr("Verifier", ctm.verifier_addr)),
        format!(
            "  {}",
            format_addr("L1 Rollup DA Manager", ctm.l1_rollup_da_manager)
        ),
        format!(
            "  {}",
            format_addr("L1 Bytecodes Supplier", ctm.l1_bytecodes_supplier_addr)
        ),
        format!(
            "  {}",
            format_addr(
                "L1 Wrapped Base Token Store",
                ctm.l1_wrapped_base_token_store
            )
        ),
        format!(
            "  {}",
            format_addr("Default Upgrade", ctm.default_upgrade_addr)
        ),
        format!(
            "  {}",
            format_addr("Genesis Upgrade", ctm.genesis_upgrade_addr)
        ),
        format!(
            "  {}",
            format_addr("Rollup L1 DA Validator", ctm.rollup_l1_da_validator_addr)
        ),
        format!(
            "  {}",
            format_addr(
                "No DA Validium L1 Validator",
                ctm.no_da_validium_l1_validator_addr
            )
        ),
        format!(
            "  {}",
            format_addr(
                "Blobs ZkSync OS L1 DA Validator",
                ctm.blobs_zksync_os_l1_da_validator_addr
            )
        ),
        format!(
            "  {}",
            format_addr("Avail L1 DA Validator", ctm.avail_l1_da_validator_addr)
        ),
    ];

    // Diamond facets (extracted from diamond_cut_data)
    if ctm.admin_facet_addr.is_some()
        || ctm.executor_facet_addr.is_some()
        || ctm.mailbox_facet_addr.is_some()
        || ctm.getters_facet_addr.is_some()
        || ctm.diamond_init_addr.is_some()
    {
        lines.push(String::new());
        lines.push("  Diamond Facets:".to_string());
        lines.push(format!(
            "    {}",
            format_addr("Admin Facet", ctm.admin_facet_addr)
        ));
        lines.push(format!(
            "    {}",
            format_addr("Executor Facet", ctm.executor_facet_addr)
        ));
        lines.push(format!(
            "    {}",
            format_addr("Mailbox Facet", ctm.mailbox_facet_addr)
        ));
        lines.push(format!(
            "    {}",
            format_addr("Getters Facet", ctm.getters_facet_addr)
        ));
        lines.push(format!(
            "    {}",
            format_addr("Diamond Init", ctm.diamond_init_addr)
        ));
    }

    // Implementation contracts (read via EIP-1967)
    if ctm.bridgehub_impl_addr.is_some()
        || ctm.message_root_impl_addr.is_some()
        || ctm.chain_type_manager_impl_addr.is_some()
    {
        lines.push(String::new());
        lines.push("  Implementation Contracts:".to_string());
        lines.push(format!(
            "    {}",
            format_addr("Bridgehub Impl", ctm.bridgehub_impl_addr)
        ));
        lines.push(format!(
            "    {}",
            format_addr("Message Root Impl", ctm.message_root_impl_addr)
        ));
        lines.push(format!(
            "    {}",
            format_addr("Native Token Vault Impl", ctm.native_token_vault_impl_addr)
        ));
        lines.push(format!(
            "    {}",
            format_addr(
                "STM Deployment Tracker Impl",
                ctm.stm_deployment_tracker_impl_addr
            )
        ));
        lines.push(format!(
            "    {}",
            format_addr("Chain Type Manager Impl", ctm.chain_type_manager_impl_addr)
        ));
        lines.push(format!(
            "    {}",
            format_addr("Server Notifier Impl", ctm.server_notifier_impl_addr)
        ));
        lines.push(format!(
            "    {}",
            format_addr("ERC20 Bridge Impl", ctm.erc20_bridge_impl_addr)
        ));
        lines.push(format!(
            "    {}",
            format_addr("Shared Bridge Impl", ctm.shared_bridge_impl_addr)
        ));
        lines.push(format!(
            "    {}",
            format_addr("L1 Nullifier Impl", ctm.l1_nullifier_impl_addr)
        ));
        lines.push(format!(
            "    {}",
            format_addr("Validator Timelock Impl", ctm.validator_timelock_impl_addr)
        ));
    }

    // Verifier components
    if ctm.verifier_fflonk_addr.is_some() || ctm.verifier_plonk_addr.is_some() {
        lines.push(String::new());
        lines.push("  Verifier Components:".to_string());
        lines.push(format!(
            "    {}",
            format_addr("Verifier Fflonk", ctm.verifier_fflonk_addr)
        ));
        lines.push(format!(
            "    {}",
            format_addr("Verifier Plonk", ctm.verifier_plonk_addr)
        ));
    }

    // Bridge token contracts
    if ctm.bridged_standard_erc20_addr.is_some() || ctm.bridged_token_beacon_addr.is_some() {
        lines.push(String::new());
        lines.push("  Bridge Token Contracts:".to_string());
        lines.push(format!(
            "    {}",
            format_addr("Bridged Standard ERC20", ctm.bridged_standard_erc20_addr)
        ));
        lines.push(format!(
            "    {}",
            format_addr("Bridged Token Beacon", ctm.bridged_token_beacon_addr)
        ));
    }

    // Avail test contracts
    if ctm.dummy_avail_bridge_addr.is_some() || ctm.dummy_vector_x_addr.is_some() {
        lines.push(String::new());
        lines.push("  Avail Test Contracts:".to_string());
        lines.push(format!(
            "    {}",
            format_addr("Dummy Avail Bridge", ctm.dummy_avail_bridge_addr)
        ));
        lines.push(format!(
            "    {}",
            format_addr("Dummy VectorX", ctm.dummy_vector_x_addr)
        ));
    }

    // Server notifier proxy admin (if separate)
    if ctm.server_notifier_proxy_admin_addr.is_some() {
        lines.push(format!(
            "  {}",
            format_addr(
                "Server Notifier Proxy Admin",
                ctm.server_notifier_proxy_admin_addr
            )
        ));
    }

    lines
}

/// Count non-None addresses in ecosystem contracts.
fn count_ecosystem_contracts(contracts: &EcosystemContracts) -> usize {
    let mut count = 0;

    // Top-level addresses
    if contracts.create2_factory_addr.is_some() {
        count += 1;
    }
    if contracts.multicall3_addr.is_some() {
        count += 1;
    }

    // Core ecosystem contracts
    if let Some(ref core) = contracts.core_ecosystem_contracts {
        if core.bridgehub_proxy_addr.is_some() {
            count += 1;
        }
        if core.message_root_proxy_addr.is_some() {
            count += 1;
        }
        if core.transparent_proxy_admin_addr.is_some() {
            count += 1;
        }
        if core.stm_deployment_tracker_proxy_addr.is_some() {
            count += 1;
        }
        if core.native_token_vault_addr.is_some() {
            count += 1;
        }
    }

    // Bridges
    if let Some(ref bridges) = contracts.bridges {
        if let Some(ref erc20) = bridges.erc20 {
            if erc20.l1_address.is_some() {
                count += 1;
            }
            if erc20.l2_address.is_some() {
                count += 1;
            }
        }
        if let Some(ref shared) = bridges.shared {
            if shared.l1_address.is_some() {
                count += 1;
            }
            if shared.l2_address.is_some() {
                count += 1;
            }
        }
        if bridges.l1_nullifier_addr.is_some() {
            count += 1;
        }
    }

    // L1 contracts
    if let Some(ref l1) = contracts.l1 {
        if l1.governance_addr.is_some() {
            count += 1;
        }
        if l1.chain_admin_addr.is_some() {
            count += 1;
        }
        if l1.transaction_filterer_addr.is_some() {
            count += 1;
        }
    }

    // ZkSync OS CTM
    if let Some(ref ctm) = contracts.zksync_os_ctm {
        count += count_ctm_contracts(ctm);
    }

    count
}

/// Count non-None addresses in ZkSyncOsCtm.
fn count_ctm_contracts(ctm: &ZkSyncOsCtm) -> usize {
    let mut count = 0;

    // Core CTM addresses
    if ctm.governance.is_some() {
        count += 1;
    }
    if ctm.chain_admin.is_some() {
        count += 1;
    }
    if ctm.proxy_admin.is_some() {
        count += 1;
    }
    if ctm.state_transition_proxy_addr.is_some() {
        count += 1;
    }
    if ctm.validator_timelock_addr.is_some() {
        count += 1;
    }
    if ctm.server_notifier_proxy_addr.is_some() {
        count += 1;
    }
    if ctm.verifier_addr.is_some() {
        count += 1;
    }
    if ctm.l1_rollup_da_manager.is_some() {
        count += 1;
    }
    if ctm.l1_bytecodes_supplier_addr.is_some() {
        count += 1;
    }
    if ctm.l1_wrapped_base_token_store.is_some() {
        count += 1;
    }
    if ctm.default_upgrade_addr.is_some() {
        count += 1;
    }
    if ctm.genesis_upgrade_addr.is_some() {
        count += 1;
    }
    if ctm.rollup_l1_da_validator_addr.is_some() {
        count += 1;
    }
    if ctm.no_da_validium_l1_validator_addr.is_some() {
        count += 1;
    }
    if ctm.blobs_zksync_os_l1_da_validator_addr.is_some() {
        count += 1;
    }
    if ctm.avail_l1_da_validator_addr.is_some() {
        count += 1;
    }

    // Diamond facets
    if ctm.admin_facet_addr.is_some() {
        count += 1;
    }
    if ctm.executor_facet_addr.is_some() {
        count += 1;
    }
    if ctm.mailbox_facet_addr.is_some() {
        count += 1;
    }
    if ctm.getters_facet_addr.is_some() {
        count += 1;
    }
    if ctm.diamond_init_addr.is_some() {
        count += 1;
    }

    // Implementation contracts
    if ctm.bridgehub_impl_addr.is_some() {
        count += 1;
    }
    if ctm.message_root_impl_addr.is_some() {
        count += 1;
    }
    if ctm.native_token_vault_impl_addr.is_some() {
        count += 1;
    }
    if ctm.stm_deployment_tracker_impl_addr.is_some() {
        count += 1;
    }
    if ctm.chain_type_manager_impl_addr.is_some() {
        count += 1;
    }
    if ctm.server_notifier_impl_addr.is_some() {
        count += 1;
    }
    if ctm.erc20_bridge_impl_addr.is_some() {
        count += 1;
    }
    if ctm.shared_bridge_impl_addr.is_some() {
        count += 1;
    }
    if ctm.l1_nullifier_impl_addr.is_some() {
        count += 1;
    }
    if ctm.validator_timelock_impl_addr.is_some() {
        count += 1;
    }

    // Verifier components
    if ctm.verifier_fflonk_addr.is_some() {
        count += 1;
    }
    if ctm.verifier_plonk_addr.is_some() {
        count += 1;
    }

    // Bridge token contracts
    if ctm.bridged_standard_erc20_addr.is_some() {
        count += 1;
    }
    if ctm.bridged_token_beacon_addr.is_some() {
        count += 1;
    }

    // Avail test contracts
    if ctm.dummy_avail_bridge_addr.is_some() {
        count += 1;
    }
    if ctm.dummy_vector_x_addr.is_some() {
        count += 1;
    }

    // Server notifier proxy admin
    if ctm.server_notifier_proxy_admin_addr.is_some() {
        count += 1;
    }

    count
}

/// Count unique chain-specific contract addresses.
///
/// Only counts addresses that are unique to the chain (not ecosystem references).
fn count_chain_contracts(contracts: &ChainContracts) -> usize {
    let mut count = 0;

    // L1 contracts (chain-specific)
    if let Some(ref l1) = contracts.l1 {
        if l1.diamond_proxy_addr.is_some() {
            count += 1;
        }
        if l1.governance_addr.is_some() {
            count += 1;
        }
        if l1.chain_admin_addr.is_some() {
            count += 1;
        }
        if l1.chain_proxy_admin_addr.is_some() {
            count += 1;
        }
    }

    // L2 contracts
    if let Some(ref l2) = contracts.l2 {
        if l2.testnet_paymaster_addr.is_some() {
            count += 1;
        }
        if l2.default_l2_upgrader.is_some() {
            count += 1;
        }
        if l2.l2_native_token_vault_proxy_addr.is_some() {
            count += 1;
        }
        if l2.consensus_registry.is_some() {
            count += 1;
        }
        if l2.multicall3.is_some() {
            count += 1;
        }
        if l2.timestamp_asserter_addr.is_some() {
            count += 1;
        }
    }

    count
}

/// Format ecosystem contracts for display.
fn format_ecosystem_contracts(contracts: &EcosystemContracts) -> String {
    let mut lines = Vec::new();

    lines.push(format_addr(
        "Create2 Factory",
        contracts.create2_factory_addr,
    ));
    lines.push(format_hash("Create2 Salt", contracts.create2_factory_salt));
    lines.push(format_addr("Multicall3", contracts.multicall3_addr));

    if let Some(ref core) = contracts.core_ecosystem_contracts {
        lines.push(String::new());
        lines.push("Core Ecosystem:".to_string());
        lines.extend(format_core(core));
    }

    if let Some(ref bridges) = contracts.bridges {
        lines.push(String::new());
        lines.push("Bridges:".to_string());
        lines.extend(format_bridges(bridges));
    }

    if let Some(ref l1) = contracts.l1 {
        lines.push(String::new());
        lines.push("L1 Contracts:".to_string());
        lines.extend(format_l1(l1));
    }

    if let Some(ref ctm) = contracts.zksync_os_ctm {
        lines.push(String::new());
        lines.push("ZkSync OS CTM:".to_string());
        lines.extend(format_ctm(ctm));
    }

    lines.join("\n")
}

/// Format chain L1 contracts section.
fn format_chain_l1(l1: &ChainL1Contracts) -> Vec<String> {
    vec![
        format!("  {}", format_addr("Diamond Proxy", l1.diamond_proxy_addr)),
        format!(
            "  {}",
            format_addr("Default Upgrade", l1.default_upgrade_addr)
        ),
        format!("  {}", format_addr("Governance", l1.governance_addr)),
        format!("  {}", format_addr("Chain Admin", l1.chain_admin_addr)),
        format!(
            "  {}",
            format_addr(
                "Access Control Restriction",
                l1.access_control_restriction_addr
            )
        ),
        format!(
            "  {}",
            format_addr("Chain Proxy Admin", l1.chain_proxy_admin_addr)
        ),
        format!("  {}", format_addr("Multicall3", l1.multicall3_addr)),
        format!("  {}", format_addr("Verifier", l1.verifier_addr)),
        format!(
            "  {}",
            format_addr("Validator Timelock", l1.validator_timelock_addr)
        ),
        format!("  {}", format_addr("Base Token", l1.base_token_addr)),
        format!(
            "  {}",
            format_hash("Base Token Asset ID", l1.base_token_asset_id)
        ),
        format!(
            "  {}",
            format_addr("Rollup L1 DA Validator", l1.rollup_l1_da_validator_addr)
        ),
        format!(
            "  {}",
            format_addr("Avail L1 DA Validator", l1.avail_l1_da_validator_addr)
        ),
        format!(
            "  {}",
            format_addr(
                "No DA Validium L1 Validator",
                l1.no_da_validium_l1_validator_addr
            )
        ),
        format!(
            "  {}",
            format_addr(
                "Blobs ZkSync OS L1 DA Validator",
                l1.blobs_zksync_os_l1_da_validator_addr
            )
        ),
    ]
}

/// Format chain ecosystem contracts (extended version with CTM fields).
fn format_chain_ecosystem_contracts(eco: &ChainEcosystemContracts) -> Vec<String> {
    vec![
        // Core fields
        format!(
            "  {}",
            format_addr("Bridgehub Proxy", eco.bridgehub_proxy_addr)
        ),
        format!(
            "  {}",
            format_addr("Message Root Proxy", eco.message_root_proxy_addr)
        ),
        format!(
            "  {}",
            format_addr("Proxy Admin", eco.transparent_proxy_admin_addr)
        ),
        format!(
            "  {}",
            format_addr(
                "STM Deployment Tracker",
                eco.stm_deployment_tracker_proxy_addr
            )
        ),
        format!(
            "  {}",
            format_addr("Native Token Vault", eco.native_token_vault_addr)
        ),
        // CTM fields
        format!("  {}", format_addr("Governance", eco.governance)),
        format!("  {}", format_addr("Chain Admin", eco.chain_admin)),
        format!("  {}", format_addr("CTM Proxy Admin", eco.proxy_admin)),
        format!(
            "  {}",
            format_addr("State Transition Proxy", eco.state_transition_proxy_addr)
        ),
        format!(
            "  {}",
            format_addr("Validator Timelock", eco.validator_timelock_addr)
        ),
        format!(
            "  {}",
            format_addr("Server Notifier", eco.server_notifier_proxy_addr)
        ),
        format!(
            "  {}",
            format_addr("Default Upgrade", eco.default_upgrade_addr)
        ),
        format!(
            "  {}",
            format_addr("Genesis Upgrade", eco.genesis_upgrade_addr)
        ),
        format!("  {}", format_addr("Verifier", eco.verifier_addr)),
        format!(
            "  {}",
            format_addr("L1 Bytecodes Supplier", eco.l1_bytecodes_supplier_addr)
        ),
        format!(
            "  {}",
            format_addr(
                "L1 Wrapped Base Token Store",
                eco.l1_wrapped_base_token_store
            )
        ),
        format!(
            "  {}",
            format_addr("L1 Rollup DA Manager", eco.l1_rollup_da_manager)
        ),
        format!(
            "  {}",
            format_addr("Rollup L1 DA Validator", eco.rollup_l1_da_validator_addr)
        ),
        format!(
            "  {}",
            format_addr(
                "No DA Validium L1 Validator",
                eco.no_da_validium_l1_validator_addr
            )
        ),
        format!(
            "  {}",
            format_addr(
                "Blobs ZkSync OS L1 DA Validator",
                eco.blobs_zksync_os_l1_da_validator_addr
            )
        ),
        format!(
            "  {}",
            format_addr("Avail L1 DA Validator", eco.avail_l1_da_validator_addr)
        ),
    ]
}

/// Format chain L2 contracts section.
fn format_chain_l2(l2: &ChainL2Contracts) -> Vec<String> {
    vec![
        format!(
            "  {}",
            format_addr("Testnet Paymaster", l2.testnet_paymaster_addr)
        ),
        format!(
            "  {}",
            format_addr("Default L2 Upgrader", l2.default_l2_upgrader)
        ),
        format!(
            "  {}",
            format_addr(
                "L2 Native Token Vault Proxy",
                l2.l2_native_token_vault_proxy_addr
            )
        ),
        format!(
            "  {}",
            format_addr("Consensus Registry", l2.consensus_registry)
        ),
        format!("  {}", format_addr("Multicall3", l2.multicall3)),
        format!(
            "  {}",
            format_addr("Timestamp Asserter", l2.timestamp_asserter_addr)
        ),
    ]
}

/// Format chain contracts for display.
fn format_chain_contracts(contracts: &ChainContracts) -> String {
    let mut lines = Vec::new();

    lines.push(format_addr(
        "Create2 Factory",
        contracts.create2_factory_addr,
    ));
    lines.push(format_hash("Create2 Salt", contracts.create2_factory_salt));

    if let Some(ref eco) = contracts.ecosystem_contracts {
        lines.push(String::new());
        lines.push("Ecosystem Contracts (reference):".to_string());
        lines.extend(format_chain_ecosystem_contracts(eco));
    }

    if let Some(ref bridges) = contracts.bridges {
        lines.push(String::new());
        lines.push("Bridges:".to_string());
        lines.extend(format_bridges(bridges));
    }

    if let Some(ref l1) = contracts.l1 {
        lines.push(String::new());
        lines.push("L1 Contracts:".to_string());
        lines.extend(format_chain_l1(l1));
    }

    if let Some(ref l2) = contracts.l2 {
        lines.push(String::new());
        lines.push("L2 Contracts:".to_string());
        lines.extend(format_chain_l2(l2));
    }

    lines.join("\n")
}

// ============================================================================
// Main command execution
// ============================================================================

/// Execute the ecosystem command.
pub async fn run(args: &EcosystemArgs, context: &Context) -> Result<()> {
    ui::intro("ADI CLI")?;

    let ecosystem_name = resolve_ecosystem_name(args.ecosystem_name.as_ref(), context.config())?;
    let state_manager = create_state_manager_with_context(&ecosystem_name, context);

    // Check if ecosystem exists
    if !ecosystem_exists(&state_manager).await? {
        ui::warning(format!(
            "Ecosystem '{}' not found.\nRun 'adi init' first to initialize the ecosystem.",
            ecosystem_name
        ))?;
        ui::outro("")?;
        return Ok(());
    }

    // Load and display ecosystem metadata
    let eco_metadata = state_manager
        .ecosystem()
        .metadata()
        .await
        .wrap_err("Failed to load ecosystem metadata")?;

    let initial_deployments = state_manager.ecosystem().initial_deployments().await.ok();

    let meta_display = format_ecosystem_metadata(&eco_metadata, initial_deployments.as_ref());
    ui::note(
        format!("Ecosystem '{}'", ui::green(&ecosystem_name)),
        meta_display,
    )?;

    // Load and display ecosystem contracts if they exist
    let contract_count = if contracts_exist(&state_manager).await? {
        let ecosystem_contracts = state_manager
            .ecosystem()
            .contracts()
            .await
            .wrap_err("Failed to load ecosystem contracts")?;

        let count = count_ecosystem_contracts(&ecosystem_contracts);
        let contracts_display = format_ecosystem_contracts(&ecosystem_contracts);
        ui::note("Contracts", contracts_display)?;
        Some(count)
    } else {
        ui::info("No contracts deployed yet. Run 'adi deploy' to deploy.")?;
        None
    };

    // Load and display chain information
    // Use --chain if provided, otherwise use default_chain from ecosystem metadata
    let chain_to_display = args.chain.as_ref().unwrap_or(&eco_metadata.default_chain);
    let chain_contract_count = display_chain_info(&state_manager, chain_to_display).await?;

    // Display summary (ecosystem + chain contracts)
    let summary = match (contract_count, chain_contract_count) {
        (Some(eco), Some(chain)) => {
            format!(
                "Total contracts: {} (ecosystem: {}, chain: {})",
                eco + chain,
                eco,
                chain
            )
        }
        (Some(eco), None) => format!("Total contracts: {}", eco),
        _ => "No contracts deployed".to_string(),
    };
    ui::outro(summary)?;
    Ok(())
}

/// Display chain information including metadata and contracts.
///
/// Returns the count of unique chain contracts if they exist.
async fn display_chain_info(
    state_manager: &StateManager,
    chain_name: &str,
) -> Result<Option<usize>> {
    let chain_ops = state_manager.chain(chain_name);

    // Check if chain exists
    if !chain_ops.exists().await? {
        ui::warning(format!("Chain '{}' not found.", chain_name))?;
        return Ok(None);
    }

    // Load and display chain metadata
    let chain_metadata = chain_ops
        .metadata()
        .await
        .wrap_err("Failed to load chain metadata")?;

    let meta_display = format_chain_metadata(&chain_metadata);
    ui::note(format!("Chain '{}'", ui::green(chain_name)), meta_display)?;

    // Load and display chain contracts if they exist
    if chain_ops.contracts_exist().await? {
        let chain_contracts = chain_ops
            .contracts()
            .await
            .wrap_err("Failed to load chain contracts")?;

        let count = count_chain_contracts(&chain_contracts);
        let contracts_display = format_chain_contracts(&chain_contracts);
        ui::note(
            format!("Chain '{}' Contracts", chain_name),
            contracts_display,
        )?;
        Ok(Some(count))
    } else {
        ui::info(format!(
            "No contracts deployed for chain '{}' yet.",
            chain_name
        ))?;
        Ok(None)
    }
}

/// Check if ecosystem metadata exists.
async fn ecosystem_exists(state_manager: &StateManager) -> Result<bool> {
    state_manager
        .exists()
        .await
        .wrap_err("Failed to check if ecosystem exists")
}

/// Check if ecosystem contracts exist.
async fn contracts_exist(state_manager: &StateManager) -> Result<bool> {
    state_manager
        .ecosystem()
        .contracts_exist()
        .await
        .wrap_err("Failed to check if contracts exist")
}
