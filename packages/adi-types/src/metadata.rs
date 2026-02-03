//! Ecosystem and chain metadata types.

use crate::{BaseToken, BatchCommitDataMode, L1Network, ProverMode, WalletCreation};
use serde::{de::Deserializer, Deserialize, Serialize, Serializer};

/// VM option for chain execution.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum VmOption {
    /// ZkSync OS VM.
    #[default]
    ZKSyncOsVM,
    /// EVM emulator.
    Evm,
}

impl Serialize for VmOption {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(match self {
            Self::ZKSyncOsVM => "ZKSyncOsVM",
            Self::Evm => "EVM",
        })
    }
}

impl<'de> Deserialize<'de> for VmOption {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.to_lowercase().as_str() {
            "zksyncosvm" | "zksyncos" | "zksync" => Ok(Self::ZKSyncOsVM),
            "evm" => Ok(Self::Evm),
            _ => Err(serde::de::Error::unknown_variant(
                &s,
                &["ZKSyncOsVM", "EVM"],
            )),
        }
    }
}

/// Ecosystem metadata from ZkStack.yaml.
///
/// Top-level configuration for a ZkSync ecosystem.
///
/// # Example YAML
/// ```yaml
/// name: "adi_ecosystem"
/// l1_network: "Sepolia"
/// link_to_code: "/deps/zksync-era"
/// chains: "/workspace/adi_ecosystem/chains"
/// config: "/workspace/adi_ecosystem/./configs/"
/// default_chain: "adi"
/// era_chain_id: 270
/// prover_version: "NoProofs"
/// wallet_creation: "Random"
/// ```
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EcosystemMetadata {
    /// Ecosystem name.
    pub name: String,

    /// L1 network.
    pub l1_network: L1Network,

    /// Path to era-contracts code.
    pub link_to_code: String,

    /// Path to chains directory.
    pub chains: String,

    /// Path to configs directory.
    pub config: String,

    /// Default chain name.
    pub default_chain: String,

    /// ERA chain ID.
    pub era_chain_id: u64,

    /// Prover version/mode.
    pub prover_version: ProverMode,

    /// Wallet creation mode.
    pub wallet_creation: WalletCreation,
}

/// Chain metadata from chain ZkStack.yaml.
///
/// Configuration for a specific chain within an ecosystem.
///
/// # Example YAML
/// ```yaml
/// id: 1
/// name: "adi"
/// chain_id: 222
/// prover_version: "NoProofs"
/// l1_network: "Sepolia"
/// ```
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChainMetadata {
    /// Chain internal ID.
    pub id: u64,

    /// Chain name.
    pub name: String,

    /// Chain ID (for EVM).
    pub chain_id: u64,

    /// Prover version/mode.
    pub prover_version: ProverMode,

    /// L1 network.
    pub l1_network: L1Network,

    /// Path to era-contracts code.
    pub link_to_code: String,

    /// Path to chain configs.
    pub configs: String,

    /// Path to RocksDB data.
    pub rocks_db_path: String,

    /// External node config path (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_node_config_path: Option<String>,

    /// Path to artifacts.
    pub artifacts_path: String,

    /// L1 batch commit data generator mode.
    pub l1_batch_commit_data_generator_mode: BatchCommitDataMode,

    /// Base token configuration.
    pub base_token: BaseToken,

    /// Wallet creation mode.
    pub wallet_creation: WalletCreation,

    /// Enable EVM emulator.
    #[serde(default)]
    pub evm_emulator: bool,

    /// Use tight ports.
    #[serde(default)]
    pub tight_ports: bool,

    /// VM option.
    pub vm_option: VmOption,

    /// Path to contracts.
    pub contracts_path: String,

    /// Path to default configs.
    pub default_configs_path: String,
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_ecosystem_metadata_deserialize() {
        let yaml = r#"
name: "adi_ecosystem"
l1_network: "Sepolia"
link_to_code: "/deps/zksync-era"
chains: "/workspace/adi_ecosystem/chains"
config: "/workspace/adi_ecosystem/./configs/"
default_chain: "adi"
era_chain_id: 270
prover_version: "NoProofs"
wallet_creation: "Random"
"#;
        let meta: EcosystemMetadata = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(meta.name, "adi_ecosystem");
        assert_eq!(meta.l1_network, L1Network::Sepolia);
        assert_eq!(meta.era_chain_id, 270);
        assert_eq!(meta.prover_version, ProverMode::NoProofs);
    }

    #[test]
    fn test_chain_metadata_deserialize() {
        let yaml = r#"
id: 1
name: "adi"
chain_id: 222
prover_version: "NoProofs"
l1_network: "Sepolia"
link_to_code: "/deps/zksync-era"
configs: "/workspace/adi_ecosystem/chains/adi/configs/"
rocks_db_path: "/workspace/adi_ecosystem/chains/adi/db/"
artifacts_path: "/workspace/adi_ecosystem/chains/adi/artifacts/"
l1_batch_commit_data_generator_mode: "Rollup"
base_token:
  address: "0x0000000000000000000000000000000000000001"
  nominator: 1
  denominator: 1
wallet_creation: "Random"
evm_emulator: false
tight_ports: false
vm_option: "ZKSyncOsVM"
contracts_path: "/deps/zksync-era/contracts/"
default_configs_path: "/deps/zksync-era/etc/env/file_based"
"#;
        let meta: ChainMetadata = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(meta.name, "adi");
        assert_eq!(meta.chain_id, 222);
        assert!(meta.base_token.is_eth());
    }

    #[test]
    fn test_vm_option_deserialize() {
        let opt: VmOption = serde_yaml::from_str("ZKSyncOsVM").unwrap();
        assert_eq!(opt, VmOption::ZKSyncOsVM);

        let opt: VmOption = serde_yaml::from_str("EVM").unwrap();
        assert_eq!(opt, VmOption::Evm);
    }
}
