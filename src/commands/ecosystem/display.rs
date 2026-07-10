//! Formatting functions for ecosystem and chain contract display.

use adi_types::{
    BaseToken, BatchCommitDataMode, BridgesConfig, ChainContracts, ChainEcosystemContracts,
    ChainL1Contracts, ChainL2Contracts, ChainMetadata, CoreEcosystemContracts, EcosystemContracts,
    EcosystemMetadata, InitialDeployments, L1Contracts, ProverMode, VmOption, ZkSyncOsCtm,
};
use alloy_primitives::{Address, B256};

use crate::ui;

// ============================================================================
// Value formatting helpers
// ============================================================================

/// Format an optional address field with green color.
///
/// A missing address or the zero address (`0x0000…0000`, meaning not deployed)
/// is rendered as "not set".
pub(super) fn format_addr(name: &str, addr: Option<Address>) -> String {
    match addr {
        Some(a) if !a.is_zero() => format!("{}: {}", name, ui::green(a)),
        _ => format!("{}: {}", name, ui::cyan("not set")),
    }
}

/// Format an optional hash field with green color.
///
/// A missing hash or the zero hash is rendered as "not set".
pub(super) fn format_hash(name: &str, hash: Option<B256>) -> String {
    match hash {
        Some(h) if !h.is_zero() => format!("{}: {}", name, ui::green(h)),
        _ => format!("{}: {}", name, ui::cyan("not set")),
    }
}

/// Format a value with green color.
pub(super) fn format_val<T: std::fmt::Display>(name: &str, val: T) -> String {
    format!("{}: {}", name, ui::green(val))
}

fn has_any_addr(addrs: &[Option<Address>]) -> bool {
    addrs
        .iter()
        .any(|addr| matches!(addr, Some(addr) if !addr.is_zero()))
}

fn append_section(lines: &mut Vec<String>, title: &str, rows: Vec<String>) {
    if rows.is_empty() {
        return;
    }

    lines.push(String::new());
    lines.push(format!("  {}:", title));
    lines.extend(rows);
}

// ============================================================================
// Metadata formatting
// ============================================================================

