//! Ecosystem-level state operations.

use crate::backend::StateBackend;
use crate::error::Result;
use crate::manager::{deserialize_yaml, serialize_yaml};
use crate::paths;
use adi_types::{
    Apps, EcosystemContracts, EcosystemMetadata, Erc20Deployments, InitialDeployments,
    PartialEcosystemMetadata, Wallets,
};
use std::path::PathBuf;
use std::sync::Arc;

/// Ecosystem-level state operations.
///
/// Provides typed access to ecosystem configuration files.
pub struct EcosystemStateOps {
    backend: Arc<dyn StateBackend>,
}

impl EcosystemStateOps {
    /// Create new ecosystem state operations.
    pub(crate) fn new(backend: Arc<dyn StateBackend>) -> Self {
        Self { backend }
    }

    // ========== METADATA ==========

    /// Read ecosystem metadata (ZkStack.yaml).
    ///
    /// # Errors
    ///
    /// Returns error if file doesn't exist or parsing fails.
    pub async fn metadata(&self) -> Result<EcosystemMetadata> {
        let content = self.backend.read(paths::ECOSYSTEM_METADATA).await?;
        deserialize_yaml(&content, &PathBuf::from(paths::ECOSYSTEM_METADATA))
    }

    /// Update ecosystem metadata with partial values.
    ///
    /// Performs read-modify-write: reads current metadata, merges with
    /// partial, and writes back.
    ///
    /// # Errors
    ///
    /// Returns error if file doesn't exist.
    pub async fn update_metadata(&self, partial: &PartialEcosystemMetadata) -> Result<()> {
        let current = self.metadata().await?;
        let merged = merge_ecosystem_metadata(current, partial);
        let yaml = serialize_yaml(&merged, &PathBuf::from(paths::ECOSYSTEM_METADATA))?;
        self.backend.write(paths::ECOSYSTEM_METADATA, &yaml).await
    }

    // ========== WALLETS ==========

    /// Read ecosystem wallets (configs/wallets.yaml).
    ///
    /// # Errors
    ///
    /// Returns error if file doesn't exist or parsing fails.
    pub async fn wallets(&self) -> Result<Wallets> {
        let key = paths::ecosystem_wallets_path();
        let content = self.backend.read(&key).await?;
        deserialize_yaml(&content, &PathBuf::from(&key))
    }

    /// Update ecosystem wallets.
    ///
    /// Performs read-modify-write with merge.
    ///
    /// # Errors
    ///
    /// Returns error if file doesn't exist.
    pub async fn update_wallets(&self, partial: &Wallets) -> Result<()> {
        let current = self.wallets().await?;
        let merged = merge_wallets(current, partial);
        let key = paths::ecosystem_wallets_path();
        let yaml = serialize_yaml(&merged, &PathBuf::from(&key))?;
        self.backend.write(&key, &yaml).await
    }

    // ========== CONTRACTS ==========

    /// Read ecosystem contracts (configs/contracts.yaml).
    ///
    /// Note: This file only exists after deployment.
    ///
    /// # Errors
    ///
    /// Returns error if file doesn't exist or parsing fails.
    pub async fn contracts(&self) -> Result<EcosystemContracts> {
        let key = paths::ecosystem_contracts_path();
        let content = self.backend.read(&key).await?;
        deserialize_yaml(&content, &PathBuf::from(&key))
    }

    /// Check if contracts file exists.
    ///
    /// # Errors
    ///
    /// Returns error if checking existence fails.
    pub async fn contracts_exist(&self) -> Result<bool> {
        self.backend.exists(&paths::ecosystem_contracts_path()).await
    }

    /// Update ecosystem contracts.
    ///
    /// # Errors
    ///
    /// Returns error if file doesn't exist.
    pub async fn update_contracts(&self, contracts: &EcosystemContracts) -> Result<()> {
        let key = paths::ecosystem_contracts_path();
        let yaml = serialize_yaml(contracts, &PathBuf::from(&key))?;
        self.backend.write(&key, &yaml).await
    }

    // ========== INITIAL DEPLOYMENTS ==========

    /// Read initial deployments (configs/initial_deployments.yaml).
    ///
    /// # Errors
    ///
    /// Returns error if file doesn't exist or parsing fails.
    pub async fn initial_deployments(&self) -> Result<InitialDeployments> {
        let key = paths::initial_deployments_path();
        let content = self.backend.read(&key).await?;
        deserialize_yaml(&content, &PathBuf::from(&key))
    }

    /// Update initial deployments.
    ///
    /// # Errors
    ///
    /// Returns error if file doesn't exist.
    pub async fn update_initial_deployments(&self, deployments: &InitialDeployments) -> Result<()> {
        let key = paths::initial_deployments_path();
        let yaml = serialize_yaml(deployments, &PathBuf::from(&key))?;
        self.backend.write(&key, &yaml).await
    }

    // ========== ERC20 DEPLOYMENTS ==========

    /// Read ERC20 deployments (configs/erc20_deployments.yaml).
    ///
    /// # Errors
    ///
    /// Returns error if file doesn't exist or parsing fails.
    pub async fn erc20_deployments(&self) -> Result<Erc20Deployments> {
        let key = paths::erc20_deployments_path();
        let content = self.backend.read(&key).await?;
        deserialize_yaml(&content, &PathBuf::from(&key))
    }

