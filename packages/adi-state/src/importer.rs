//! State importer for reading zkstack output and storing through backend.
//!
//! This module provides functionality to import state from a source directory
//! (e.g., temp directory where zkstack ran) and store it through the StateManager.

use crate::error::{Result, StateError};
use crate::StateManager;
use adi_types::{ChainMetadata, EcosystemMetadata, Wallets};
use serde::de::DeserializeOwned;
use std::path::Path;
use tokio::fs;

/// Read and deserialize a YAML file from the filesystem.
async fn read_yaml_file<T: DeserializeOwned>(path: &Path) -> Result<T> {
    let content = fs::read_to_string(path)
        .await
        .map_err(|e| StateError::ReadFailed {
            path: path.to_path_buf(),
            source: e,
        })?;

    serde_yaml::from_str(&content).map_err(|e| StateError::YamlParseFailed {
        path: path.to_path_buf(),
        source: e,
    })
}

/// Import ecosystem state from a source directory.
///
/// Reads YAML files from the source directory (e.g., temp dir where zkstack ran)
/// and stores them through the StateManager backend.
///
/// # Arguments
///
/// * `state_manager` - StateManager configured with target backend.
/// * `source_dir` - Directory containing zkstack output.
/// * `ecosystem_name` - Name of the ecosystem.
/// * `chain_name` - Name of the default chain.
///
/// # Errors
///
/// Returns error if reading or creating state fails.
pub async fn import_ecosystem_state(
    state_manager: &StateManager,
    source_dir: &Path,
    ecosystem_name: &str,
    chain_name: &str,
) -> Result<()> {
    let ecosystem_dir = source_dir.join(ecosystem_name);
    log::info!("Importing ecosystem state from {}", ecosystem_dir.display());

    // Import ecosystem-level files
    import_ecosystem_metadata(state_manager, &ecosystem_dir).await?;
    import_ecosystem_wallets(state_manager, &ecosystem_dir).await?;

    // Import chain-level files
    import_chain_metadata(state_manager, &ecosystem_dir, chain_name).await?;
    import_chain_wallets(state_manager, &ecosystem_dir, chain_name).await?;

    log::info!("Ecosystem state imported successfully");
    Ok(())
}

async fn import_ecosystem_metadata(
    state_manager: &StateManager,
    ecosystem_dir: &Path,
) -> Result<()> {
    let path = ecosystem_dir.join("ZkStack.yaml");
    log::debug!("Importing ecosystem metadata from {}", path.display());

    let mut metadata: EcosystemMetadata = read_yaml_file(&path).await?;

    // Transform paths from /workspace/<ecosystem_name>/... to /workspace/...
    // Container mounts ecosystem dir directly to /workspace during deployment
    metadata.chains = "/workspace/chains".to_string();
    metadata.config = "/workspace/configs/".to_string();

    state_manager.ecosystem().create_metadata(&metadata).await
}

async fn import_ecosystem_wallets(
    state_manager: &StateManager,
    ecosystem_dir: &Path,
) -> Result<()> {
    let path = ecosystem_dir.join("configs").join("wallets.yaml");
    log::debug!("Importing ecosystem wallets from {}", path.display());

    let wallets: Wallets = read_yaml_file(&path).await?;
    state_manager.ecosystem().create_wallets(&wallets).await
}

async fn import_chain_metadata(
    state_manager: &StateManager,
    ecosystem_dir: &Path,
    chain_name: &str,
) -> Result<()> {
    let path = ecosystem_dir
        .join("chains")
        .join(chain_name)
        .join("ZkStack.yaml");
    log::debug!(
        "Importing chain '{}' metadata from {}",
        chain_name,
        path.display()
    );

    let mut metadata: ChainMetadata = read_yaml_file(&path).await?;

    // Transform paths from /workspace/<ecosystem_name>/chains/... to /workspace/chains/...
    // Container mounts ecosystem dir directly to /workspace during deployment
    metadata.configs = format!("/workspace/chains/{}/configs/", chain_name);
    metadata.rocks_db_path = format!("/workspace/chains/{}/db/", chain_name);
    metadata.artifacts_path = format!("/workspace/chains/{}/artifacts/", chain_name);

    state_manager
        .chain(chain_name)
        .create_metadata(&metadata)
        .await
}

async fn import_chain_wallets(
    state_manager: &StateManager,
    ecosystem_dir: &Path,
    chain_name: &str,
) -> Result<()> {
    let path = ecosystem_dir
        .join("chains")
        .join(chain_name)
        .join("configs")
        .join("wallets.yaml");
    log::debug!(
        "Importing chain '{}' wallets from {}",
        chain_name,
        path.display()
    );

    let wallets: Wallets = read_yaml_file(&path).await?;
    state_manager
        .chain(chain_name)
        .create_wallets(&wallets)
        .await
}
