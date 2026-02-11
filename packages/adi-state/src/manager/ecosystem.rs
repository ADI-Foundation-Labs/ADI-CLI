//! Ecosystem-level state operations.

use crate::backend::StateBackend;
use crate::error::Result;
use crate::paths;
use adi_types::{
    Apps, EcosystemContracts, EcosystemMetadata, Erc20Deployments, InitialDeployments, Logger,
    PartialEcosystemMetadata, Wallets,
};
use std::sync::Arc;

/// Ecosystem-level state operations.
///
/// Provides typed access to ecosystem configuration files.
pub struct EcosystemStateOps {
    backend: Arc<dyn StateBackend>,
    logger: Arc<dyn Logger>,
}

impl EcosystemStateOps {
    /// Create new ecosystem state operations.
    pub(crate) fn new(backend: Arc<dyn StateBackend>, logger: Arc<dyn Logger>) -> Self {
        Self { backend, logger }
    }

    // ========== METADATA ==========

    /// Read ecosystem metadata (ZkStack.yaml).
    ///
    /// # Errors
    ///
    /// Returns error if file doesn't exist or parsing fails.
    pub async fn metadata(&self) -> Result<EcosystemMetadata> {
        self.backend.read_ecosystem_metadata().await
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
        self.logger
            .debug("Updating ecosystem metadata with partial values");
        let current = self.metadata().await?;
        let merged = merge_ecosystem_metadata(current, partial);
        self.backend.write_ecosystem_metadata(&merged).await?;
        self.logger.debug("Ecosystem metadata updated successfully");
        Ok(())
    }

    // ========== WALLETS ==========

    /// Read ecosystem wallets (configs/wallets.yaml).
    ///
    /// # Errors
    ///
    /// Returns error if file doesn't exist or parsing fails.
    pub async fn wallets(&self) -> Result<Wallets> {
        self.backend.read_ecosystem_wallets().await
    }

    /// Update ecosystem wallets.
    ///
    /// Performs read-modify-write with merge.
    ///
    /// # Errors
    ///
    /// Returns error if file doesn't exist.
    pub async fn update_wallets(&self, partial: &Wallets) -> Result<()> {
        self.logger.debug("Updating ecosystem wallets");
        let current = self.wallets().await?;
        let merged = merge_wallets(current, partial);
        self.backend.write_ecosystem_wallets(&merged).await?;
        self.logger.debug("Ecosystem wallets updated successfully");
        Ok(())
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
        self.backend.read_ecosystem_contracts().await
    }

    /// Check if contracts file exists.
    ///
    /// # Errors
    ///
    /// Returns error if checking existence fails.
    pub async fn contracts_exist(&self) -> Result<bool> {
        let key = paths::ecosystem_contracts_path();
        let exists = self.backend.exists(&key).await?;
        self.logger
            .debug(&format!("Ecosystem contracts file exists: {}", exists));
        Ok(exists)
    }

    /// Update ecosystem contracts.
    ///
    /// # Errors
    ///
    /// Returns error if file doesn't exist.
    pub async fn update_contracts(&self, contracts: &EcosystemContracts) -> Result<()> {
        self.logger.debug("Updating ecosystem contracts");
        self.backend.write_ecosystem_contracts(contracts).await?;
        self.logger
            .debug("Ecosystem contracts updated successfully");
        Ok(())
    }

    // ========== INITIAL DEPLOYMENTS ==========

    /// Read initial deployments (configs/initial_deployments.yaml).
    ///
    /// # Errors
    ///
    /// Returns error if file doesn't exist or parsing fails.
    pub async fn initial_deployments(&self) -> Result<InitialDeployments> {
        self.backend.read_initial_deployments().await
    }

    /// Update initial deployments.
    ///
    /// # Errors
    ///
    /// Returns error if file doesn't exist.
    pub async fn update_initial_deployments(&self, deployments: &InitialDeployments) -> Result<()> {
        self.logger.debug("Updating initial deployments");
        self.backend.write_initial_deployments(deployments).await?;
        self.logger
            .debug("Initial deployments updated successfully");
        Ok(())
    }

    // ========== ERC20 DEPLOYMENTS ==========

