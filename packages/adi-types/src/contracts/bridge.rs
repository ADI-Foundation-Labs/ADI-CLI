//! Bridge contract address types shared by ecosystem and chain.

use alloy_primitives::Address;
use serde::{Deserialize, Serialize};

/// Bridge contract addresses.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
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
#[serde(rename_all = "snake_case")]
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
