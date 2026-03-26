//! Chain TOML configuration generation for upgrade scripts.
//!
//! Defines typed structs that serialize to chain.toml format required
//! by forge upgrade scripts.

use std::path::Path;

use alloy_provider::Provider;
use serde::Serialize;

use crate::config::UpgradeConfig;
use crate::error::{Result, UpgradeError};
use crate::onchain;
use crate::versions::VersionHandler;

/// Top-level chain.toml configuration.
#[derive(Debug, Clone, Serialize)]
pub struct ChainTomlConfig {
    /// Era chain ID.
    pub era_chain_id: u64,
    /// Whether to use testnet verifier.
    pub testnet_verifier: bool,
    /// Governance upgrade timer initial delay.
    pub governance_upgrade_timer_initial_delay: u64,
    /// Owner address (governance contract).
    pub owner_address: String,
    /// Support L2 legacy shared bridge test.
    pub support_l2_legacy_shared_bridge_test: bool,
    /// Old protocol version hex.
    pub old_protocol_version: String,
    /// Priority transactions L2 gas limit.
    pub priority_txs_l2_gas_limit: u64,
    /// Max expected L1 gas price.
    pub max_expected_l1_gas_price: u64,
    /// Whether this is a ZkSync OS upgrade.
    pub is_zk_sync_os: bool,
    /// Whether to redeploy DA manager.
    pub redeploy_da_manager: bool,

    /// Contracts section.
    pub contracts: ContractsSection,
    /// Tokens section.
    pub tokens: TokensSection,
    /// Gateway section.
    pub gateway: GatewaySection,
    /// State transition section.
    pub state_transition: StateTransitionSection,
    /// ZkSync OS section.
    pub zksync_os: ZkSyncOsSection,
}

/// Contracts configuration section.
#[derive(Debug, Clone, Serialize)]
pub struct ContractsSection {
    /// Governance minimum delay.
    pub governance_min_delay: u64,
    /// Maximum number of chains.
    pub max_number_of_chains: u64,
    /// Create2 factory salt.
    pub create2_factory_salt: String,
    /// Create2 factory address.
    pub create2_factory_addr: String,
    /// Validator timelock execution delay.
    pub validator_timelock_execution_delay: u64,
    /// Genesis root hash.
    pub genesis_root: String,
    /// Genesis rollup leaf index.
    pub genesis_rollup_leaf_index: u64,
    /// Genesis batch commitment.
    pub genesis_batch_commitment: String,
    /// Recursion node level VK hash.
    pub recursion_node_level_vk_hash: String,
    /// Recursion leaf level VK hash.
    pub recursion_leaf_level_vk_hash: String,
    /// Recursion circuits set VKs hash.
    pub recursion_circuits_set_vks_hash: String,
    /// Priority transaction max gas limit.
    pub priority_tx_max_gas_limit: u64,
    /// Diamond init pubdata pricing mode.
    pub diamond_init_pubdata_pricing_mode: u64,
    /// Diamond init batch overhead L1 gas.
    pub diamond_init_batch_overhead_l1_gas: u64,
    /// Diamond init max pubdata per batch.
    pub diamond_init_max_pubdata_per_batch: u64,
    /// Diamond init max L2 gas per batch.
    pub diamond_init_max_l2_gas_per_batch: u64,
    /// Diamond init priority tx max pubdata.
    pub diamond_init_priority_tx_max_pubdata: u64,
    /// Diamond init minimal L2 gas price.
    pub diamond_init_minimal_l2_gas_price: u64,
    /// Bootloader hash.
    pub bootloader_hash: String,
    /// Default AA hash.
    pub default_aa_hash: String,
    /// EVM emulator hash.
    pub evm_emulator_hash: String,
    /// Bridgehub proxy address.
    pub bridgehub_proxy_address: String,
    /// Rollup DA manager address.
    pub rollup_da_manager: String,
    /// Governance security council address.
    pub governance_security_council_address: String,
    /// Latest protocol version (hex).
    pub latest_protocol_version: String,
    /// L1 bytecodes supplier address.
    pub l1_bytecodes_supplier_addr: String,
    /// Protocol upgrade handler proxy address.
    pub protocol_upgrade_handler_proxy_address: String,
    /// Protocol upgrade handler implementation address.
    pub protocol_upgrade_handler_implementation_address: String,
}

/// Tokens configuration section.
#[derive(Debug, Clone, Serialize)]
pub struct TokensSection {
    /// WETH token address.
    pub token_weth_address: String,
}

/// Gateway configuration section.
#[derive(Debug, Clone, Serialize)]
pub struct GatewaySection {
    /// Gateway chain ID.
    pub chain_id: u64,
    /// Gateway state transition configuration.
    pub gateway_state_transition: GatewayStateTransitionSection,
}

/// Gateway state transition configuration.
#[derive(Debug, Clone, Serialize)]
pub struct GatewayStateTransitionSection {
    /// Chain type manager proxy address.
    pub chain_type_manager_proxy_addr: String,
    /// Rollup DA manager address.
    pub rollup_da_manager: String,
    /// Chain type manager proxy admin address.
    pub chain_type_manager_proxy_admin: String,
    /// Rollup SL DA validator address.
    pub rollup_sl_da_validator: String,
}

