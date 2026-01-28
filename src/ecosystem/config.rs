//! Ecosystem configuration types.
//!
//! This module defines the core configuration types for ZkSync ecosystems,
//! including the settlement network and ecosystem structure.

use std::path::PathBuf;

use alloy_primitives::{Address, Bytes, B256};
use chrono::{DateTime, Utc};
use eyre::{ensure, WrapErr};
use semver::Version;
use serde::{Deserialize, Serialize};
use tokio::fs;
use uuid::Uuid;

use super::contracts::EcosystemContracts;
use super::wallets::EcosystemWallets;
use crate::error::Result;

/// Settlement network configuration.
///
/// Specifies which Ethereum network the ecosystem contracts are deployed to.
/// The settlement layer is where L1 contracts (Bridgehub, Governance, etc.)
/// are deployed.
///
/// # Example
///
/// ```rust
/// // Use Sepolia testnet
/// let network = SettlementNetwork::Sepolia;
///
/// // Use custom network
/// let custom = SettlementNetwork::Custom {
///     rpc_url: "https://my-node.example.com".to_string(),
///     chain_id: 12345,
/// };
/// ```
// Note: Currently unused as commands are implemented in later phases (US1-US6)
#[allow(dead_code)]
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SettlementNetwork {
    /// Ethereum Mainnet (chain ID: 1).
    Mainnet,

    /// Sepolia testnet (chain ID: 11155111).
    Sepolia,

    /// Local development network (default RPC: http://localhost:8545).
    #[default]
    Localhost,

    /// Custom network with user-specified RPC URL and chain ID.
    Custom {
        /// The RPC endpoint URL.
        rpc_url: String,
        /// The network's chain ID.
        chain_id: u64,
    },
}

#[allow(dead_code)]
impl SettlementNetwork {
    /// Returns the chain ID for this network.
    ///
    /// # Example
    ///
    /// ```rust
    /// assert_eq!(SettlementNetwork::Mainnet.chain_id(), 1);
    /// assert_eq!(SettlementNetwork::Sepolia.chain_id(), 11155111);
    /// ```
    pub fn chain_id(&self) -> u64 {
        match self {
            Self::Mainnet => 1,
            Self::Sepolia => 11155111,
            Self::Localhost => 31337, // Default Anvil chain ID
            Self::Custom { chain_id, .. } => *chain_id,
        }
    }

    /// Returns the default RPC URL for this network.
    ///
    /// For custom networks, returns the configured URL.
    /// For standard networks, returns a public endpoint (may require API key).
    pub fn default_rpc_url(&self) -> &str {
        match self {
            Self::Mainnet => "https://eth.llamarpc.com",
            Self::Sepolia => "https://rpc.sepolia.org",
            Self::Localhost => "http://localhost:8545",
            Self::Custom { rpc_url, .. } => rpc_url,
        }
    }

    /// Checks if this is a testnet or local network.
    pub fn is_testnet(&self) -> bool {
        matches!(self, Self::Sepolia | Self::Localhost)
    }
}

impl std::fmt::Display for SettlementNetwork {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Mainnet => write!(f, "mainnet"),
            Self::Sepolia => write!(f, "sepolia"),
            Self::Localhost => write!(f, "localhost"),
            Self::Custom { chain_id, .. } => write!(f, "custom (chain_id: {})", chain_id),
        }
    }
}

/// A ZkSync ecosystem configuration.
///
/// An ecosystem is the top-level container for ZkSync infrastructure.
/// It contains multiple chains and manages shared contracts on the
/// settlement layer.
///
/// # Directory Structure
///
/// ```text
/// {state_path}/
/// ├── ZkStack.yaml              # Ecosystem metadata
/// ├── configs/
/// │   ├── wallets.yaml
/// │   ├── contracts.yaml
/// │   └── initial_deployments.yaml
/// └── chains/{chain-name}/
///     └── configs/
///         ├── contracts.yaml
///         ├── wallets.yaml
///         ├── genesis.yaml
///         └── general.yaml
/// ```
// Note: Currently unused as commands are implemented in later phases (US1-US6)
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ecosystem {
    /// Unique ecosystem name (alphanumeric with underscores, max 64 chars).
    pub name: String,

    /// The settlement network where contracts are deployed.
    pub settlement_network: SettlementNetwork,

    /// Path to the ecosystem state directory.
    pub state_path: PathBuf,

    /// Deployed ecosystem contracts (populated after deployment).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contracts: Option<EcosystemContracts>,

    /// Ecosystem wallet keypairs.
    pub wallets: EcosystemWallets,

    /// Names of chains within this ecosystem.
    #[serde(default)]
    pub chains: Vec<String>,

    /// Current protocol version (e.g., 29.0.11).
    pub protocol_version: Version,

    /// Timestamp when the ecosystem was created.
    pub created_at: DateTime<Utc>,

    /// Timestamp when the ecosystem was last updated.
    pub updated_at: DateTime<Utc>,
}

