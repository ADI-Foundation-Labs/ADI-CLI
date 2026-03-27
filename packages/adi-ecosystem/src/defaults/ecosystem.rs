use secrecy::SecretString;
use serde::{Deserialize, Serialize};
use url::Url;

use alloy_primitives::Address;

use crate::types::L1Network;

use super::chain::ChainDefaults;

/// Ecosystem-level ownership configuration.
///
/// Used for transferring/accepting ecosystem ownership (separate from chain ownership).
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct EcosystemOwnershipDefaults {
    /// Address to transfer ecosystem ownership to (newEcosystemOwner).
    #[serde(default)]
    pub new_owner: Option<Address>,

    /// Private key for accepting ecosystem ownership.
    /// Note: This field is never serialized (skipped) for security.
    #[serde(default, skip_serializing)]
    pub private_key: Option<SecretString>,
}

/// Ecosystem-level configuration defaults.
///
/// Contains ecosystem-wide settings and nested chain configurations.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EcosystemDefaults {
    /// Ecosystem name.
    #[serde(default = "default_ecosystem_name")]
    pub name: String,

    /// L1 network (settlement layer).
    #[serde(default)]
    pub l1_network: L1Network,

    /// Settlement layer RPC URL.
    #[serde(default)]
    pub rpc_url: Option<Url>,

    /// Ecosystem-level ownership configuration.
    #[serde(default)]
    pub ownership: EcosystemOwnershipDefaults,

    /// Chain configurations.
    #[serde(default)]
    pub chains: Vec<ChainDefaults>,
}

fn default_ecosystem_name() -> String {
    "adi_ecosystem".to_string()
}

impl Default for EcosystemDefaults {
    fn default() -> Self {
        Self {
            name: default_ecosystem_name(),
            l1_network: L1Network::Sepolia,
            rpc_url: None,
            ownership: EcosystemOwnershipDefaults::default(),
            chains: Vec::new(),
        }
    }
}

impl EcosystemDefaults {
    /// Get a chain by name.
    #[must_use]
    pub fn get_chain(&self, name: &str) -> Option<&ChainDefaults> {
        self.chains.iter().find(|c| c.name == name)
    }

    /// Get a mutable reference to a chain by name.
    pub fn get_chain_mut(&mut self, name: &str) -> Option<&mut ChainDefaults> {
        self.chains.iter_mut().find(|c| c.name == name)
    }

    /// Get all chain names.
    #[must_use]
    pub fn chain_names(&self) -> Vec<&str> {
        self.chains.iter().map(|c| c.name.as_str()).collect()
    }

    /// Get the first/default chain if available.
    #[must_use]
    pub fn default_chain(&self) -> Option<&ChainDefaults> {
        self.chains.first()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    use crate::types::L1Network;

    #[test]
    fn test_ecosystem_defaults() {
        let ecosystem = EcosystemDefaults::default();
        assert_eq!(ecosystem.name, "adi_ecosystem");
        assert_eq!(ecosystem.l1_network, L1Network::Sepolia);
        assert!(ecosystem.chains.is_empty());
    }

    #[test]
    fn test_get_chain() {
        let mut ecosystem = EcosystemDefaults::default();
        ecosystem.chains.push(ChainDefaults {
            name: "chain_a".to_string(),
            chain_id: 100,
            ..Default::default()
        });
        ecosystem.chains.push(ChainDefaults {
            name: "chain_b".to_string(),
            chain_id: 200,
            ..Default::default()
        });

        assert!(ecosystem.get_chain("chain_a").is_some());
        assert_eq!(ecosystem.get_chain("chain_a").unwrap().chain_id, 100);
        assert!(ecosystem.get_chain("chain_b").is_some());
        assert!(ecosystem.get_chain("nonexistent").is_none());

        let names = ecosystem.chain_names();
        assert_eq!(names, vec!["chain_a", "chain_b"]);
    }

    #[test]
    fn test_deserialize_ecosystem_defaults() {
        let yaml = r#"
name: my_ecosystem
l1_network: Sepolia
rpc_url: "https://sepolia.example.com"
ownership:
  new_owner: "0x1111111111111111111111111111111111111111"
chains:
  - name: chain_a
    chain_id: 222
    blobs: true
  - name: chain_b
    chain_id: 333
"#;
        let ecosystem: EcosystemDefaults = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(ecosystem.name, "my_ecosystem");
        assert_eq!(ecosystem.l1_network, L1Network::Sepolia);
        assert!(ecosystem.ownership.new_owner.is_some());
        assert_eq!(ecosystem.chains.len(), 2);
        assert_eq!(ecosystem.chains[0].name, "chain_a");
        assert!(ecosystem.chains[0].blobs);
        assert_eq!(ecosystem.chains[1].name, "chain_b");
        assert!(!ecosystem.chains[1].blobs);
    }
}
