//! Chain TOML configuration generation for upgrade scripts.
//!
//! Defines typed structs that serialize to chain.toml format required
//! by forge upgrade scripts.

mod types;

use std::path::Path;

use alloy_provider::Provider;

use crate::config::UpgradeConfig;
use crate::error::{Result, UpgradeError};
use crate::onchain;
use crate::versions::VersionHandler;

pub use types::{
    ChainTomlConfig, ContractsSection, GatewaySection, GatewayStateTransitionSection,
    PreviousUpgradeValues, StateTransitionSection, TokensSection, ZkSyncOsSection,
};

/// Generate chain.toml content from handler defaults, state, and on-chain queries.
///
/// # Errors
///
/// Returns [`UpgradeError::Config`] if on-chain queries or serialization fails.
pub async fn generate_chain_toml<P: Provider + Clone>(
    handler: &dyn VersionHandler,
    config: &UpgradeConfig,
    provider: &P,
    chain_id: u64,
    previous_values: &PreviousUpgradeValues,
) -> Result<String> {
    log::info!("Generating chain.toml for upgrade");

    // Get version-specific defaults
    let mut toml_config = handler.chain_toml_defaults();

    // Patch dynamic chain ID values
    toml_config.era_chain_id = chain_id;
    toml_config.gateway.chain_id = chain_id;
    toml_config.zksync_os.sample_chain_id = chain_id;

    // Query on-chain values
    let governance = onchain::query_owner(provider, config.bridgehub_address).await?;
    let ctm = onchain::query_ctm(provider, config.bridgehub_address, chain_id).await?;
    let diamond = onchain::query_zk_chain(provider, config.bridgehub_address, chain_id).await?;
    let verifier = onchain::query_verifier(provider, diamond).await?;
    let is_testnet = onchain::query_is_testnet_verifier(provider, verifier).await;
    log::info!("Detected testnet_verifier = {is_testnet} for verifier {verifier}");

    toml_config.testnet_verifier = is_testnet;
    toml_config.owner_address = governance;
    toml_config.contracts.bridgehub_proxy_address = config.bridgehub_address;
    toml_config.zksync_os.optional_ctm_address = ctm;
    toml_config.zksync_os.current_dual_verifier = verifier;

    // From ecosystem state config
    if let Some(addr) = config.create2_factory_addr {
        toml_config.contracts.create2_factory_addr = addr;
    }
    if let Some(salt) = config.create2_factory_salt {
        toml_config.contracts.create2_factory_salt = salt.to_string();
    }

    // Apply previous upgrade values to state_transition section
    apply_previous_values(&mut toml_config.state_transition, previous_values);

    // Serialize to TOML
    toml::to_string(&toml_config)
        .map_err(|e| UpgradeError::Config(format!("Failed to serialize chain.toml: {e}")))
}

/// Write chain.toml to the upgrade env directory.
///
/// # Errors
///
/// Returns [`UpgradeError::Config`] if directory creation or file write fails.
pub fn write_chain_toml(content: &str, state_dir: &Path, upgrade_env_dir: &str) -> Result<()> {
    let dir = state_dir.join("l1-contracts").join(upgrade_env_dir);
    std::fs::create_dir_all(&dir).map_err(|e| {
        UpgradeError::Config(format!("Failed to create dir {}: {e}", dir.display()))
    })?;

    let path = dir.join("chain.toml");
    log::info!("Writing chain.toml to {}", path.display());
    std::fs::write(&path, content)
        .map_err(|e| UpgradeError::Config(format!("Failed to write chain.toml: {e}")))?;

    Ok(())
}

