//! Typed structs for chain.toml configuration.
//!
//! These structs serialize to the TOML format required by forge upgrade scripts.

use serde::Serialize;

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
