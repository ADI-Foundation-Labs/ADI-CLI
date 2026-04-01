//! Ecosystem-level contract address types.

use alloy_primitives::{Address, B256};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::bridge::BridgesConfig;

/// L1 governance and admin contracts.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct L1Contracts {
    /// Governance address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub governance_addr: Option<Address>,

    /// Chain admin address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chain_admin_addr: Option<Address>,

    /// Transaction filterer address (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transaction_filterer_addr: Option<Address>,
}

/// Core ecosystem contract addresses.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct CoreEcosystemContracts {
    /// Bridgehub proxy address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bridgehub_proxy_addr: Option<Address>,

    /// Message root proxy address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_root_proxy_addr: Option<Address>,

    /// Transparent proxy admin address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transparent_proxy_admin_addr: Option<Address>,

    /// STM deployment tracker proxy address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stm_deployment_tracker_proxy_addr: Option<Address>,

    /// Native token vault address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub native_token_vault_addr: Option<Address>,
}

/// ZkSync OS Chain Type Manager (CTM) contracts.
///
/// These contracts are deployed as part of the ZkSync OS ecosystem.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ZkSyncOsCtm {
    /// Governance address for the CTM.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub governance: Option<Address>,

    /// Chain admin address for the CTM.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chain_admin: Option<Address>,

    /// Proxy admin address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proxy_admin: Option<Address>,

    /// State transition proxy address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state_transition_proxy_addr: Option<Address>,

    /// Validator timelock address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validator_timelock_addr: Option<Address>,

    /// Server notifier proxy address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_notifier_proxy_addr: Option<Address>,

    /// Verifier address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verifier_addr: Option<Address>,

    /// L1 Rollup DA manager address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub l1_rollup_da_manager: Option<Address>,

    /// L1 bytecodes supplier address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub l1_bytecodes_supplier_addr: Option<Address>,

    /// L1 wrapped base token store address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub l1_wrapped_base_token_store: Option<Address>,

    /// Default upgrade address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_upgrade_addr: Option<Address>,

    /// Genesis upgrade address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub genesis_upgrade_addr: Option<Address>,

    /// Rollup L1 DA validator address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rollup_l1_da_validator_addr: Option<Address>,

    /// No DA validium L1 validator address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub no_da_validium_l1_validator_addr: Option<Address>,

    /// Blobs ZkSync OS L1 DA validator address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blobs_zksync_os_l1_da_validator_addr: Option<Address>,

    /// Avail L1 DA validator address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avail_l1_da_validator_addr: Option<Address>,

    /// Diamond cut data (hex-encoded ABI data containing facet addresses).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diamond_cut_data: Option<String>,

    /// Force deployments data (hex-encoded).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub force_deployments_data: Option<String>,

    // Diamond facets (extracted from diamond_cut_data)
    /// Admin facet address.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub admin_facet_addr: Option<Address>,

    /// Executor facet address.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub executor_facet_addr: Option<Address>,

    /// Mailbox facet address.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mailbox_facet_addr: Option<Address>,

    /// Getters facet address.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub getters_facet_addr: Option<Address>,

    /// DiamondInit address.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diamond_init_addr: Option<Address>,

    // Implementation contracts (read via EIP-1967 storage slot)
    /// Bridgehub implementation address.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bridgehub_impl_addr: Option<Address>,

    /// Message root implementation address.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message_root_impl_addr: Option<Address>,

    /// Native token vault implementation address.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub native_token_vault_impl_addr: Option<Address>,

    /// STM deployment tracker implementation address.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stm_deployment_tracker_impl_addr: Option<Address>,

    /// Chain type manager implementation address.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chain_type_manager_impl_addr: Option<Address>,

    /// Server notifier implementation address.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub server_notifier_impl_addr: Option<Address>,

    /// ERC20 bridge implementation address.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub erc20_bridge_impl_addr: Option<Address>,

    /// Shared bridge (L1 Asset Router) implementation address.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shared_bridge_impl_addr: Option<Address>,

    /// L1 Nullifier implementation address.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub l1_nullifier_impl_addr: Option<Address>,

    /// Validator timelock implementation address.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub validator_timelock_impl_addr: Option<Address>,

    // Verifier components
    /// ZKsyncOS Verifier Fflonk address.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verifier_fflonk_addr: Option<Address>,

    /// ZKsyncOS Verifier Plonk address.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verifier_plonk_addr: Option<Address>,

    // Bridge token contracts
    /// Bridged Standard ERC20 implementation address.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bridged_standard_erc20_addr: Option<Address>,

    /// Bridged Token Beacon address.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bridged_token_beacon_addr: Option<Address>,

    // Avail test contracts
    /// Dummy Avail Bridge address (test/mock).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dummy_avail_bridge_addr: Option<Address>,

    /// Dummy VectorX address (test/mock).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dummy_vector_x_addr: Option<Address>,

    /// Server notifier proxy admin address.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub server_notifier_proxy_admin_addr: Option<Address>,

    /// Verifier owner address (for verification constructor args).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verifier_owner_addr: Option<Address>,

    /// Whether the verifier is a testnet verifier (ZKsyncOSTestnetVerifier).
    /// Detected at runtime via mockVerify call.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_testnet_verifier: Option<bool>,
}