    /// Read ERC20 deployments (configs/erc20_deployments.yaml).
    ///
    /// # Errors
    ///
    /// Returns error if file doesn't exist or parsing fails.
    pub async fn erc20_deployments(&self) -> Result<Erc20Deployments> {
        self.backend.read_erc20_deployments().await
    }

    /// Update ERC20 deployments.
    ///
    /// # Errors
    ///
    /// Returns error if file doesn't exist.
    pub async fn update_erc20_deployments(&self, deployments: &Erc20Deployments) -> Result<()> {
        self.logger.debug("Updating ERC20 deployments");
        self.backend.write_erc20_deployments(deployments).await?;
        self.logger.debug("ERC20 deployments updated successfully");
        Ok(())
    }

    // ========== APPS ==========

    /// Read apps config (configs/apps.yaml).
    ///
    /// # Errors
    ///
    /// Returns error if file doesn't exist or parsing fails.
    pub async fn apps(&self) -> Result<Apps> {
        self.backend.read_apps().await
    }

    /// Update apps config.
    ///
    /// # Errors
    ///
    /// Returns error if file doesn't exist.
    pub async fn update_apps(&self, apps: &Apps) -> Result<()> {
        self.logger.debug("Updating apps config");
        self.backend.write_apps(apps).await?;
        self.logger.debug("Apps config updated successfully");
        Ok(())
    }

    // ========== CREATE OPERATIONS ==========

    /// Create ecosystem metadata (ZkStack.yaml).
    ///
    /// # Arguments
    ///
    /// * `metadata` - The ecosystem metadata to create.
    ///
    /// # Errors
    ///
    /// Returns error if file already exists or creation fails.
    pub async fn create_metadata(&self, metadata: &EcosystemMetadata) -> Result<()> {
        self.logger.debug("Creating ecosystem metadata");
        self.backend.create_ecosystem_metadata(metadata).await?;
        self.logger.debug("Ecosystem metadata created successfully");
        Ok(())
    }

    /// Create ecosystem wallets (configs/wallets.yaml).
    ///
    /// # Errors
    ///
    /// Returns error if file already exists or creation fails.
    pub async fn create_wallets(&self, wallets: &Wallets) -> Result<()> {
        self.logger.debug("Creating ecosystem wallets");
        self.backend.create_ecosystem_wallets(wallets).await?;
        self.logger.debug("Ecosystem wallets created successfully");
        Ok(())
    }

    /// Create ecosystem contracts (configs/contracts.yaml).
    ///
    /// # Errors
    ///
    /// Returns error if file already exists or creation fails.
    pub async fn create_contracts(&self, contracts: &EcosystemContracts) -> Result<()> {
        self.logger.debug("Creating ecosystem contracts");
        self.backend.create_ecosystem_contracts(contracts).await?;
        self.logger
            .debug("Ecosystem contracts created successfully");
        Ok(())
    }

    /// Create initial deployments (configs/initial_deployments.yaml).
    ///
    /// # Errors
    ///
    /// Returns error if file already exists or creation fails.
    pub async fn create_initial_deployments(&self, deployments: &InitialDeployments) -> Result<()> {
        self.logger.debug("Creating initial deployments");
        self.backend.create_initial_deployments(deployments).await?;
        self.logger
            .debug("Initial deployments created successfully");
        Ok(())
    }

    /// Create ERC20 deployments (configs/erc20_deployments.yaml).
    ///
    /// # Errors
    ///
    /// Returns error if file already exists or creation fails.
    pub async fn create_erc20_deployments(&self, deployments: &Erc20Deployments) -> Result<()> {
        self.logger.debug("Creating ERC20 deployments");
        self.backend.create_erc20_deployments(deployments).await?;
        self.logger.debug("ERC20 deployments created successfully");
        Ok(())
    }

    /// Create apps config (configs/apps.yaml).
    ///
    /// # Errors
    ///
    /// Returns error if file already exists or creation fails.
    pub async fn create_apps(&self, apps: &Apps) -> Result<()> {
        self.logger.debug("Creating apps config");
        self.backend.create_apps(apps).await?;
        self.logger.debug("Apps config created successfully");
        Ok(())
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
