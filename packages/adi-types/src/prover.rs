//! Prover and wallet creation mode types.

use clap::ValueEnum;
use serde::{de::Deserializer, Deserialize, Serialize, Serializer};
use strum::{Display, EnumString};

/// Prover mode for the ecosystem.
///
/// Maps to the `prover_version` field in ZkStack.yaml files.
///
/// # Serde Behavior
/// - Serializes to kebab-case: `"no-proofs"`, `"gpu"`
/// - Deserializes case-insensitively: accepts `"NoProofs"`, `"no-proofs"`, `"noproofs"`
#[derive(Clone, Debug, Default, Display, EnumString, PartialEq, Eq, ValueEnum)]
#[strum(serialize_all = "kebab-case")]
pub enum ProverMode {
    /// No proofs (development mode).
    #[default]
    NoProofs,
    /// GPU prover.
    Gpu,
}

impl Serialize for ProverMode {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(match self {
            Self::NoProofs => "no-proofs",
            Self::Gpu => "gpu",
        })
    }
}

impl<'de> Deserialize<'de> for ProverMode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        // Normalize: remove hyphens/underscores and lowercase
        let normalized = s.to_lowercase().replace(['-', '_'], "");
        match normalized.as_str() {
            "noproofs" => Ok(Self::NoProofs),
            "gpu" => Ok(Self::Gpu),
            _ => Err(serde::de::Error::unknown_variant(
                &s,
                &["no-proofs", "NoProofs", "gpu", "Gpu"],
            )),
        }
    }
}

/// Wallet creation mode.
///
/// Maps to the `wallet_creation` field in ZkStack.yaml files.
#[derive(Clone, Debug, Default, Display, EnumString, PartialEq, Eq, ValueEnum)]
pub enum WalletCreation {
    /// Random wallet generation.
    #[default]
    Random,
    /// Use empty wallets.
    Empty,
    /// Use in-file wallets.
    InFile,
    /// Use localhost wallets.
    Localhost,
}

impl Serialize for WalletCreation {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(match self {
            Self::Random => "Random",
            Self::Empty => "Empty",
            Self::InFile => "InFile",
            Self::Localhost => "Localhost",
        })
    }
}

impl<'de> Deserialize<'de> for WalletCreation {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.to_lowercase().as_str() {
            "random" => Ok(Self::Random),
            "empty" => Ok(Self::Empty),
            "infile" | "in_file" | "in-file" => Ok(Self::InFile),
            "localhost" => Ok(Self::Localhost),
            _ => Err(serde::de::Error::unknown_variant(
                &s,
                &["Random", "Empty", "InFile", "Localhost"],
            )),
        }
    }
}

/// L1 batch commit data generator mode.
///
/// Maps to the `l1_batch_commit_data_generator_mode` field in chain ZkStack.yaml.
#[derive(Clone, Debug, Default, Display, EnumString, PartialEq, Eq, ValueEnum)]
pub enum BatchCommitDataMode {
    /// Rollup mode.
    #[default]
    Rollup,
    /// Validium mode.
    Validium,
}

impl Serialize for BatchCommitDataMode {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(match self {
            Self::Rollup => "Rollup",
            Self::Validium => "Validium",
        })
    }
}

impl<'de> Deserialize<'de> for BatchCommitDataMode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.to_lowercase().as_str() {
            "rollup" => Ok(Self::Rollup),
            "validium" => Ok(Self::Validium),
            _ => Err(serde::de::Error::unknown_variant(
                &s,
                &["Rollup", "Validium"],
            )),
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_prover_mode_display() {
        assert_eq!(ProverMode::NoProofs.to_string(), "no-proofs");
        assert_eq!(ProverMode::Gpu.to_string(), "gpu");
    }

    #[test]
    fn test_prover_mode_parse() {
        assert_eq!(
            "no-proofs".parse::<ProverMode>().unwrap(),
            ProverMode::NoProofs
        );
        assert_eq!("gpu".parse::<ProverMode>().unwrap(), ProverMode::Gpu);
    }

    #[test]
    fn test_prover_mode_deserialize_kebab() {
        let mode: ProverMode = serde_yaml::from_str("no-proofs").unwrap();
        assert_eq!(mode, ProverMode::NoProofs);
    }

    #[test]
    fn test_prover_mode_deserialize_pascal() {
        // zkstack YAML uses PascalCase
        let mode: ProverMode = serde_yaml::from_str("NoProofs").unwrap();
        assert_eq!(mode, ProverMode::NoProofs);
    }

    #[test]
    fn test_wallet_creation_deserialize() {
        let mode: WalletCreation = serde_yaml::from_str("Random").unwrap();
        assert_eq!(mode, WalletCreation::Random);

        let mode: WalletCreation = serde_yaml::from_str("random").unwrap();
        assert_eq!(mode, WalletCreation::Random);
    }

    #[test]
    fn test_batch_commit_data_mode_deserialize() {
        let mode: BatchCommitDataMode = serde_yaml::from_str("Rollup").unwrap();
        assert_eq!(mode, BatchCommitDataMode::Rollup);

        let mode: BatchCommitDataMode = serde_yaml::from_str("rollup").unwrap();
        assert_eq!(mode, BatchCommitDataMode::Rollup);
    }
}
