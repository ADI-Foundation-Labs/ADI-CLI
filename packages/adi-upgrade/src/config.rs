//! Upgrade configuration generation.

use std::io::Write;
use std::path::Path;

use adi_state::StateManager;
use alloy_primitives::Address;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::error::{Result, UpgradeError};

/// Configuration for upgrade operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpgradeConfig {
    /// Settlement layer RPC URL.
    pub l1_rpc_url: Url,

    /// Ecosystem name.
    pub ecosystem_name: String,

    /// Governor address (from wallets.yaml).
    pub governor_address: Address,

    /// Deployer address (from wallets.yaml).
    pub deployer_address: Address,

    /// Bridgehub address.
    pub bridgehub_address: Option<Address>,

    /// CTM address (queried on-chain).
    pub ctm_address: Option<Address>,

    /// Governance address (queried on-chain).
    pub governance_address: Option<Address>,

    /// Gas price multiplier.
    pub gas_multiplier: f64,
}

impl UpgradeConfig {
    /// Load upgrade config from ecosystem state.
    ///
    /// # Arguments
    ///
    /// * `state_manager` - State manager for the ecosystem
    /// * `ecosystem_name` - Name of the ecosystem
    /// * `l1_rpc_url` - Settlement layer RPC URL
    /// * `gas_multiplier` - Gas price multiplier
    ///
    /// # Errors
    ///
    /// Returns [`UpgradeError::Config`] if wallets or contracts cannot be loaded,
    /// or if required wallet addresses are missing from state.
    pub async fn from_state(
        state_manager: &StateManager,
        ecosystem_name: &str,
        l1_rpc_url: Url,
        gas_multiplier: f64,
    ) -> Result<Self> {
        log::debug!("Loading upgrade config from state");

        let wallets = state_manager
            .ecosystem()
            .wallets()
            .await
            .map_err(|e| UpgradeError::Config(format!("Failed to load wallets: {e}")))?;

        let governor_address = wallets
            .governor
            .as_ref()
            .map(|w| w.address)
            .ok_or_else(|| {
                UpgradeError::Config("Governor wallet not found in state".to_string())
            })?;

        let deployer_address = wallets
            .deployer
            .as_ref()
            .map(|w| w.address)
            .ok_or_else(|| {
                UpgradeError::Config("Deployer wallet not found in state".to_string())
            })?;

        let contracts = state_manager
            .ecosystem()
            .contracts()
            .await
            .map_err(|e| UpgradeError::Config(format!("Failed to load contracts: {e}")))?;

        let bridgehub_address = contracts
            .core_ecosystem_contracts
            .as_ref()
            .and_then(|c| c.bridgehub_proxy_addr);

        Ok(Self {
            l1_rpc_url,
            ecosystem_name: ecosystem_name.to_string(),
            governor_address,
            deployer_address,
            bridgehub_address,
            ctm_address: None,
            governance_address: None,
            gas_multiplier,
        })
    }

    /// Write config to chain.toml format for forge script.
    ///
    /// # Errors
    ///
    /// Returns [`UpgradeError::Config`] if the file cannot be created or written.
    pub fn write_chain_toml(&self, path: &Path) -> Result<()> {
        log::debug!("Writing chain.toml to {}", path.display());

        let content = format!(
            r#"[profile.default]
l1_rpc_url = "{}"
governor = "{}"
deployer = "{}"
"#,
            self.l1_rpc_url, self.governor_address, self.deployer_address,
        );

        let mut file = std::fs::File::create(path)
            .map_err(|e| UpgradeError::Config(format!("Failed to create chain.toml: {e}")))?;

        file.write_all(content.as_bytes())
            .map_err(|e| UpgradeError::Config(format!("Failed to write chain.toml: {e}")))?;

        Ok(())
    }
}
