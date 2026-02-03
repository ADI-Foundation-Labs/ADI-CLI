//! Partial types for merge operations.
//!
//! Each `Partial*` type has all fields as `Option<T>` to support
//! incremental updates and merging from multiple sources.

use crate::{BaseToken, BatchCommitDataMode, L1Network, ProverMode, VmOption, WalletCreation};
use serde::{Deserialize, Serialize};

/// Partial ecosystem metadata for merge operations.
///
/// All fields are optional to allow partial updates.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PartialEcosystemMetadata {
    /// Ecosystem name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// L1 network.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub l1_network: Option<L1Network>,

    /// Path to era-contracts code.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub link_to_code: Option<String>,

    /// Path to chains directory.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chains: Option<String>,

    /// Path to configs directory.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<String>,

    /// Default chain name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_chain: Option<String>,

    /// ERA chain ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub era_chain_id: Option<u64>,

    /// Prover version/mode.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prover_version: Option<ProverMode>,

    /// Wallet creation mode.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wallet_creation: Option<WalletCreation>,
}

/// Partial chain metadata for merge operations.
///
/// All fields are optional to allow partial updates.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PartialChainMetadata {
    /// Chain internal ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<u64>,

    /// Chain name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// Chain ID (for EVM).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chain_id: Option<u64>,

    /// Prover version/mode.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prover_version: Option<ProverMode>,

    /// L1 network.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub l1_network: Option<L1Network>,

    /// Path to era-contracts code.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub link_to_code: Option<String>,

    /// Path to chain configs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub configs: Option<String>,

    /// Path to RocksDB data.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rocks_db_path: Option<String>,

    /// External node config path.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_node_config_path: Option<String>,

    /// Path to artifacts.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artifacts_path: Option<String>,

    /// L1 batch commit data generator mode.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub l1_batch_commit_data_generator_mode: Option<BatchCommitDataMode>,

    /// Base token configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_token: Option<BaseToken>,

    /// Wallet creation mode.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wallet_creation: Option<WalletCreation>,

    /// Enable EVM emulator.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evm_emulator: Option<bool>,

    /// Use tight ports.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tight_ports: Option<bool>,

    /// VM option.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vm_option: Option<VmOption>,

    /// Path to contracts.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contracts_path: Option<String>,

    /// Path to default configs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_configs_path: Option<String>,
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_partial_ecosystem_metadata_default() {
        let partial = PartialEcosystemMetadata::default();
        assert!(partial.name.is_none());
        assert!(partial.l1_network.is_none());
    }

    #[test]
    fn test_partial_ecosystem_metadata_deserialize() {
        let yaml = r#"
name: "new_name"
default_chain: "new_chain"
"#;
        let partial: PartialEcosystemMetadata = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(partial.name, Some("new_name".to_string()));
        assert_eq!(partial.default_chain, Some("new_chain".to_string()));
        assert!(partial.l1_network.is_none());
    }

    #[test]
    fn test_partial_chain_metadata_deserialize() {
        let yaml = r#"
chain_id: 333
evm_emulator: true
"#;
        let partial: PartialChainMetadata = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(partial.chain_id, Some(333));
        assert_eq!(partial.evm_emulator, Some(true));
        assert!(partial.name.is_none());
    }
}
