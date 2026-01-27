//! Ecosystem contract addresses.
//!
//! This module defines the contract address structures for ecosystem-level
//! contracts deployed on the settlement layer.

use alloy_primitives::{Address, B256};
use serde::{Deserialize, Serialize};

/// Contract addresses deployed at ecosystem level on the settlement layer.
///
/// These contracts are shared by all chains within the ecosystem and include
/// the core infrastructure (Bridgehub, Governance), verifiers, DA infrastructure,
/// and token bridges.
///
/// # Note
///
/// The actual contracts deployed vary by protocol version. This struct captures
/// the core contracts used in v0.29.x-v0.30.x. Additional contracts (libraries,
/// implementations, facets) are tracked separately in deployment output files.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EcosystemContracts {
    // ========================================
    // Core Infrastructure
    // ========================================
    /// Bridgehub proxy contract - central hub for chain registration.
    pub bridgehub_proxy_addr: Address,

    /// State transition manager proxy contract.
    pub state_transition_proxy_addr: Address,

    /// Governance contract for protocol upgrades.
    pub governance_addr: Address,

    /// Chain admin contract for chain-level administration.
    pub chain_admin_addr: Address,

    // ========================================
    // Verifiers
    // ========================================
    /// Main verifier contract address.
    pub verifier_addr: Address,

    /// FFLONK verifier (optional, version-dependent).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verifier_fflonk_addr: Option<Address>,

    /// PLONK verifier (optional, version-dependent).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verifier_plonk_addr: Option<Address>,

    // ========================================
    // Data Availability (DA) Infrastructure
    // ========================================
    /// L1 Rollup DA Manager contract.
    pub l1_rollup_da_manager: Address,

    /// Rollup L1 DA Validator contract.
    pub rollup_l1_da_validator: Address,

    // ========================================
    // Token Infrastructure
    // ========================================
    /// Native Token Vault contract for token bridging.
    pub native_token_vault_addr: Address,

    /// L1 Nullifier contract for deposit tracking.
    pub l1_nullifier_addr: Address,

    /// L1 Asset Router contract for asset management.
    pub l1_asset_router: Address,

    // ========================================
    // Timelock & Server
    // ========================================
    /// Validator Timelock contract for delayed operations.
    pub validator_timelock_addr: Address,

    /// Server Notifier proxy contract.
    pub server_notifier_proxy_addr: Address,

    // ========================================
    // Factory
    // ========================================
    /// Create2 Factory contract for deterministic deployments.
    pub create2_factory_addr: Address,

    /// Salt used with the Create2 Factory.
    pub create2_factory_salt: B256,

    // TODO: Include additional contracts for database storage (v29.11 deploys ~50 ecosystem contracts):
    //
    // Forge Libraries (auto-deployed via Create2 Factory):
    // - bytecode_utils_addr: Address          // BytecodeUtils library
    // - utils_addr: Address                   // Utils library
    //
    // L1 Core - Infrastructure:
    // - proxy_admin_addr: Address             // ProxyAdmin (via Create2AndTransfer)
    // - multicall3_addr: Option<Address>      // Multicall3 (optional, may pre-exist)
    // - transaction_filterer_addr: Option<Address> // Transaction filterer
    //
    // L1 Core - Implementation contracts (paired with proxies above):
    // - l1_bridgehub_impl_addr: Address       // L1Bridgehub implementation
    // - l1_message_root_impl_addr: Address    // L1MessageRoot implementation
    // - l1_nullifier_impl_addr: Address       // L1Nullifier implementation
    // - l1_asset_router_impl_addr: Address    // L1AssetRouter implementation
    // - l1_native_token_vault_impl_addr: Address // L1NativeTokenVault implementation
    // - l1_erc20_bridge_impl_addr: Address    // L1ERC20Bridge implementation
    // - ctm_deployment_tracker_impl_addr: Address // CTMDeploymentTracker implementation
    // - l1_chain_asset_handler_impl_addr: Address // L1ChainAssetHandler implementation
    //
    // L1 Core - Proxy addresses (some already in struct above):
    // - l1_message_root_proxy_addr: Address   // L1MessageRoot proxy
    // - l1_nullifier_proxy_addr: Address      // L1Nullifier proxy (same as l1_nullifier_addr?)
    // - l1_asset_router_proxy_addr: Address   // L1AssetRouter proxy (same as l1_asset_router?)
    // - l1_erc20_bridge_proxy_addr: Address   // L1ERC20Bridge proxy
    // - ctm_deployment_tracker_proxy_addr: Address // CTMDeploymentTracker proxy
    // - l1_chain_asset_handler_proxy_addr: Address // L1ChainAssetHandler proxy
    //
    // L1 Core - Token infrastructure:
    // - bridged_standard_erc20_addr: Address  // BridgedStandardERC20 (via Create2AndTransfer)
    // - bridged_token_beacon_addr: Address    // BridgedTokenBeacon
    //
    // CTM (ChainTypeManager) - DA infrastructure:
    // - rollup_da_manager_addr: Address       // RollupDAManager (via Create2AndTransfer)
    // - validium_l1_da_validator_addr: Address // ValidiumL1DAValidator
    // - dummy_avail_bridge_addr: Address      // DummyAvailBridge
    // - dummy_vector_x_addr: Address          // DummyVectorX
    // - avail_l1_da_validator_addr: Address   // AvailL1DAValidator
    //
    // CTM - Verifiers:
    // - verifier_fflonk_impl_addr: Address    // VerifierFflonk (or verifier_fflonk_addr above)
    // - verifier_plonk_impl_addr: Address     // VerifierPlonk (or verifier_plonk_addr above)
    // - dual_verifier_addr: Address           // DualVerifier
    //
    // CTM - Upgrade contracts:
    // - default_upgrade_addr: Address         // DefaultUpgrade
    // - l1_genesis_upgrade_addr: Address      // L1GenesisUpgrade
    // - bytecodes_supplier_addr: Address      // BytecodesSupplier
    //
    // CTM - Validator/Server:
    // - validator_timelock_impl_addr: Address // ValidatorTimelock implementation
    // - server_notifier_impl_addr: Address    // ServerNotifier implementation
    // - server_notifier_proxy_admin_addr: Address // ProxyAdmin for ServerNotifier
    //
    // CTM - Diamond Facets:
    // - executor_facet_addr: Address          // ExecutorFacet
    // - admin_facet_addr: Address             // AdminFacet
    // - mailbox_facet_addr: Address           // MailboxFacet
    // - getters_facet_addr: Address           // GettersFacet
    // - diamond_init_addr: Address            // DiamondInit
    //
    // CTM - ChainTypeManager:
    // - chain_type_manager_impl_addr: Address // ChainTypeManager implementation
}

impl EcosystemContracts {
    /// Validates that all required contract addresses are non-zero.
    ///
    /// # Errors
    ///
    /// Returns an error if any required address is zero.
    pub fn validate(&self) -> eyre::Result<()> {
        use eyre::ensure;

        ensure!(
            !self.bridgehub_proxy_addr.is_zero(),
            "Bridgehub proxy address cannot be zero"
        );

        ensure!(
            !self.governance_addr.is_zero(),
            "Governance address cannot be zero"
        );

        ensure!(
            !self.verifier_addr.is_zero(),
            "Verifier address cannot be zero"
        );

        ensure!(
            !self.create2_factory_addr.is_zero(),
            "Create2 Factory address cannot be zero"
        );

        Ok(())
    }
}
