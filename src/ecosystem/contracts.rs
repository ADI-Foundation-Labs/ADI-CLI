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
// Note: Currently unused as commands are implemented in later phases (US1-US6)
#[allow(dead_code)]
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

#[allow(dead_code)]
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

    /// Parse ecosystem contracts from zkstack output YAML file.
    ///
    /// The zkstack CLI outputs contract addresses to `configs/contracts.yaml`
    /// after deployment. This function parses that file to extract addresses.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the contracts.yaml file
    ///
    /// # Returns
    ///
    /// Parsed `EcosystemContracts` with all deployed addresses.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - File cannot be read
    /// - YAML parsing fails
    /// - Required addresses are missing
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let contracts = EcosystemContracts::from_yaml_file(
    ///     Path::new("/path/to/ecosystem/configs/contracts.yaml")
    /// )?;
    /// ```
    pub fn from_yaml_file(path: &std::path::Path) -> eyre::Result<Self> {
        use eyre::WrapErr;

        let content = std::fs::read_to_string(path)
            .wrap_err_with(|| format!("Failed to read contracts file: {}", path.display()))?;

        Self::from_yaml_str(&content)
    }

    /// Parse ecosystem contracts from a YAML string.
    ///
    /// # Arguments
    ///
    /// * `yaml` - YAML content as a string
    ///
    /// # Returns
    ///
    /// Parsed `EcosystemContracts` with all deployed addresses.
    ///
    /// # Errors
    ///
    /// Returns an error if parsing fails or required addresses are missing.
    pub fn from_yaml_str(yaml: &str) -> eyre::Result<Self> {
        use eyre::WrapErr;

        // Parse as a generic YAML value first to handle nested structure
        let value: serde_yaml::Value =
            serde_yaml::from_str(yaml).wrap_err("Failed to parse contracts YAML")?;

        // Try to deserialize directly first
        if let Ok(contracts) = serde_yaml::from_str::<Self>(yaml) {
            return Ok(contracts);
        }

        // If direct deserialization fails, try to extract from nested structure
        // zkstack may output in different formats
        Self::extract_from_yaml_value(&value)
    }

    /// Extract contract addresses from a nested YAML structure.
    ///
    /// Handles various zkstack output formats.
    fn extract_from_yaml_value(value: &serde_yaml::Value) -> eyre::Result<Self> {
        use eyre::WrapErr;

        // Helper to get address from value
        fn get_address(value: &serde_yaml::Value, key: &str) -> eyre::Result<Address> {
            let addr_str = value
                .get(key)
                .and_then(|v| v.as_str())
                .ok_or_else(|| eyre::eyre!("Missing contract address: {}", key))?;

            addr_str
                .parse::<Address>()
                .wrap_err_with(|| format!("Invalid address for {}: {}", key, addr_str))
        }

        // Helper to get optional address from value
        fn get_optional_address(value: &serde_yaml::Value, key: &str) -> Option<Address> {
            value
                .get(key)
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse::<Address>().ok())
        }

        // Helper to get B256 from value
        fn get_b256(value: &serde_yaml::Value, key: &str) -> eyre::Result<B256> {
            let hash_str = value
                .get(key)
                .and_then(|v| v.as_str())
                .ok_or_else(|| eyre::eyre!("Missing hash: {}", key))?;

            hash_str
                .parse::<B256>()
                .wrap_err_with(|| format!("Invalid B256 for {}: {}", key, hash_str))
        }

        // Try various key naming conventions
        let bridgehub_keys = [
            "bridgehub_proxy_addr",
            "bridgehub_proxy_address",
            "bridgehubProxyAddr",
            "bridgehub",
        ];
        let state_transition_keys = [
            "state_transition_proxy_addr",
            "state_transition_proxy_address",
            "stateTransitionProxyAddr",
            "state_transition_manager_addr",
        ];
        let governance_keys = [
            "governance_addr",
            "governance_address",
            "governanceAddr",
            "governance",
        ];
        let chain_admin_keys = [
            "chain_admin_addr",
            "chain_admin_address",
            "chainAdminAddr",
            "chain_admin",
        ];
        let verifier_keys = [
            "verifier_addr",
            "verifier_address",
            "verifierAddr",
            "verifier",
        ];
        let l1_rollup_da_manager_keys = [
            "l1_rollup_da_manager",
            "l1_rollup_da_manager_addr",
            "rollup_da_manager",
        ];
        let rollup_l1_da_validator_keys = [
            "rollup_l1_da_validator",
            "rollup_l1_da_validator_addr",
            "rollup_da_validator",
        ];
        let native_token_vault_keys = [
            "native_token_vault_addr",
            "native_token_vault_address",
            "nativeTokenVaultAddr",
        ];
        let l1_nullifier_keys = ["l1_nullifier_addr", "l1_nullifier_address", "l1Nullifier"];
        let l1_asset_router_keys = ["l1_asset_router", "l1_asset_router_addr", "assetRouter"];
        let validator_timelock_keys = [
            "validator_timelock_addr",
            "validator_timelock_address",
            "validatorTimelock",
        ];
        let server_notifier_keys = [
            "server_notifier_proxy_addr",
            "server_notifier_proxy_address",
            "serverNotifierProxy",
        ];
        let create2_factory_keys = [
            "create2_factory_addr",
            "create2_factory_address",
            "create2Factory",
        ];
        let create2_factory_salt_keys =
            ["create2_factory_salt", "create2FactorySalt", "create2_salt"];

        fn find_address(value: &serde_yaml::Value, keys: &[&str]) -> eyre::Result<Address> {
            for key in keys {
                if let Some(v) = value.get(*key) {
                    if let Some(s) = v.as_str() {
                        if let Ok(addr) = s.parse::<Address>() {
                            return Ok(addr);
                        }
                    }
                }
            }
            Err(eyre::eyre!("Missing address for keys: {:?}", keys))
        }

        fn find_b256(value: &serde_yaml::Value, keys: &[&str]) -> eyre::Result<B256> {
            for key in keys {
                if let Some(v) = value.get(*key) {
                    if let Some(s) = v.as_str() {
                        if let Ok(hash) = s.parse::<B256>() {
                            return Ok(hash);
                        }
                    }
                }
            }
            Err(eyre::eyre!("Missing B256 for keys: {:?}", keys))
        }

        fn find_optional_address(value: &serde_yaml::Value, keys: &[&str]) -> Option<Address> {
            for key in keys {
                if let Some(v) = value.get(*key) {
                    if let Some(s) = v.as_str() {
                        if let Ok(addr) = s.parse::<Address>() {
                            return Some(addr);
                        }
                    }
                }
            }
            None
        }

        Ok(Self {
            bridgehub_proxy_addr: find_address(value, &bridgehub_keys)
                .wrap_err("Missing bridgehub address")?,
            state_transition_proxy_addr: find_address(value, &state_transition_keys)
                .wrap_err("Missing state transition address")?,
            governance_addr: find_address(value, &governance_keys)
                .wrap_err("Missing governance address")?,
            chain_admin_addr: find_address(value, &chain_admin_keys)
                .wrap_err("Missing chain admin address")?,
            verifier_addr: find_address(value, &verifier_keys)
                .wrap_err("Missing verifier address")?,
            verifier_fflonk_addr: find_optional_address(
                value,
                &["verifier_fflonk_addr", "verifier_fflonk", "verifierFflonk"],
            ),
            verifier_plonk_addr: find_optional_address(
                value,
                &["verifier_plonk_addr", "verifier_plonk", "verifierPlonk"],
            ),
            l1_rollup_da_manager: find_address(value, &l1_rollup_da_manager_keys)
                .wrap_err("Missing L1 rollup DA manager address")?,
            rollup_l1_da_validator: find_address(value, &rollup_l1_da_validator_keys)
                .wrap_err("Missing rollup L1 DA validator address")?,
            native_token_vault_addr: find_address(value, &native_token_vault_keys)
                .wrap_err("Missing native token vault address")?,
            l1_nullifier_addr: find_address(value, &l1_nullifier_keys)
                .wrap_err("Missing L1 nullifier address")?,
            l1_asset_router: find_address(value, &l1_asset_router_keys)
                .wrap_err("Missing L1 asset router address")?,
            validator_timelock_addr: find_address(value, &validator_timelock_keys)
                .wrap_err("Missing validator timelock address")?,
            server_notifier_proxy_addr: find_address(value, &server_notifier_keys)
                .wrap_err("Missing server notifier proxy address")?,
            create2_factory_addr: find_address(value, &create2_factory_keys)
                .wrap_err("Missing create2 factory address")?,
            create2_factory_salt: find_b256(value, &create2_factory_salt_keys)
                .wrap_err("Missing create2 factory salt")?,
        })
    }
}