/// State transition configuration section.
#[derive(Debug, Clone, Serialize)]
pub struct StateTransitionSection {
    /// Admin facet address.
    pub admin_facet_addr: String,
    /// Diamond init address.
    pub diamond_init_addr: String,
    /// Executor facet address.
    pub executor_facet_addr: String,
    /// Genesis upgrade address.
    pub genesis_upgrade_addr: String,
    /// Getters facet address.
    pub getters_facet_addr: String,
    /// Mailbox facet address.
    pub mailbox_facet_addr: String,
    /// Force deployments data (hex).
    pub force_deployments_data: String,
}

/// ZkSync OS configuration section.
#[derive(Debug, Clone, Serialize)]
pub struct ZkSyncOsSection {
    /// Sample chain ID.
    pub sample_chain_id: u64,
    /// Optional CTM address.
    pub optional_ctm_address: String,
    /// Current dual verifier address.
    pub current_dual_verifier: String,
}

/// Values extracted from previous upgrade YAML for `[state_transition]` section.
#[derive(Debug, Clone, Default)]
pub struct PreviousUpgradeValues {
    /// Admin facet address.
    pub admin_facet_addr: Option<String>,
    /// Diamond init address.
    pub diamond_init_addr: Option<String>,
    /// Executor facet address.
    pub executor_facet_addr: Option<String>,
    /// Genesis upgrade address.
    pub genesis_upgrade_addr: Option<String>,
    /// Getters facet address.
    pub getters_facet_addr: Option<String>,
    /// Mailbox facet address.
    pub mailbox_facet_addr: Option<String>,
    /// Force deployments data.
    pub force_deployments_data: Option<String>,
}

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
    toml_config.owner_address = format!("{governance}");
    toml_config.contracts.bridgehub_proxy_address = format!("{}", config.bridgehub_address);
    toml_config.zksync_os.optional_ctm_address = format!("{ctm}");
    toml_config.zksync_os.current_dual_verifier = format!("{verifier}");

    // From ecosystem state config
    if let Some(addr) = config.create2_factory_addr {
        toml_config.contracts.create2_factory_addr = format!("{addr}");
    }
    if let Some(salt) = config.create2_factory_salt {
        toml_config.contracts.create2_factory_salt = format!("{salt}");
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
    if let Some(v) = &values.admin_facet_addr {
        section.admin_facet_addr = v.clone();
    }
    if let Some(v) = &values.diamond_init_addr {
        section.diamond_init_addr = v.clone();
    }
    if let Some(v) = &values.executor_facet_addr {
        section.executor_facet_addr = v.clone();
    }
    if let Some(v) = &values.genesis_upgrade_addr {
        section.genesis_upgrade_addr = v.clone();
    }
    if let Some(v) = &values.getters_facet_addr {
        section.getters_facet_addr = v.clone();
    }
    if let Some(v) = &values.mailbox_facet_addr {
        section.mailbox_facet_addr = v.clone();
    }
    if let Some(v) = &values.force_deployments_data {
        section.force_deployments_data = v.clone();
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_chain_toml_serializes() {
        let config = ChainTomlConfig {
            era_chain_id: 270,
            testnet_verifier: true,
            governance_upgrade_timer_initial_delay: 0,
            owner_address: "0xaaaa".to_string(),
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
                create2_factory_addr: "0xbbbb".to_string(),
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
                bridgehub_proxy_address: "0xdddd".to_string(),
                rollup_da_manager: "0xeeee".to_string(),
                governance_security_council_address: "0xffff".to_string(),
                latest_protocol_version: "0x1e00000001".to_string(),
                l1_bytecodes_supplier_addr: "0x1111".to_string(),
                protocol_upgrade_handler_proxy_address: "0x2222".to_string(),
                protocol_upgrade_handler_implementation_address: "0x3333".to_string(),
            },
            tokens: TokensSection {
                token_weth_address: "0x4444".to_string(),
            },
            gateway: GatewaySection {
                chain_id: 270,
                gateway_state_transition: GatewayStateTransitionSection {
                    chain_type_manager_proxy_addr: "0x00".to_string(),
                    rollup_da_manager: "0x00".to_string(),
                    chain_type_manager_proxy_admin: "0x00".to_string(),
                    rollup_sl_da_validator: "0x00".to_string(),
                },
            },
            state_transition: StateTransitionSection {
                admin_facet_addr: "0x5555".to_string(),
                diamond_init_addr: "0x6666".to_string(),
                executor_facet_addr: "0x7777".to_string(),
                genesis_upgrade_addr: "0x8888".to_string(),
                getters_facet_addr: "0x9999".to_string(),
                mailbox_facet_addr: "0xaaaa".to_string(),
                force_deployments_data: "0xbbbb".to_string(),
            },
            zksync_os: ZkSyncOsSection {
                sample_chain_id: 270,
                optional_ctm_address: "0xcccc".to_string(),
                current_dual_verifier: "0xdddd".to_string(),
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
        assert!(toml_str.contains("bridgehub_proxy_address = \"0xdddd\""));
    }
}