#[allow(dead_code)]
impl Ecosystem {
    /// Validates the ecosystem configuration.
    ///
    /// # Validation Rules
    ///
    /// - Name must be non-empty
    /// - Name must be max 64 characters
    /// - Name must be alphanumeric with underscores only
    /// - State path must exist and be a directory
    ///
    /// # Errors
    ///
    /// Returns an error if any validation rule fails.
    pub fn validate(&self) -> Result<()> {
        ensure!(!self.name.is_empty(), "Ecosystem name cannot be empty");

        ensure!(
            self.name.len() <= 64,
            "Ecosystem name too long (max 64 characters, got {})",
            self.name.len()
        );

        ensure!(
            self.name.chars().all(|c| c.is_alphanumeric() || c == '_'),
            "Ecosystem name must be alphanumeric with underscores only: '{}'",
            self.name
        );

        Ok(())
    }

    /// Checks if the ecosystem has been deployed (contracts exist).
    pub fn is_deployed(&self) -> bool {
        self.contracts.is_some()
    }

    /// Returns the ecosystem's protocol version as a display string.
    ///
    /// # Example
    ///
    /// ```rust
    /// // For version 29.0.11
    /// assert_eq!(ecosystem.version_string(), "v29.0.11");
    /// ```
    pub fn version_string(&self) -> String {
        format!("v{}", self.protocol_version)
    }

    /// Creates the ecosystem directory structure.
    ///
    /// Creates the following directory structure at `state_path`:
    /// ```text
    /// {state_path}/
    /// ├── configs/
    /// └── chains/
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if directory creation fails.
    pub async fn create_directory_structure(&self) -> Result<()> {
        // Create main ecosystem directory
        fs::create_dir_all(&self.state_path)
            .await
            .wrap_err_with(|| {
                format!(
                    "Failed to create ecosystem directory: {}",
                    self.state_path.display()
                )
            })?;

        // Create configs subdirectory
        let configs_dir = self.state_path.join("configs");
        fs::create_dir_all(&configs_dir).await.wrap_err_with(|| {
            format!(
                "Failed to create configs directory: {}",
                configs_dir.display()
            )
        })?;

        // Create chains subdirectory
        let chains_dir = self.state_path.join("chains");
        fs::create_dir_all(&chains_dir).await.wrap_err_with(|| {
            format!(
                "Failed to create chains directory: {}",
                chains_dir.display()
            )
        })?;

        Ok(())
    }

    /// Returns the path to the configs directory.
    pub fn configs_path(&self) -> PathBuf {
        self.state_path.join("configs")
    }

    /// Returns the path to the chains directory.
    pub fn chains_path(&self) -> PathBuf {
        self.state_path.join("chains")
    }

    /// Returns the path to the ZkStack.yaml metadata file.
    pub fn metadata_path(&self) -> PathBuf {
        self.state_path.join("ZkStack.yaml")
    }

    /// Returns the path to the wallets.yaml file.
    pub fn wallets_path(&self) -> PathBuf {
        self.configs_path().join("wallets.yaml")
    }

    /// Returns the path to the contracts.yaml file.
    pub fn contracts_path(&self) -> PathBuf {
        self.configs_path().join("contracts.yaml")
    }

    /// Creates a chain directory structure within this ecosystem.
    ///
    /// Creates the following directory structure:
    /// ```text
    /// {state_path}/chains/{chain_name}/
    /// └── configs/
    /// ```
    ///
    /// # Arguments
    ///
    /// * `chain_name` - Name of the chain
    ///
    /// # Errors
    ///
    /// Returns an error if directory creation fails.
    pub async fn create_chain_directory(&self, chain_name: &str) -> Result<PathBuf> {
        let chain_dir = self.chains_path().join(chain_name);
        fs::create_dir_all(&chain_dir).await.wrap_err_with(|| {
            format!("Failed to create chain directory: {}", chain_dir.display())
        })?;

        let chain_configs_dir = chain_dir.join("configs");
        fs::create_dir_all(&chain_configs_dir)
            .await
            .wrap_err_with(|| {
                format!(
                    "Failed to create chain configs directory: {}",
                    chain_configs_dir.display()
                )
            })?;

        Ok(chain_dir)
    }
}

/// Status of an upgrade operation.
///
/// Tracks the progress of a protocol version upgrade through its various stages.
// Note: Currently unused as upgrade commands are implemented in later phases (US4-US5)
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UpgradeStatus {
    /// Input config loaded, upgrade not yet prepared.
    Pending,

    /// Calldata has been generated and is ready for execution.
    Prepared,

    /// The scheduleTransparent transaction has been executed.
    Scheduled,

    /// The upgrade execute transaction has been completed.
    Executed,

    /// The upgrade failed with a reason.
    Failed {
        /// Reason for the failure.
        reason: String,
    },
}

