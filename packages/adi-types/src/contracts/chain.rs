//! Chain-level contract address types.

use alloy_primitives::{Address, B256};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::bridge::BridgesConfig;

/// Chain L1 contracts from chains/*/configs/contracts.yaml.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
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

    /// Chain admin owner (for verification constructor args).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chain_admin_owner: Option<Address>,

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

    /// Blobs ZkSync OS L1 DA validator address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blobs_zksync_os_l1_da_validator_addr: Option<Address>,
}

/// Chain L2 contracts from chains/*/configs/contracts.yaml.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ChainL2Contracts {
    /// Testnet paymaster address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub testnet_paymaster_addr: Option<Address>,

    /// Default L2 upgrader address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_l2_upgrader: Option<Address>,

    /// L2 native token vault proxy address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub l2_native_token_vault_proxy_addr: Option<Address>,

    /// Consensus registry address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub consensus_registry: Option<Address>,

    /// Multicall3 address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub multicall3: Option<Address>,

    /// Timestamp asserter address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp_asserter_addr: Option<Address>,
}

/// Chain ecosystem contracts reference (extended version with CTM fields).
///
/// Used in chain's configs/contracts.yaml under `ecosystem_contracts`.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ChainEcosystemContracts {
    // Core ecosystem fields
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

    // CTM fields
    /// Governance address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub governance: Option<Address>,

    /// Chain admin address.
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

    /// Default upgrade address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_upgrade_addr: Option<Address>,

    /// Genesis upgrade address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub genesis_upgrade_addr: Option<Address>,

    /// Verifier address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verifier_addr: Option<Address>,

    /// L1 bytecodes supplier address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub l1_bytecodes_supplier_addr: Option<Address>,

    /// L1 wrapped base token store address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub l1_wrapped_base_token_store: Option<Address>,

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

    /// L1 Rollup DA manager address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub l1_rollup_da_manager: Option<Address>,
}

/// Chain contracts from chains/*/configs/contracts.yaml.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ChainContracts {
    /// Create2 factory address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub create2_factory_addr: Option<Address>,

    /// Create2 factory salt.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub create2_factory_salt: Option<B256>,

    /// Ecosystem contracts (reference to ecosystem-level contracts with CTM fields).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ecosystem_contracts: Option<ChainEcosystemContracts>,

    /// Bridge contracts.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bridges: Option<BridgesConfig>,

    /// L1 contracts specific to this chain.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub l1: Option<ChainL1Contracts>,

    /// L2 contracts.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub l2: Option<ChainL2Contracts>,

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
}
