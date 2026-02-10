//! Contract address types for ecosystem and chain deployments.

use alloy_primitives::{Address, B256};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Bridge contract addresses.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct BridgeContracts {
    /// L1 bridge address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub l1_address: Option<Address>,

    /// L2 bridge address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub l2_address: Option<Address>,
}

/// Bridges configuration.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct BridgesConfig {
    /// ERC20 bridge.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub erc20: Option<BridgeContracts>,

    /// Shared bridge.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shared: Option<BridgeContracts>,

    /// L1 nullifier address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub l1_nullifier_addr: Option<Address>,
}

/// L1 governance and admin contracts.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
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
}

/// Ecosystem contracts configuration from configs/contracts.yaml.
///
/// Complex nested structure with many optional fields.
/// Uses `#[serde(flatten)]` to capture additional fields not explicitly defined.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
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
}

/// Chain L1 contracts from chains/*/configs/contracts.yaml.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ChainL1Contracts {
    /// Default upgrade address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_upgrade_addr: Option<Address>,

    /// Diamond proxy address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diamond_proxy_addr: Option<Address>,

    /// Governance address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub governance_addr: Option<Address>,

    /// Chain admin address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chain_admin_addr: Option<Address>,

    /// Access control restriction address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_control_restriction_addr: Option<Address>,

    /// Chain proxy admin address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chain_proxy_admin_addr: Option<Address>,

    /// Multicall3 address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub multicall3_addr: Option<Address>,

    /// Verifier address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verifier_addr: Option<Address>,

    /// Validator timelock address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validator_timelock_addr: Option<Address>,

    /// Base token address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_token_addr: Option<Address>,

    /// Base token asset ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_token_asset_id: Option<B256>,

    /// Rollup L1 DA validator address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rollup_l1_da_validator_addr: Option<Address>,

    /// Avail L1 DA validator address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avail_l1_da_validator_addr: Option<Address>,

    /// No DA validium L1 validator address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub no_da_validium_l1_validator_addr: Option<Address>,
}

/// Chain contracts from chains/*/configs/contracts.yaml.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ChainContracts {
    /// Create2 factory address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub create2_factory_addr: Option<Address>,

    /// Create2 factory salt.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub create2_factory_salt: Option<B256>,

    /// Ecosystem contracts (reference to ecosystem-level contracts).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ecosystem_contracts: Option<CoreEcosystemContracts>,

    /// Bridge contracts.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bridges: Option<BridgesConfig>,

    /// L1 contracts specific to this chain.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub l1: Option<ChainL1Contracts>,

    /// L2 contracts (often empty).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub l2: Option<HashMap<String, serde_yaml::Value>>,

    /// Additional unmapped fields for forward compatibility.
    #[serde(flatten)]
    pub extra: HashMap<String, serde_yaml::Value>,
}

impl ChainContracts {
    /// Returns the diamond proxy address if available.
    pub fn diamond_proxy_addr(&self) -> Option<Address> {
        self.l1.as_ref().and_then(|l| l.diamond_proxy_addr)
    }

    /// Returns the chain admin address if available.
    pub fn chain_admin_addr(&self) -> Option<Address> {
        self.l1.as_ref().and_then(|l| l.chain_admin_addr)
    }

    /// Returns the governance address if available.
    pub fn governance_addr(&self) -> Option<Address> {
        self.l1.as_ref().and_then(|l| l.governance_addr)
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
    fn test_chain_contracts_deserialize() {
        let yaml = r#"
create2_factory_addr: "0x4e59b44847b379578588920ca78fbf26c0b4956c"
l1:
  diamond_proxy_addr: "0x1234567890123456789012345678901234567890"
  governance_addr: "0x2345678901234567890123456789012345678901"
"#;
        let contracts: ChainContracts = serde_yaml::from_str(yaml).unwrap();
        assert!(contracts.diamond_proxy_addr().is_some());
        assert!(contracts.governance_addr().is_some());
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
