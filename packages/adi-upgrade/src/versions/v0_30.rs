//! Version handlers for v0.30.x protocol versions.

use super::{PostUpgradeHook, VersionHandler};
use crate::chain_toml::{
    ChainTomlConfig, ContractsSection, GatewaySection, GatewayStateTransitionSection,
    StateTransitionSection, TokensSection, ZkSyncOsSection,
};

/// Handler for v0.30.1 upgrades.
pub struct V0_30_1Handler;

impl VersionHandler for V0_30_1Handler {
    fn upgrade_script(&self) -> &str {
        "EcosystemUpgrade_v30_1_zk_os.s.sol"
    }

    fn upgrade_env_dir(&self) -> &str {
        "upgrade-envs/v0.30.1-airbender-fix"
    }

    fn upgrade_output_toml(&self) -> &str {
        "v30.1-ecosystem-upgrade-output.toml"
    }

    fn upgrade_output_yaml(&self) -> &str {
        "v30.1-ecosystem.yaml"
    }

    fn upgrade_name(&self) -> &str {
        "v30-zk-sync-os-blobs"
    }

    fn old_protocol_version_hex(&self) -> &str {
        "0x1e00000000"
    }

    fn previous_upgrade_yaml(&self) -> &str {
        "v0.30.0-ecosystem.yaml"
    }

    fn post_upgrade_hooks(&self) -> Vec<PostUpgradeHook> {
        // v0.30.1 has no post-upgrade hooks
        vec![]
    }

    fn chain_toml_defaults(&self) -> ChainTomlConfig {
        ChainTomlConfig {
            era_chain_id: 0, // Set dynamically
            testnet_verifier: false,
            governance_upgrade_timer_initial_delay: 0,
            owner_address: String::new(), // Set dynamically
            support_l2_legacy_shared_bridge_test: false,
            old_protocol_version: "0x1e00000000".to_string(),
            priority_txs_l2_gas_limit: 2000000,
            max_expected_l1_gas_price: 30000000000,
            is_zk_sync_os: true,
            redeploy_da_manager: true,
            contracts: ContractsSection {
                governance_min_delay: 0,
                max_number_of_chains: 100,
                create2_factory_salt: String::new(), // Set dynamically
                create2_factory_addr: String::new(), // Set dynamically
                validator_timelock_execution_delay: 0,
                genesis_root: "0x423c107626aff95d3d086eabd92132dc9485e021ae3cb4c7735d5e963578e3d0"
                    .to_string(),
                genesis_rollup_leaf_index: 0,
                genesis_batch_commitment:
                    "0000000000000000000000000000000000000000000000000000000000000001".to_string(),
                recursion_node_level_vk_hash:
                    "0x0000000000000000000000000000000000000000000000000000000000000000".to_string(),
                recursion_leaf_level_vk_hash:
                    "0x0000000000000000000000000000000000000000000000000000000000000000".to_string(),
                recursion_circuits_set_vks_hash:
                    "0x0000000000000000000000000000000000000000000000000000000000000000".to_string(),
                priority_tx_max_gas_limit: 72000000,
                diamond_init_pubdata_pricing_mode: 0,
                diamond_init_batch_overhead_l1_gas: 1000000,
                diamond_init_max_pubdata_per_batch: 120000,
                diamond_init_max_l2_gas_per_batch: 80000000,
                diamond_init_priority_tx_max_pubdata: 99000,
                diamond_init_minimal_l2_gas_price: 250000000,
                bootloader_hash:
                    "0x0000000000000000000000000000000000000000000000000000000000000001".to_string(),
                default_aa_hash:
                    "0x0000000000000000000000000000000000000000000000000000000000000001".to_string(),
                evm_emulator_hash:
                    "0x0000000000000000000000000000000000000000000000000000000000000001".to_string(),
                bridgehub_proxy_address: String::new(), // Set dynamically
                rollup_da_manager: "0xE689e79a06D3D09f99C21E534cCF6a8b7C9b3C45".to_string(),
                governance_security_council_address: "0xed04b1ac422251851a3EC953Ff4395e5c2443647"
                    .to_string(),
                latest_protocol_version: "0x1e00000001".to_string(),
                l1_bytecodes_supplier_addr: "0xC9F20FC268Fc3e0e597660550033Bf2C24218fd8"
                    .to_string(),
                protocol_upgrade_handler_proxy_address:
                    "0xE30Dca3047B37dc7d88849dE4A4Dc07937ad5Ab3".to_string(),
                protocol_upgrade_handler_implementation_address:
                    "0x36625Bd3dDB469377C6e9893712158cA3c0cC14B".to_string(),
            },
            tokens: TokensSection {
                token_weth_address: "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2".to_string(),
            },
            gateway: GatewaySection {
                chain_id: 0, // Set dynamically
                gateway_state_transition: GatewayStateTransitionSection {
                    chain_type_manager_proxy_addr: "0x0000000000000000000000000000000000000000"
                        .to_string(),
                    rollup_da_manager: "0x0000000000000000000000000000000000000000".to_string(),
                    chain_type_manager_proxy_admin: "0x0000000000000000000000000000000000000000"
                        .to_string(),
                    rollup_sl_da_validator: "0x0000000000000000000000000000000000000000"
                        .to_string(),
                },
            },
            state_transition: StateTransitionSection {
                admin_facet_addr: String::new(), // Set from previous upgrade
                diamond_init_addr: String::new(),
                executor_facet_addr: String::new(),
                genesis_upgrade_addr: String::new(),
                getters_facet_addr: String::new(),
                mailbox_facet_addr: String::new(),
                force_deployments_data: String::new(),
            },
            zksync_os: ZkSyncOsSection {
                sample_chain_id: 0,                   // Set dynamically
                optional_ctm_address: String::new(),  // Set dynamically
                current_dual_verifier: String::new(), // Set dynamically
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::versions::VersionHandler;

    #[test]
    fn test_v0_30_1_handler_values() {
        let handler = V0_30_1Handler;
        assert_eq!(
            handler.upgrade_script(),
            "EcosystemUpgrade_v30_1_zk_os.s.sol"
        );
        assert!(!handler.upgrade_env_dir().is_empty());
        assert!(!handler.upgrade_output_toml().is_empty());
        assert!(!handler.upgrade_output_yaml().is_empty());
        assert!(!handler.upgrade_name().is_empty());
    }

    #[test]
    fn test_v0_30_1_handler_defaults() {
        let handler = V0_30_1Handler;
        let defaults = handler.chain_toml_defaults();
        assert!(defaults.is_zk_sync_os);
        assert!(defaults.redeploy_da_manager);
        assert_eq!(defaults.old_protocol_version, "0x1e00000000");
        assert_eq!(defaults.contracts.latest_protocol_version, "0x1e00000001");
    }
}