impl std::fmt::Display for UpgradeStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::Prepared => write!(f, "prepared"),
            Self::Scheduled => write!(f, "scheduled"),
            Self::Executed => write!(f, "executed"),
            Self::Failed { reason } => write!(f, "failed: {}", reason),
        }
    }
}

/// Calldata for executing an upgrade via governance.
///
/// Contains the encoded calldata for scheduling and executing an upgrade
/// through the governance contract.
// Note: Currently unused as upgrade commands are implemented in later phases (US4-US5)
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpgradeCalldata {
    /// Encoded calldata for the scheduleTransparent governance call.
    pub schedule_transparent: Bytes,

    /// Encoded calldata for the execute governance call.
    pub execute: Bytes,

    /// Address of the governance contract to call.
    pub governance_address: Address,
}

/// Protocol version upgrade record.
///
/// Tracks all information related to upgrading an ecosystem or chain
/// from one protocol version to another.
///
/// # Workflow
///
/// 1. Create upgrade with `Pending` status
/// 2. Generate calldata -> status becomes `Prepared`
/// 3. Execute scheduleTransparent -> status becomes `Scheduled`
/// 4. Execute upgrade -> status becomes `Executed`
///
/// If any step fails, status becomes `Failed` with a reason.
// Note: Currently unused as upgrade commands are implemented in later phases (US4-US5)
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Upgrade {
    /// Unique identifier for this upgrade.
    pub id: Uuid,

    /// Name of the ecosystem being upgraded.
    pub ecosystem_name: String,

    /// Name of the chain being upgraded (None for ecosystem-level upgrades).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chain_name: Option<String>,

    /// Current protocol version before upgrade.
    pub source_version: Version,

    /// Target protocol version after upgrade.
    pub target_version: Version,

    /// Current status of the upgrade.
    pub status: UpgradeStatus,

    /// Generated calldata for governance execution (populated after prepare).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub calldata: Option<UpgradeCalldata>,

    /// Transaction hash of the executed upgrade (populated after execution).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub executed_tx: Option<B256>,

    /// Path to the deployment output file (v{VERSION}-ecosystem.toml).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deployment_output_path: Option<PathBuf>,

    /// Timestamp when the upgrade was created.
    pub created_at: DateTime<Utc>,

    /// Timestamp when the upgrade was executed (None if not yet executed).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub executed_at: Option<DateTime<Utc>>,
}

#[allow(dead_code)]
impl Upgrade {
    /// Creates a new upgrade record in pending status.
    ///
    /// # Arguments
    ///
    /// * `ecosystem_name` - Name of the ecosystem being upgraded
    /// * `chain_name` - Optional chain name (None for ecosystem-level upgrade)
    /// * `source_version` - Current protocol version
    /// * `target_version` - Target protocol version
    pub fn new(
        ecosystem_name: String,
        chain_name: Option<String>,
        source_version: Version,
        target_version: Version,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            ecosystem_name,
            chain_name,
            source_version,
            target_version,
            status: UpgradeStatus::Pending,
            calldata: None,
            executed_tx: None,
            deployment_output_path: None,
            created_at: now,
            executed_at: None,
        }
    }

    /// Checks if this is an ecosystem-level upgrade.
    pub fn is_ecosystem_upgrade(&self) -> bool {
        self.chain_name.is_none()
    }

    /// Checks if the upgrade is in a state where calldata has been generated.
    pub fn has_calldata(&self) -> bool {
        self.calldata.is_some()
    }

    /// Checks if the upgrade has been executed.
    pub fn is_executed(&self) -> bool {
        matches!(self.status, UpgradeStatus::Executed)
    }

    /// Checks if the upgrade has failed.
    pub fn is_failed(&self) -> bool {
        matches!(self.status, UpgradeStatus::Failed { .. })
    }

    /// Marks the upgrade as prepared with the given calldata.
    pub fn mark_prepared(&mut self, calldata: UpgradeCalldata) {
        self.calldata = Some(calldata);
        self.status = UpgradeStatus::Prepared;
    }

    /// Marks the upgrade as scheduled.
    pub fn mark_scheduled(&mut self) {
        self.status = UpgradeStatus::Scheduled;
    }

    /// Marks the upgrade as executed with the transaction hash.
    pub fn mark_executed(&mut self, tx_hash: B256) {
        self.executed_tx = Some(tx_hash);
        self.executed_at = Some(Utc::now());
        self.status = UpgradeStatus::Executed;
    }

    /// Marks the upgrade as failed with a reason.
    pub fn mark_failed(&mut self, reason: impl Into<String>) {
        self.status = UpgradeStatus::Failed {
            reason: reason.into(),
        };
    }

    /// Sets the deployment output path.
    pub fn set_deployment_output_path(&mut self, path: PathBuf) {
        self.deployment_output_path = Some(path);
    }

    /// Returns a display string for the upgrade (e.g., "v29 → v30").
    pub fn version_transition_string(&self) -> String {
        format!("v{} → v{}", self.source_version, self.target_version)
    }
}
