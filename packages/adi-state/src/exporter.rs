//! State exporter for writing backend state to filesystem for Docker consumption.
//!
//! This module provides functionality to export state from the StateManager
//! to YAML files in a directory structure that zkstack/Docker can consume.

use crate::error::{Result, StateError};
use crate::StateManager;
use serde::Serialize;
use std::path::Path;
use tokio::fs;

/// Export ecosystem state to YAML files for Docker/zkstack consumption.
///
/// Reads state from the backend via StateManager and writes YAML files
/// to the target directory with the structure zkstack expects.
///
/// Always writes YAML format regardless of backend storage format.
///
/// # Arguments
///
/// * `state_manager` - StateManager configured with source backend.
/// * `target_dir` - Directory to write files to (ecosystem root).
///
/// # Output Structure
///
/// ```text
/// target_dir/
/// ├── ZkStack.yaml
/// ├── configs/
/// │   ├── wallets.yaml
/// │   └── contracts.yaml (if exists)
/// └── chains/{chain_name}/
///     ├── ZkStack.yaml
///     └── configs/
///         └── wallets.yaml
/// ```
///
/// # Errors
///
/// Returns error if reading from backend or writing files fails.
pub async fn export_ecosystem_state(state_manager: &StateManager, target_dir: &Path) -> Result<()> {
    state_manager.logger().info(&format!(
        "Exporting ecosystem state to {}",
        target_dir.display()
    ));

    // Create target directory structure
    let configs_dir = target_dir.join("configs");
    fs::create_dir_all(&configs_dir)
        .await
        .map_err(|e| StateError::WriteFailed {
            path: configs_dir.clone(),
            source: e,
        })?;

    // Export ecosystem metadata
    let metadata = state_manager.ecosystem().metadata().await?;
    write_yaml(&target_dir.join("ZkStack.yaml"), &metadata).await?;
    state_manager.logger().debug("Exported ecosystem metadata");

    // Export ecosystem wallets
    let wallets = state_manager.ecosystem().wallets().await?;
    write_yaml(&configs_dir.join("wallets.yaml"), &wallets).await?;
    state_manager.logger().debug("Exported ecosystem wallets");

    // Export contracts if they exist
    if state_manager.ecosystem().contracts_exist().await? {
        let contracts = state_manager.ecosystem().contracts().await?;
        write_yaml(&configs_dir.join("contracts.yaml"), &contracts).await?;
        state_manager.logger().debug("Exported ecosystem contracts");
    }

    // Export all chains
    for chain_name in state_manager.list_chains().await? {
        export_chain_state(state_manager, target_dir, &chain_name).await?;
    }

    state_manager
        .logger()
        .info("Ecosystem state exported successfully");
    Ok(())
}

/// Export a specific chain's state to the target directory.
///
/// # Arguments
///
/// * `state_manager` - StateManager configured with source backend.
/// * `target_dir` - Ecosystem root directory.
/// * `chain_name` - Name of the chain to export.
///
/// # Errors
///
/// Returns error if reading from backend or writing files fails.
pub async fn export_chain_state(
    state_manager: &StateManager,
    target_dir: &Path,
    chain_name: &str,
) -> Result<()> {
    state_manager
        .logger()
        .debug(&format!("Exporting chain '{}' state", chain_name));

    let chain_dir = target_dir.join("chains").join(chain_name);
    let configs_dir = chain_dir.join("configs");
    fs::create_dir_all(&configs_dir)
        .await
        .map_err(|e| StateError::WriteFailed {
            path: configs_dir.clone(),
            source: e,
        })?;

    let chain_ops = state_manager.chain(chain_name);

    // Export chain metadata
    let metadata = chain_ops.metadata().await?;
    write_yaml(&chain_dir.join("ZkStack.yaml"), &metadata).await?;
    state_manager
        .logger()
        .debug(&format!("Exported chain '{}' metadata", chain_name));

    // Export chain wallets
    let wallets = chain_ops.wallets().await?;
    write_yaml(&configs_dir.join("wallets.yaml"), &wallets).await?;
    state_manager
        .logger()
        .debug(&format!("Exported chain '{}' wallets", chain_name));

    // Export contracts if they exist
    if chain_ops.contracts_exist().await? {
        let contracts = chain_ops.contracts().await?;
        write_yaml(&configs_dir.join("contracts.yaml"), &contracts).await?;
        state_manager
            .logger()
            .debug(&format!("Exported chain '{}' contracts", chain_name));
    }

    state_manager.logger().debug(&format!(
        "Chain '{}' state exported successfully",
        chain_name
    ));
    Ok(())
}

/// Write a value as YAML to a file.
async fn write_yaml<T: Serialize>(path: &Path, data: &T) -> Result<()> {
    let yaml = serde_yaml::to_string(data).map_err(|e| StateError::YamlSerializeFailed {
        path: path.to_path_buf(),
        source: e,
    })?;

    // Create parent directories if needed
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)
                .await
                .map_err(|e| StateError::WriteFailed {
                    path: parent.to_path_buf(),
                    source: e,
                })?;
        }
    }

    fs::write(path, yaml)
        .await
        .map_err(|e| StateError::WriteFailed {
            path: path.to_path_buf(),
            source: e,
        })
}
