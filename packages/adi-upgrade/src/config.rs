//! Upgrade configuration generation.

use std::path::PathBuf;

use adi_state::StateManager;
use alloy_primitives::{Address, B256};
use secrecy::SecretString;
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

    /// Governor private key (from wallets.yaml).
    #[serde(skip)]
    pub governor_private_key: SecretString,

    /// Deployer private key (from wallets.yaml).
    #[serde(skip)]
    pub deployer_private_key: SecretString,

    /// Bridgehub proxy address (required).
    pub bridgehub_address: Address,

    /// Create2 factory address.
    pub create2_factory_addr: Option<Address>,

    /// Create2 factory salt.
    pub create2_factory_salt: Option<B256>,

    /// Ecosystem state directory path.
    pub state_dir: PathBuf,

    /// Gas price multiplier percentage (None = skip gas price, e.g. localhost).
    pub gas_multiplier: Option<u64>,
}

/// Compute gas price in wei from percentage multiplier.
///
/// Uses a base gas price of 20 gwei and applies the percentage.
/// For example, multiplier 200 (= 2x) with base 20 gwei = 40 gwei.
pub(crate) fn compute_gas_price(multiplier: u64) -> u128 {
    const BASE_GAS_PRICE_GWEI: u128 = 20;
    const GWEI_TO_WEI: u128 = 1_000_000_000;

    BASE_GAS_PRICE_GWEI * GWEI_TO_WEI * u128::from(multiplier) / 100
}

impl UpgradeConfig {
    /// Load upgrade config from ecosystem state.
    ///
    /// # Arguments
    ///
    /// * `state_manager` - State manager for the ecosystem
    /// * `ecosystem_name` - Name of the ecosystem
    /// * `l1_rpc_url` - Settlement layer RPC URL
    /// * `gas_multiplier` - Gas price multiplier percentage (None for localhost)
    /// * `state_dir` - Ecosystem state directory path
    ///
    /// # Errors
    ///
    /// Returns [`UpgradeError::Config`] if wallets or contracts cannot be loaded,
    /// or if required wallet addresses are missing from state.
    pub async fn from_state(
        state_manager: &StateManager,
        ecosystem_name: &str,
        l1_rpc_url: Url,
        gas_multiplier: Option<u64>,
        state_dir: PathBuf,
    ) -> Result<Self> {
        log::debug!("Loading upgrade config from state");

        let wallets = state_manager
            .ecosystem()
            .wallets()
            .await
            .map_err(|e| UpgradeError::Config(format!("Failed to load wallets: {e}")))?;

        let governor = wallets.governor.as_ref().ok_or_else(|| {
            UpgradeError::Config("Governor wallet not found in state".to_string())
        })?;

        let governor_address = governor.address;
        let governor_private_key = SecretString::from(governor.expose_private_key().to_string());

        let deployer = wallets.deployer.as_ref().ok_or_else(|| {
            UpgradeError::Config("Deployer wallet not found in state".to_string())
        })?;

        let deployer_address = deployer.address;
        let deployer_private_key = SecretString::from(deployer.expose_private_key().to_string());

        let contracts = state_manager
            .ecosystem()
            .contracts()
            .await
            .map_err(|e| UpgradeError::Config(format!("Failed to load contracts: {e}")))?;

        let bridgehub_address = contracts
            .core_ecosystem_contracts
            .as_ref()
            .and_then(|c| c.bridgehub_proxy_addr)
            .ok_or_else(|| UpgradeError::Config("Bridgehub address not found in state".into()))?;

        let create2_factory_addr = contracts.create2_factory_addr;
        let create2_factory_salt = contracts.create2_factory_salt;

        Ok(Self {
            l1_rpc_url,
            ecosystem_name: ecosystem_name.to_string(),
            governor_address,
            deployer_address,
            governor_private_key,
            deployer_private_key,
            bridgehub_address,
            create2_factory_addr,
            create2_factory_salt,
            state_dir,
            gas_multiplier,
        })
    }
}