/// Ecosystem contracts configuration from configs/contracts.yaml.
///
/// Complex nested structure with many optional fields.
/// Uses `#[serde(flatten)]` to capture additional fields not explicitly defined.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct EcosystemContracts {
    /// Create2 factory address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub create2_factory_addr: Option<Address>,

    /// Create2 factory salt.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub create2_factory_salt: Option<B256>,

    /// Multicall3 address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub multicall3_addr: Option<Address>,

    /// Core ecosystem contracts.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub core_ecosystem_contracts: Option<CoreEcosystemContracts>,

    /// Bridge contracts.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bridges: Option<BridgesConfig>,

    /// L1 contracts.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub l1: Option<L1Contracts>,

    /// ZkSync OS Chain Type Manager contracts.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub zksync_os_ctm: Option<ZkSyncOsCtm>,

    /// Server notifier proxy address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_notifier_proxy_addr: Option<Address>,

    /// Verifier address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verifier_addr: Option<Address>,

    /// L1 Rollup DA manager address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub l1_rollup_da_manager: Option<Address>,

    /// ChainAdmin owner (for verification constructor args).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chain_admin_owner: Option<Address>,

    /// Additional unmapped fields for forward compatibility.
    #[serde(flatten)]
    pub extra: HashMap<String, serde_yaml::Value>,
}

impl EcosystemContracts {
    /// Returns the bridgehub proxy address if available.
    pub fn bridgehub_addr(&self) -> Option<Address> {
        self.core_ecosystem_contracts
            .as_ref()
            .and_then(|c| c.bridgehub_proxy_addr)
    }

    /// Returns the governance address if available.
    ///
    /// Checks L1 contracts first, then falls back to ZkSync OS CTM.
    pub fn governance_addr(&self) -> Option<Address> {
        self.l1
            .as_ref()
            .and_then(|l| l.governance_addr)
            .or_else(|| self.zksync_os_ctm.as_ref().and_then(|c| c.governance))
    }

    /// Returns the chain admin address if available.
    ///
    /// Checks L1 contracts first, then falls back to ZkSync OS CTM.
    pub fn chain_admin_addr(&self) -> Option<Address> {
        self.l1
            .as_ref()
            .and_then(|l| l.chain_admin_addr)
            .or_else(|| self.zksync_os_ctm.as_ref().and_then(|c| c.chain_admin))
    }

    /// Returns the validator timelock address if available.
    pub fn validator_timelock_addr(&self) -> Option<Address> {
        self.zksync_os_ctm
            .as_ref()
            .and_then(|c| c.validator_timelock_addr)
    }

    /// Returns the server notifier proxy address if available.
    ///
    /// Checks root level first, then falls back to ZkSync OS CTM.
    pub fn server_notifier_addr(&self) -> Option<Address> {
        self.server_notifier_proxy_addr.or_else(|| {
            self.zksync_os_ctm
                .as_ref()
                .and_then(|c| c.server_notifier_proxy_addr)
        })
    }

    /// Returns the verifier address if available.
    ///
    /// Checks root level first, then falls back to ZkSync OS CTM.
    pub fn verifier_addr(&self) -> Option<Address> {
        self.verifier_addr
            .or_else(|| self.zksync_os_ctm.as_ref().and_then(|c| c.verifier_addr))
    }

    /// Returns the L1 Rollup DA manager address if available.
    ///
    /// Checks root level first, then falls back to ZkSync OS CTM.
    pub fn l1_rollup_da_manager_addr(&self) -> Option<Address> {
        self.l1_rollup_da_manager.or_else(|| {
            self.zksync_os_ctm
                .as_ref()
                .and_then(|c| c.l1_rollup_da_manager)
        })
    }

    /// Returns the native token vault address if available.
    pub fn native_token_vault_addr(&self) -> Option<Address> {
        self.core_ecosystem_contracts
            .as_ref()
            .and_then(|c| c.native_token_vault_addr)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_ecosystem_contracts_deserialize() {
        let yaml = r#"
create2_factory_addr: "0x4e59b44847b379578588920ca78fbf26c0b4956c"
create2_factory_salt: "0x11a3107563d5ef54e10104dc13fcb68775698f309a25a9b129dae8ccea406fda"
multicall3_addr: "0xca11bde05977b3631167028862be2a173976ca11"
core_ecosystem_contracts:
  bridgehub_proxy_addr: "0x1234567890123456789012345678901234567890"
"#;
        let contracts: EcosystemContracts = serde_yaml::from_str(yaml).unwrap();
        assert!(contracts.create2_factory_addr.is_some());
        assert!(contracts.bridgehub_addr().is_some());
    }

    #[test]
    fn test_contracts_with_extra_fields() {
        let yaml = r#"
create2_factory_addr: "0x4e59b44847b379578588920ca78fbf26c0b4956c"
some_future_field: "value"
another_field:
  nested: true
"#;
        let contracts: EcosystemContracts = serde_yaml::from_str(yaml).unwrap();
        assert!(contracts.create2_factory_addr.is_some());
        assert!(contracts.extra.contains_key("some_future_field"));
    }
}