    /// Update ERC20 deployments.
    ///
    /// # Errors
    ///
    /// Returns error if file doesn't exist.
    pub async fn update_erc20_deployments(&self, deployments: &Erc20Deployments) -> Result<()> {
        let key = paths::erc20_deployments_path();
        let yaml = serialize_yaml(deployments, &PathBuf::from(&key))?;
        self.backend.write(&key, &yaml).await
    }

    // ========== APPS ==========

    /// Read apps config (configs/apps.yaml).
    ///
    /// # Errors
    ///
    /// Returns error if file doesn't exist or parsing fails.
    pub async fn apps(&self) -> Result<Apps> {
        let key = paths::apps_path();
        let content = self.backend.read(&key).await?;
        deserialize_yaml(&content, &PathBuf::from(&key))
    }

    /// Update apps config.
    ///
    /// # Errors
    ///
    /// Returns error if file doesn't exist.
    pub async fn update_apps(&self, apps: &Apps) -> Result<()> {
        let key = paths::apps_path();
        let yaml = serialize_yaml(apps, &PathBuf::from(&key))?;
        self.backend.write(&key, &yaml).await
    }
}

/// Merge partial ecosystem metadata into current metadata.
///
/// Only overwrites fields that are `Some` in partial.
fn merge_ecosystem_metadata(
    mut current: EcosystemMetadata,
    partial: &PartialEcosystemMetadata,
) -> EcosystemMetadata {
    if let Some(ref name) = partial.name {
        current.name.clone_from(name);
    }
    if let Some(ref network) = partial.l1_network {
        current.l1_network = network.clone();
    }
    if let Some(ref link) = partial.link_to_code {
        current.link_to_code.clone_from(link);
    }
    if let Some(ref chains) = partial.chains {
        current.chains.clone_from(chains);
    }
    if let Some(ref config) = partial.config {
        current.config.clone_from(config);
    }
    if let Some(ref default_chain) = partial.default_chain {
        current.default_chain.clone_from(default_chain);
    }
    if let Some(era_chain_id) = partial.era_chain_id {
        current.era_chain_id = era_chain_id;
    }
    if let Some(ref prover_version) = partial.prover_version {
        current.prover_version = prover_version.clone();
    }
    if let Some(ref wallet_creation) = partial.wallet_creation {
        current.wallet_creation = wallet_creation.clone();
    }
    current
}

/// Merge partial wallets into current wallets.
///
/// Only overwrites wallet slots that are `Some` in partial.
pub(crate) fn merge_wallets(mut current: Wallets, partial: &Wallets) -> Wallets {
    if partial.deployer.is_some() {
        current.deployer.clone_from(&partial.deployer);
    }
    if partial.operator.is_some() {
        current.operator.clone_from(&partial.operator);
    }
    if partial.blob_operator.is_some() {
        current.blob_operator.clone_from(&partial.blob_operator);
    }
    if partial.prove_operator.is_some() {
        current.prove_operator.clone_from(&partial.prove_operator);
    }
    if partial.execute_operator.is_some() {
        current
            .execute_operator
            .clone_from(&partial.execute_operator);
    }
    if partial.fee_account.is_some() {
        current.fee_account.clone_from(&partial.fee_account);
    }
    if partial.governor.is_some() {
        current.governor.clone_from(&partial.governor);
    }
    if partial.token_multiplier_setter.is_some() {
        current
            .token_multiplier_setter
            .clone_from(&partial.token_multiplier_setter);
    }
    current
}

#[cfg(test)]
mod tests {
    use super::*;
    use adi_types::{L1Network, ProverMode, WalletCreation};

    #[test]
    fn test_merge_ecosystem_metadata_partial() {
        let current = EcosystemMetadata {
            name: "original".to_string(),
            l1_network: L1Network::Sepolia,
            link_to_code: "/old/path".to_string(),
            chains: "/old/chains".to_string(),
            config: "/old/config".to_string(),
            default_chain: "old_chain".to_string(),
            era_chain_id: 270,
            prover_version: ProverMode::NoProofs,
            wallet_creation: WalletCreation::Random,
        };

        let partial = PartialEcosystemMetadata {
            name: Some("updated".to_string()),
            default_chain: Some("new_chain".to_string()),
            ..Default::default()
        };

        let merged = merge_ecosystem_metadata(current, &partial);

        assert_eq!(merged.name, "updated");
        assert_eq!(merged.default_chain, "new_chain");
        // Unchanged fields
        assert_eq!(merged.l1_network, L1Network::Sepolia);
        assert_eq!(merged.link_to_code, "/old/path");
        assert_eq!(merged.era_chain_id, 270);
    }

    #[test]
    fn test_merge_ecosystem_metadata_empty_partial() {
        let current = EcosystemMetadata {
            name: "original".to_string(),
            l1_network: L1Network::Sepolia,
            link_to_code: "/old/path".to_string(),
            chains: "/old/chains".to_string(),
            config: "/old/config".to_string(),
            default_chain: "old_chain".to_string(),
            era_chain_id: 270,
            prover_version: ProverMode::NoProofs,
            wallet_creation: WalletCreation::Random,
        };

        let partial = PartialEcosystemMetadata::default();
        let merged = merge_ecosystem_metadata(current.clone(), &partial);

        assert_eq!(merged.name, current.name);
        assert_eq!(merged.l1_network, current.l1_network);
        assert_eq!(merged.default_chain, current.default_chain);
    }
}
