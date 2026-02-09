//! Network configuration types.

use clap::ValueEnum;
use serde::{de::Deserializer, Deserialize, Serialize, Serializer};
use strum::{Display, EnumString};

/// L1 network for ecosystem deployment.
///
/// Maps to the `l1_network` field in ZkStack.yaml files.
///
/// # Serde Behavior
/// - Serializes to PascalCase: `"Localhost"`, `"Sepolia"`, `"Mainnet"` (zkstack format)
/// - Deserializes case-insensitively: accepts both `"Sepolia"` and `"sepolia"`
#[derive(Clone, Debug, Default, Display, EnumString, PartialEq, Eq, ValueEnum)]
#[strum(serialize_all = "lowercase")]
pub enum L1Network {
    /// Local development network.
    #[default]
    Localhost,
    /// Sepolia testnet.
    Sepolia,
    /// Ethereum mainnet.
    Mainnet,
}

impl Serialize for L1Network {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(match self {
            Self::Localhost => "Localhost",
            Self::Sepolia => "Sepolia",
            Self::Mainnet => "Mainnet",
        })
    }
}

impl<'de> Deserialize<'de> for L1Network {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.to_lowercase().as_str() {
            "localhost" => Ok(Self::Localhost),
            "sepolia" => Ok(Self::Sepolia),
            "mainnet" => Ok(Self::Mainnet),
            _ => Err(serde::de::Error::unknown_variant(
                &s,
                &["localhost", "sepolia", "mainnet"],
            )),
        }
    }
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
    fn test_l1_network_parse() {
        assert_eq!(
            "localhost".parse::<L1Network>().unwrap(),
            L1Network::Localhost
        );
        assert_eq!("sepolia".parse::<L1Network>().unwrap(), L1Network::Sepolia);
    }

    #[test]
    fn test_l1_network_serialize() {
        let yaml = serde_yaml::to_string(&L1Network::Sepolia).unwrap();
        assert!(yaml.contains("Sepolia"));
    }

    #[test]
    fn test_l1_network_deserialize_lowercase() {
        let network: L1Network = serde_yaml::from_str("sepolia").unwrap();
        assert_eq!(network, L1Network::Sepolia);
    }

    #[test]
    fn test_l1_network_deserialize_pascalcase() {
        // zkstack YAML uses PascalCase
        let network: L1Network = serde_yaml::from_str("Sepolia").unwrap();
        assert_eq!(network, L1Network::Sepolia);
    }

    #[test]
    fn test_l1_network_deserialize_uppercase() {
        let network: L1Network = serde_yaml::from_str("MAINNET").unwrap();
        assert_eq!(network, L1Network::Mainnet);
    }
}