/// Format ecosystem metadata for display.
pub(super) fn format_ecosystem_metadata(
    meta: &EcosystemMetadata,
    deployments: Option<&InitialDeployments>,
) -> String {
    let mut lines = vec![
        format_val("L1 Network", meta.l1_network),
        format_val("Era Chain ID", meta.era_chain_id),
        format_val("Prover Mode", format_prover_mode(meta.prover_version)),
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
pub(super) fn format_chain_metadata(meta: &ChainMetadata) -> String {
    let base_token_display = format_base_token(&meta.base_token);

    [
        format_val("Chain ID", meta.chain_id),
        format_val("L1 Network", meta.l1_network),
        format_val("Prover Mode", format_prover_mode(meta.prover_version)),
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
pub(super) fn format_prover_mode(mode: ProverMode) -> &'static str {
    match mode {
        ProverMode::NoProofs => "NoProofs",
        ProverMode::Gpu => "GPU",
    }
}

/// Format batch commit data mode for display.
pub(super) fn format_batch_mode(mode: &BatchCommitDataMode) -> &'static str {
    match mode {
        BatchCommitDataMode::Rollup => "Rollup",
        BatchCommitDataMode::Validium => "Validium",
    }
}

/// Format VM option for display.
pub(super) fn format_vm_option(opt: &VmOption) -> &'static str {
    match opt {
        VmOption::ZKSyncOsVM => "ZKSyncOsVM",
        VmOption::Evm => "EVM",
    }
}

/// Format base token for display.
pub(super) fn format_base_token(token: &BaseToken) -> String {
    if token.is_eth() {
        "ETH".to_string()
    } else {
        format!("{}", token.address)
    }
}

// ============================================================================
// Contract formatting
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
pub(super) fn format_bridges(bridges: &BridgesConfig) -> Vec<String> {
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

/// Format ecosystem-owned ZkSync OS CTM fields.
fn format_ecosystem_ctm(ctm: &ZkSyncOsCtm) -> Vec<String> {
    let mut lines = vec![
        format!("  {}", format_addr("Governance", ctm.governance)),
        format!("  {}", format_addr("Chain Admin", ctm.chain_admin)),
        format!("  {}", format_addr("Proxy Admin", ctm.proxy_admin)),
        format!(
            "  {}",
            format_addr(
                "L1 Wrapped Base Token Store",
                ctm.l1_wrapped_base_token_store
            )
        ),
    ];

    if has_any_addr(&[
        ctm.bridgehub_impl_addr,
        ctm.message_root_impl_addr,
        ctm.native_token_vault_impl_addr,
        ctm.stm_deployment_tracker_impl_addr,
        ctm.erc20_bridge_impl_addr,
        ctm.shared_bridge_impl_addr,
        ctm.l1_nullifier_impl_addr,
    ]) {
        append_section(
            &mut lines,
            "Implementation Contracts",
            vec![
                format!(
                    "    {}",
                    format_addr("Bridgehub Impl", ctm.bridgehub_impl_addr)
                ),
                format!(
                    "    {}",
                    format_addr("Message Root Impl", ctm.message_root_impl_addr)
                ),
                format!(
                    "    {}",
                    format_addr("Native Token Vault Impl", ctm.native_token_vault_impl_addr)
                ),
                format!(
                    "    {}",
                    format_addr(
                        "STM Deployment Tracker Impl",
                        ctm.stm_deployment_tracker_impl_addr
                    )
                ),
                format!(
                    "    {}",
                    format_addr("ERC20 Bridge Impl", ctm.erc20_bridge_impl_addr)
                ),
                format!(
                    "    {}",
                    format_addr("Shared Bridge Impl", ctm.shared_bridge_impl_addr)
                ),
                format!(
                    "    {}",
                    format_addr("L1 Nullifier Impl", ctm.l1_nullifier_impl_addr)
                ),
            ],
        );
    }

    if has_any_addr(&[
        ctm.bridged_standard_erc20_addr,
        ctm.bridged_token_beacon_addr,
    ]) {
        append_section(
            &mut lines,
            "Bridge Token Contracts",
            vec![
                format!(
                    "    {}",
                    format_addr("Bridged Standard ERC20", ctm.bridged_standard_erc20_addr)
                ),
                format!(
                    "    {}",
                    format_addr("Bridged Token Beacon", ctm.bridged_token_beacon_addr)
                ),
            ],
        );
    }

    lines
}

/// Format ecosystem contracts for display.
pub(super) fn format_ecosystem_contracts(contracts: &EcosystemContracts) -> String {
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
        lines.push("Ecosystem L1 Contracts:".to_string());
        lines.extend(format_l1(l1));
    }

    if let Some(ref ctm) = contracts.zksync_os_ctm {
        lines.push(String::new());
        lines.push("ZkSync OS CTM:".to_string());
        lines.extend(format_ecosystem_ctm(ctm));
    }

    lines.join("\n")
}

/// Format chain L1 deployment contracts from the chain config.
fn format_chain_l1_deployments(l1: Option<&ChainL1Contracts>) -> Vec<String> {
    l1.map(|l1| {
        vec![format!(
            "  {}",
            format_addr("Diamond Proxy", l1.diamond_proxy_addr)
        )]
    })
    .unwrap_or_default()
}

/// Format chain L1 deployment contracts copied from zkstack's CTM reference.
fn format_chain_ecosystem_deployments(contracts: &ChainEcosystemContracts) -> Vec<String> {
    vec![
        format!(
            "  {}",
            format_addr(
                "State Transition Proxy",
                contracts.state_transition_proxy_addr
            )
        ),
        format!(
            "  {}",
            format_addr(
                "Validator Timelock Proxy",
                contracts.validator_timelock_addr
            )
        ),
        format!(
            "  {}",
            format_addr(
                "Server Notifier Proxy",
                contracts.server_notifier_proxy_addr
            )
        ),
        format!("  {}", format_addr("Verifier", contracts.verifier_addr)),
        format!(
            "  {}",
            format_addr("Rollup DA Manager", contracts.l1_rollup_da_manager)
        ),
        format!(
            "  {}",
            format_addr(
                "L1 Bytecodes Supplier",
                contracts.l1_bytecodes_supplier_addr
            )
        ),
        format!(
            "  {}",
            format_addr("Default Upgrade", contracts.default_upgrade_addr)
        ),
        format!(
            "  {}",
            format_addr("Genesis Upgrade", contracts.genesis_upgrade_addr)
        ),
        format!(
            "  {}",
            format_addr(
                "Rollup L1 DA Validator",
                contracts.rollup_l1_da_validator_addr
            )
        ),
        format!(
            "  {}",
            format_addr(
                "No DA Validium L1 Validator",
                contracts.no_da_validium_l1_validator_addr
            )
        ),
        format!(
            "  {}",
            format_addr(
                "Blobs ZkSync OS L1 DA Validator",
                contracts.blobs_zksync_os_l1_da_validator_addr
            )
        ),
        format!(
            "  {}",
            format_addr(
                "Avail L1 DA Validator",
                contracts.avail_l1_da_validator_addr
            )
        ),
    ]
}

/// Format chain L1 deployment details derived after deployment.
fn format_chain_ctm_deployments(ctm: &ZkSyncOsCtm) -> Vec<String> {
    let mut lines = Vec::new();

    if has_any_addr(&[
        ctm.admin_facet_addr,
        ctm.executor_facet_addr,
        ctm.mailbox_facet_addr,
        ctm.getters_facet_addr,
        ctm.diamond_init_addr,
    ]) {
        append_section(
            &mut lines,
            "Diamond Facets",
            vec![
                format!("    {}", format_addr("Admin Facet", ctm.admin_facet_addr)),
                format!(
                    "    {}",
                    format_addr("Executor Facet", ctm.executor_facet_addr)
                ),
                format!(
                    "    {}",
                    format_addr("Mailbox Facet", ctm.mailbox_facet_addr)
                ),
                format!(
                    "    {}",
                    format_addr("Getters Facet", ctm.getters_facet_addr)
                ),
                format!("    {}", format_addr("Diamond Init", ctm.diamond_init_addr)),
            ],
        );
    }

    if has_any_addr(&[
        ctm.chain_type_manager_impl_addr,
        ctm.server_notifier_impl_addr,
        ctm.validator_timelock_impl_addr,
    ]) {
        append_section(
            &mut lines,
            "Chain Implementation Contracts",
            vec![
                format!(
                    "    {}",
                    format_addr("Chain Type Manager Impl", ctm.chain_type_manager_impl_addr)
                ),
                format!(
                    "    {}",
                    format_addr("Server Notifier Impl", ctm.server_notifier_impl_addr)
                ),
                format!(
                    "    {}",
                    format_addr("Validator Timelock Impl", ctm.validator_timelock_impl_addr)
                ),
            ],
        );
    }

    if has_any_addr(&[ctm.verifier_fflonk_addr, ctm.verifier_plonk_addr]) {
        append_section(
            &mut lines,
            "Verifier Components",
            vec![
                format!(
                    "    {}",
                    format_addr("Verifier Fflonk", ctm.verifier_fflonk_addr)
                ),
                format!(
                    "    {}",
                    format_addr("Verifier Plonk", ctm.verifier_plonk_addr)
                ),
            ],
        );
    }

    if has_any_addr(&[ctm.dummy_avail_bridge_addr, ctm.dummy_vector_x_addr]) {
        append_section(
            &mut lines,
            "Optional Chain Helpers",
            vec![
                format!(
                    "    {}",
                    format_addr("Dummy Avail Bridge", ctm.dummy_avail_bridge_addr)
                ),
                format!(
                    "    {}",
                    format_addr("Dummy VectorX", ctm.dummy_vector_x_addr)
                ),
            ],
        );
    }

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

/// Format chain config and role addresses that are useful but not deployment contracts.
fn format_chain_config_roles(l1: &ChainL1Contracts) -> Vec<String> {
    vec![
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
        format!("  {}", format_addr("Base Token", l1.base_token_addr)),
        format!(
            "  {}",
            format_hash("Base Token Asset ID", l1.base_token_asset_id)
        ),
        format!(
            "  {}",
            format_addr("Fee Adjuster Config", l1.fee_adjuster_config)
        ),
        format!(
            "  {}",
            format_addr("Nox Transaction Filterer", l1.nox_transaction_filterer_addr)
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

fn has_chain_l1_section(contracts: &ChainContracts, ctm: Option<&ZkSyncOsCtm>) -> bool {
    [
        contracts.l1.is_some(),
        contracts.ecosystem_contracts.is_some(),
        ctm.is_some(),
    ]
    .into_iter()
    .any(std::convert::identity)
}

/// Format chain contracts with optional derived CTM fields from ecosystem state.
pub(super) fn format_chain_contracts_with_ctm(
    contracts: &ChainContracts,
    ctm: Option<&ZkSyncOsCtm>,
) -> String {
    let mut lines = Vec::new();

    if has_chain_l1_section(contracts, ctm) {
        lines.push("Chain L1 Contracts:".to_string());
        lines.extend(format_chain_l1_deployments(contracts.l1.as_ref()));
        if let Some(ref ecosystem_contracts) = contracts.ecosystem_contracts {
            lines.extend(format_chain_ecosystem_deployments(ecosystem_contracts));
        }
        if let Some(ctm) = ctm {
            lines.extend(format_chain_ctm_deployments(ctm));
        }
    }

    if let Some(ref l1) = contracts.l1 {
        if !lines.is_empty() {
            lines.push(String::new());
        }
        lines.push("Chain Config / Roles:".to_string());
        lines.extend(format_chain_config_roles(l1));
    }

    if let Some(ref l2) = contracts.l2 {
        if !lines.is_empty() {
            lines.push(String::new());
        }
        lines.push("Chain L2 Contracts:".to_string());
        lines.extend(format_chain_l2(l2));
    }

    lines.join("\n")
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use adi_types::{ChainEcosystemContracts, ZkSyncOsCtm};
    use alloy_primitives::{address, b256};

    #[test]
    fn ecosystem_contracts_omit_chain_ctm_deployment_artifacts() {
        let contracts = EcosystemContracts {
            zksync_os_ctm: Some(ZkSyncOsCtm {
                governance: Some(address!("0000000000000000000000000000000000000001")),
                chain_admin: Some(address!("0000000000000000000000000000000000000002")),
                proxy_admin: Some(address!("0000000000000000000000000000000000000003")),
                bridgehub_impl_addr: Some(address!("0000000000000000000000000000000000000004")),
                bridged_standard_erc20_addr: Some(address!(
                    "0000000000000000000000000000000000000005"
                )),
                state_transition_proxy_addr: Some(address!(
                    "0000000000000000000000000000000000000011"
                )),
                validator_timelock_addr: Some(address!("0000000000000000000000000000000000000012")),
                l1_rollup_da_manager: Some(address!("0000000000000000000000000000000000000013")),
                admin_facet_addr: Some(address!("0000000000000000000000000000000000000014")),
                chain_type_manager_impl_addr: Some(address!(
                    "0000000000000000000000000000000000000015"
                )),
                verifier_fflonk_addr: Some(address!("0000000000000000000000000000000000000016")),
                server_notifier_proxy_admin_addr: Some(address!(
                    "0000000000000000000000000000000000000017"
                )),
                ..Default::default()
            }),
            ..Default::default()
        };

        let output = format_ecosystem_contracts(&contracts);

        assert!(output.contains("Governance"));
        assert!(output.contains("Bridgehub Impl"));
        assert!(output.contains("Bridged Standard ERC20"));
        assert!(!output.contains("State Transition Proxy"));
        assert!(!output.contains("Validator Timelock"));
        assert!(!output.contains("L1 Rollup DA Manager"));
        assert!(!output.contains("Diamond Facets"));
        assert!(!output.contains("Chain Type Manager Impl"));
        assert!(!output.contains("Verifier Fflonk"));
        assert!(!output.contains("Server Notifier Proxy Admin"));
    }

    #[test]
    fn chain_contracts_group_l1_deployments_roles_and_l2_separately() {
        let contracts = ChainContracts {
            ecosystem_contracts: Some(ChainEcosystemContracts {
                state_transition_proxy_addr: Some(address!(
                    "0000000000000000000000000000000000000011"
                )),
                validator_timelock_addr: Some(address!("0000000000000000000000000000000000000012")),
                verifier_addr: Some(address!("0000000000000000000000000000000000000013")),
                l1_rollup_da_manager: Some(address!("0000000000000000000000000000000000000014")),
                l1_bytecodes_supplier_addr: Some(address!(
                    "0000000000000000000000000000000000000015"
                )),
                default_upgrade_addr: Some(address!("0000000000000000000000000000000000000016")),
                genesis_upgrade_addr: Some(address!("0000000000000000000000000000000000000017")),
                rollup_l1_da_validator_addr: Some(address!(
                    "0000000000000000000000000000000000000018"
                )),
                no_da_validium_l1_validator_addr: Some(address!(
                    "0000000000000000000000000000000000000019"
                )),
                avail_l1_da_validator_addr: Some(address!(
                    "000000000000000000000000000000000000001a"
                )),
                ..Default::default()
            }),
            l1: Some(ChainL1Contracts {
                diamond_proxy_addr: Some(address!("0000000000000000000000000000000000000021")),
                governance_addr: Some(address!("0000000000000000000000000000000000000022")),
                chain_admin_addr: Some(address!("0000000000000000000000000000000000000023")),
                base_token_addr: Some(address!("0000000000000000000000000000000000000024")),
                base_token_asset_id: Some(b256!(
                    "0000000000000000000000000000000000000000000000000000000000000031"
                )),
                ..Default::default()
            }),
            l2: Some(ChainL2Contracts {
                default_l2_upgrader: Some(address!("0000000000000000000000000000000000000041")),
                ..Default::default()
            }),
            ..Default::default()
        };

        let output = format_chain_contracts_with_ctm(&contracts, None);

        assert!(output.contains("Chain L1 Contracts:"));
        assert!(output.contains("State Transition Proxy"));
        assert!(output.contains("Validator Timelock Proxy"));
        assert!(output.contains("L1 Bytecodes Supplier"));
        assert!(output.contains("Rollup L1 DA Validator"));
        assert!(output.contains("Chain Config / Roles:"));
        assert!(output.contains("Governance"));
        assert!(output.contains("Base Token Asset ID"));
        assert!(output.contains("Chain L2 Contracts:"));
        assert!(output.contains("Default L2 Upgrader"));
    }

    #[test]
    fn chain_contracts_include_derived_ctm_deployments_when_available() {
        let contracts = ChainContracts {
            l1: Some(ChainL1Contracts {
                diamond_proxy_addr: Some(address!("0000000000000000000000000000000000000021")),
                ..Default::default()
            }),
            ..Default::default()
        };
        let ctm = ZkSyncOsCtm {
            admin_facet_addr: Some(address!("0000000000000000000000000000000000000031")),
            chain_type_manager_impl_addr: Some(address!(
                "0000000000000000000000000000000000000032"
            )),
            verifier_fflonk_addr: Some(address!("0000000000000000000000000000000000000033")),
            server_notifier_proxy_admin_addr: Some(address!(
                "0000000000000000000000000000000000000034"
            )),
            ..Default::default()
        };

        let output = format_chain_contracts_with_ctm(&contracts, Some(&ctm));

        assert!(output.contains("Diamond Facets:"));
        assert!(output.contains("Admin Facet"));
        assert!(output.contains("Chain Implementation Contracts:"));
        assert!(output.contains("Chain Type Manager Impl"));
        assert!(output.contains("Verifier Components:"));
        assert!(output.contains("Verifier Fflonk"));
        assert!(output.contains("Server Notifier Proxy Admin"));
    }
}