/// Apply previous upgrade values to the state transition section.
fn apply_previous_values(section: &mut StateTransitionSection, values: &PreviousUpgradeValues) {
    if let Some(v) = values.admin_facet_addr {
        section.admin_facet_addr = v;
    }
    if let Some(v) = values.diamond_init_addr {
        section.diamond_init_addr = v;
    }
    if let Some(v) = values.executor_facet_addr {
        section.executor_facet_addr = v;
    }
    if let Some(v) = values.genesis_upgrade_addr {
        section.genesis_upgrade_addr = v;
    }
    if let Some(v) = values.getters_facet_addr {
        section.getters_facet_addr = v;
    }
    if let Some(v) = values.mailbox_facet_addr {
        section.mailbox_facet_addr = v;
    }
    if let Some(v) = &values.force_deployments_data {
        section.force_deployments_data = v.clone();
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use alloy_primitives::{address, Address};

    use super::*;

    #[test]
    fn test_chain_toml_serializes() {
        let config = ChainTomlConfig {
            era_chain_id: 270,
            testnet_verifier: true,
            governance_upgrade_timer_initial_delay: 0,
            owner_address: address!("0000000000000000000000000000000000001111"),
            support_l2_legacy_shared_bridge_test: false,
            old_protocol_version: "0x1e00000000".to_string(),
            priority_txs_l2_gas_limit: 2000000,
            max_expected_l1_gas_price: 30000000000,
            is_zk_sync_os: true,
            redeploy_da_manager: true,
            contracts: ContractsSection {
                governance_min_delay: 0,
                max_number_of_chains: 100,
                create2_factory_salt: "0x00".to_string(),
                create2_factory_addr: address!("0000000000000000000000000000000000002222"),
                validator_timelock_execution_delay: 0,
                genesis_root: "0xcccc".to_string(),
                genesis_rollup_leaf_index: 0,
                genesis_batch_commitment: "0x01".to_string(),
                recursion_node_level_vk_hash: "0x00".to_string(),
                recursion_leaf_level_vk_hash: "0x00".to_string(),
                recursion_circuits_set_vks_hash: "0x00".to_string(),
                priority_tx_max_gas_limit: 72000000,
                diamond_init_pubdata_pricing_mode: 0,
                diamond_init_batch_overhead_l1_gas: 1000000,
                diamond_init_max_pubdata_per_batch: 120000,
                diamond_init_max_l2_gas_per_batch: 80000000,
                diamond_init_priority_tx_max_pubdata: 99000,
                diamond_init_minimal_l2_gas_price: 250000000,
                bootloader_hash: "0x01".to_string(),
                default_aa_hash: "0x01".to_string(),
                evm_emulator_hash: "0x01".to_string(),
                bridgehub_proxy_address: address!("0000000000000000000000000000000000003333"),
                rollup_da_manager: address!("0000000000000000000000000000000000004444"),
                governance_security_council_address: address!(
                    "0000000000000000000000000000000000005555"
                ),
                latest_protocol_version: "0x1e00000001".to_string(),
                l1_bytecodes_supplier_addr: address!("0000000000000000000000000000000000006666"),
                protocol_upgrade_handler_proxy_address: address!(
                    "0000000000000000000000000000000000007777"
                ),
                protocol_upgrade_handler_implementation_address: address!(
                    "0000000000000000000000000000000000008888"
                ),
            },
            tokens: TokensSection {
                token_weth_address: address!("0000000000000000000000000000000000009999"),
            },
            gateway: GatewaySection {
                chain_id: 270,
                gateway_state_transition: GatewayStateTransitionSection {
                    chain_type_manager_proxy_addr: Address::ZERO,
                    rollup_da_manager: Address::ZERO,
                    chain_type_manager_proxy_admin: Address::ZERO,
                    rollup_sl_da_validator: Address::ZERO,
                },
            },
            state_transition: StateTransitionSection {
                admin_facet_addr: address!("000000000000000000000000000000000000aaaa"),
                diamond_init_addr: address!("000000000000000000000000000000000000bbbb"),
                executor_facet_addr: address!("000000000000000000000000000000000000cccc"),
                genesis_upgrade_addr: address!("000000000000000000000000000000000000dddd"),
                getters_facet_addr: address!("000000000000000000000000000000000000eeee"),
                mailbox_facet_addr: address!("000000000000000000000000000000000000ffff"),
                force_deployments_data: "0xbbbb".to_string(),
            },
            zksync_os: ZkSyncOsSection {
                sample_chain_id: 270,
                optional_ctm_address: address!("0000000000000000000000000000000000001234"),
                current_dual_verifier: address!("0000000000000000000000000000000000005678"),
            },
        };

        let toml_str = toml::to_string(&config);
        assert!(toml_str.is_ok());
        let toml_str = toml_str.unwrap();

        // Verify key sections exist
        assert!(toml_str.contains("era_chain_id = 270"));
        assert!(toml_str.contains("[contracts]"));
        assert!(toml_str.contains("[tokens]"));
        assert!(toml_str.contains("[gateway]"));
        assert!(toml_str.contains("[state_transition]"));
        assert!(toml_str.contains("[zksync_os]"));
        assert!(toml_str
            .contains("bridgehub_proxy_address = \"0x0000000000000000000000000000000000003333\""));
    }
}
