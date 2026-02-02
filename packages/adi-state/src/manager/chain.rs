//! Chain-level state operations.

use crate::backend::StateBackend;
use crate::error::Result;
use crate::manager::ecosystem::merge_wallets;
use crate::manager::{deserialize_yaml, serialize_yaml};
use crate::paths;
use adi_types::{ChainContracts, ChainMetadata, PartialChainMetadata, Wallets};
use std::path::PathBuf;
use std::sync::Arc;

/// Chain-level state operations.
///
/// Provides typed access to chain configuration files.
pub struct ChainStateOps {
    backend: Arc<dyn StateBackend>,
    chain_name: String,
}

impl ChainStateOps {
    /// Create new chain state operations.
    pub(crate) fn new(backend: Arc<dyn StateBackend>, chain_name: String) -> Self {
        Self { backend, chain_name }
    }

    /// Get the chain name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.chain_name
    }

    // ========== METADATA ==========

    /// Read chain metadata (chains/{name}/ZkStack.yaml).
    ///
    /// # Errors
    ///
    /// Returns error if file doesn't exist or parsing fails.
    pub async fn metadata(&self) -> Result<ChainMetadata> {
        let key = paths::chain_metadata_path(&self.chain_name);
        log::debug!("Reading chain '{}' metadata from {}", self.chain_name, key);
        let content = self.backend.read(&key).await?;
        let metadata: ChainMetadata = deserialize_yaml(&content, &PathBuf::from(&key))?;
        log::debug!(
            "Loaded chain metadata: name={}, chain_id={}",
            metadata.name,
            metadata.chain_id
        );
        Ok(metadata)
    }

    /// Update chain metadata with partial values.
    ///
    /// Performs read-modify-write.
    ///
    /// # Errors
    ///
    /// Returns error if file doesn't exist.
    pub async fn update_metadata(&self, partial: &PartialChainMetadata) -> Result<()> {
        log::debug!("Updating chain '{}' metadata with partial values", self.chain_name);
        let current = self.metadata().await?;
        let merged = merge_chain_metadata(current, partial);
        let key = paths::chain_metadata_path(&self.chain_name);
        let yaml = serialize_yaml(&merged, &PathBuf::from(&key))?;
        self.backend.write(&key, &yaml).await?;
        log::debug!("Chain '{}' metadata updated successfully", self.chain_name);
        Ok(())
    }

    // ========== WALLETS ==========

    /// Read chain wallets (chains/{name}/configs/wallets.yaml).
    ///
    /// # Errors
    ///
    /// Returns error if file doesn't exist or parsing fails.
    pub async fn wallets(&self) -> Result<Wallets> {
        let key = paths::chain_wallets_path(&self.chain_name);
        log::debug!("Reading chain '{}' wallets from {}", self.chain_name, key);
        let content = self.backend.read(&key).await?;
        let wallets: Wallets = deserialize_yaml(&content, &PathBuf::from(&key))?;
        log::debug!(
            "Loaded chain '{}' wallets: deployer={}, governor={}",
            self.chain_name,
            wallets.deployer.is_some(),
            wallets.governor.is_some()
        );
        Ok(wallets)
    }

    /// Update chain wallets.
    ///
    /// Performs read-modify-write with merge.
    ///
    /// # Errors
    ///
    /// Returns error if file doesn't exist.
    pub async fn update_wallets(&self, partial: &Wallets) -> Result<()> {
        log::debug!("Updating chain '{}' wallets", self.chain_name);
        let current = self.wallets().await?;
        let merged = merge_wallets(current, partial);
        let key = paths::chain_wallets_path(&self.chain_name);
        let yaml = serialize_yaml(&merged, &PathBuf::from(&key))?;
        self.backend.write(&key, &yaml).await?;
        log::debug!("Chain '{}' wallets updated successfully", self.chain_name);
        Ok(())
    }

    // ========== CONTRACTS ==========

    /// Read chain contracts (chains/{name}/configs/contracts.yaml).
    ///
    /// Note: This file only exists after chain deployment.
    ///
    /// # Errors
    ///
    /// Returns error if file doesn't exist or parsing fails.
    pub async fn contracts(&self) -> Result<ChainContracts> {
        let key = paths::chain_contracts_path(&self.chain_name);
        log::debug!("Reading chain '{}' contracts from {}", self.chain_name, key);
        let content = self.backend.read(&key).await?;
        let contracts: ChainContracts = deserialize_yaml(&content, &PathBuf::from(&key))?;
        log::debug!("Loaded chain '{}' contracts successfully", self.chain_name);
        Ok(contracts)
    }

    /// Check if contracts file exists.
    ///
    /// # Errors
    ///
    /// Returns error if checking existence fails.
    pub async fn contracts_exist(&self) -> Result<bool> {
        let key = paths::chain_contracts_path(&self.chain_name);
        let exists = self.backend.exists(&key).await?;
        log::debug!("Chain '{}' contracts file exists: {}", self.chain_name, exists);
        Ok(exists)
    }

    /// Update chain contracts.
    ///
    /// # Errors
    ///
    /// Returns error if file doesn't exist.
    pub async fn update_contracts(&self, contracts: &ChainContracts) -> Result<()> {
        let key = paths::chain_contracts_path(&self.chain_name);
        log::debug!("Updating chain '{}' contracts", self.chain_name);
        let yaml = serialize_yaml(contracts, &PathBuf::from(&key))?;
        self.backend.write(&key, &yaml).await?;
        log::debug!("Chain '{}' contracts updated successfully", self.chain_name);
        Ok(())
    }

    /// Check if this chain exists (has metadata file).
    ///
    /// # Errors
    ///
    /// Returns error if checking existence fails.
    pub async fn exists(&self) -> Result<bool> {
        let key = paths::chain_metadata_path(&self.chain_name);
        let exists = self.backend.exists(&key).await?;
        log::debug!("Chain '{}' exists: {}", self.chain_name, exists);
        Ok(exists)
    }
}

