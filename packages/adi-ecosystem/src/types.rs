//! Domain types for ecosystem configuration.

use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};

/// L1 network for ecosystem deployment.
#[derive(
    Clone, Debug, Default, Serialize, Deserialize, Display, EnumString, PartialEq, Eq, ValueEnum,
)]
#[strum(serialize_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum L1Network {
    /// Local development network.
    #[default]
    Localhost,
    /// Sepolia testnet.
    Sepolia,
    /// Ethereum mainnet.
    Mainnet,
}

/// Prover mode for the ecosystem.
#[derive(
    Clone, Debug, Default, Serialize, Deserialize, Display, EnumString, PartialEq, Eq, ValueEnum,
)]
#[strum(serialize_all = "kebab-case")]
#[serde(rename_all = "kebab-case")]
pub enum ProverMode {
    /// No proofs (development mode).
    #[default]
    NoProofs,
    /// GPU prover.
    Gpu,
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_l1_network_display() {
        assert_eq!(L1Network::Localhost.to_string(), "localhost");
        assert_eq!(L1Network::Sepolia.to_string(), "sepolia");
        assert_eq!(L1Network::Mainnet.to_string(), "mainnet");
    }

    #[test]
    fn test_prover_mode_display() {
        assert_eq!(ProverMode::NoProofs.to_string(), "no-proofs");
        assert_eq!(ProverMode::Gpu.to_string(), "gpu");
    }

    #[test]
    fn test_l1_network_parse() {
        assert_eq!(
            "localhost".parse::<L1Network>().unwrap(),
            L1Network::Localhost
        );
        assert_eq!("sepolia".parse::<L1Network>().unwrap(), L1Network::Sepolia);
    }

    #[test]
    fn test_prover_mode_parse() {
        assert_eq!(
            "no-proofs".parse::<ProverMode>().unwrap(),
            ProverMode::NoProofs
        );
        assert_eq!("gpu".parse::<ProverMode>().unwrap(), ProverMode::Gpu);
    }
}