/// Merge partial chain metadata into current metadata.
fn merge_chain_metadata(mut current: ChainMetadata, partial: &PartialChainMetadata) -> ChainMetadata {
    if let Some(id) = partial.id {
        current.id = id;
    }
    if let Some(ref name) = partial.name {
        current.name.clone_from(name);
    }
    if let Some(chain_id) = partial.chain_id {
        current.chain_id = chain_id;
    }
    if let Some(ref prover_version) = partial.prover_version {
        current.prover_version = prover_version.clone();
    }
    if let Some(ref l1_network) = partial.l1_network {
        current.l1_network = l1_network.clone();
    }
    if let Some(ref link_to_code) = partial.link_to_code {
        current.link_to_code.clone_from(link_to_code);
    }
    if let Some(ref configs) = partial.configs {
        current.configs.clone_from(configs);
    }
    if let Some(ref rocks_db_path) = partial.rocks_db_path {
        current.rocks_db_path.clone_from(rocks_db_path);
    }
    if let Some(ref external_node_config_path) = partial.external_node_config_path {
        current.external_node_config_path = Some(external_node_config_path.clone());
    }
    if let Some(ref artifacts_path) = partial.artifacts_path {
        current.artifacts_path.clone_from(artifacts_path);
    }
    if let Some(ref mode) = partial.l1_batch_commit_data_generator_mode {
        current.l1_batch_commit_data_generator_mode = mode.clone();
    }
    if let Some(ref base_token) = partial.base_token {
        current.base_token = base_token.clone();
    }
    if let Some(ref wallet_creation) = partial.wallet_creation {
        current.wallet_creation = wallet_creation.clone();
    }
    if let Some(evm_emulator) = partial.evm_emulator {
        current.evm_emulator = evm_emulator;
    }
    if let Some(tight_ports) = partial.tight_ports {
        current.tight_ports = tight_ports;
    }
    if let Some(ref vm_option) = partial.vm_option {
        current.vm_option = vm_option.clone();
    }
    if let Some(ref contracts_path) = partial.contracts_path {
        current.contracts_path.clone_from(contracts_path);
    }
    if let Some(ref default_configs_path) = partial.default_configs_path {
        current.default_configs_path.clone_from(default_configs_path);
    }
    current
}

#[cfg(test)]
mod tests {
    use super::*;
    use adi_types::{BaseToken, BatchCommitDataMode, L1Network, ProverMode, VmOption, WalletCreation};

    #[test]
    fn test_merge_chain_metadata_partial() {
        let current = ChainMetadata {
            id: 1,
            name: "original".to_string(),
            chain_id: 100,
            prover_version: ProverMode::NoProofs,
            l1_network: L1Network::Sepolia,
            link_to_code: "/old/path".to_string(),
            configs: "/old/configs".to_string(),
            rocks_db_path: "/old/db".to_string(),
            external_node_config_path: None,
            artifacts_path: "/old/artifacts".to_string(),
            l1_batch_commit_data_generator_mode: BatchCommitDataMode::Rollup,
            base_token: BaseToken::eth(),
            wallet_creation: WalletCreation::Random,
            evm_emulator: false,
            tight_ports: false,
            vm_option: VmOption::ZKSyncOsVM,
            contracts_path: "/old/contracts".to_string(),
            default_configs_path: "/old/defaults".to_string(),
        };

        let partial = PartialChainMetadata {
            chain_id: Some(200),
            evm_emulator: Some(true),
            ..Default::default()
        };

        let merged = merge_chain_metadata(current, &partial);

        assert_eq!(merged.chain_id, 200);
        assert!(merged.evm_emulator);
        // Unchanged fields
        assert_eq!(merged.id, 1);
        assert_eq!(merged.name, "original");
        assert_eq!(merged.l1_network, L1Network::Sepolia);
    }

    #[test]
    fn test_merge_chain_metadata_empty_partial() {
        let current = ChainMetadata {
            id: 1,
            name: "original".to_string(),
            chain_id: 100,
            prover_version: ProverMode::NoProofs,
            l1_network: L1Network::Sepolia,
            link_to_code: "/path".to_string(),
            configs: "/configs".to_string(),
            rocks_db_path: "/db".to_string(),
            external_node_config_path: None,
            artifacts_path: "/artifacts".to_string(),
            l1_batch_commit_data_generator_mode: BatchCommitDataMode::Rollup,
            base_token: BaseToken::eth(),
            wallet_creation: WalletCreation::Random,
            evm_emulator: false,
            tight_ports: false,
            vm_option: VmOption::ZKSyncOsVM,
            contracts_path: "/contracts".to_string(),
            default_configs_path: "/defaults".to_string(),
        };

        let partial = PartialChainMetadata::default();
        let merged = merge_chain_metadata(current.clone(), &partial);

        assert_eq!(merged.id, current.id);
        assert_eq!(merged.name, current.name);
        assert_eq!(merged.chain_id, current.chain_id);
    }
}
