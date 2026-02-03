//! Wallet types for ecosystem and chain operations.

use alloy_primitives::Address;
use secrecy::{ExposeSecret, SecretString};
use serde::{de::Deserializer, ser::SerializeStruct, Deserialize, Serialize, Serializer};

/// A wallet with address and private key.
///
/// Private keys are wrapped in `SecretString` for security:
/// - Automatic zeroization on drop
/// - Excluded from Debug output
///
/// # Example YAML
/// ```yaml
/// deployer:
///   address: "0xe1d0e06d6e911d72e1f69ed9af358e2a67d766d9"
///   private_key: "0x40424fcdd7f5189ae1a5929e9f156f76deb7f05ac5a36abbdc6f77716dc54b10"
/// ```
#[derive(Clone)]
pub struct Wallet {
    /// Wallet address.
    pub address: Address,

    /// Private key (secret).
    pub private_key: SecretString,
}

impl Wallet {
    /// Creates a new wallet with the given address and private key.
    pub fn new(address: Address, private_key: SecretString) -> Self {
        Self {
            address,
            private_key,
        }
    }

    /// Returns a reference to the private key (for controlled exposure).
    pub fn expose_private_key(&self) -> &str {
        self.private_key.expose_secret()
    }
}

impl std::fmt::Debug for Wallet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Wallet")
            .field("address", &self.address)
            .field("private_key", &"[REDACTED]")
            .finish()
    }
}

impl Serialize for Wallet {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("Wallet", 2)?;
        state.serialize_field("address", &self.address)?;
        state.serialize_field("private_key", self.private_key.expose_secret())?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for Wallet {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct WalletHelper {
            address: Address,
            private_key: String,
        }

        let helper = WalletHelper::deserialize(deserializer)?;
        Ok(Wallet {
            address: helper.address,
            private_key: SecretString::from(helper.private_key),
        })
    }
}

/// Collection of wallets for ecosystem or chain operations.
///
/// Contains the 8 standard wallet roles used in ZkSync ecosystems:
/// - `deployer` - Deploys contracts
/// - `operator` - Operates the chain
/// - `blob_operator` - Handles blob operations
/// - `prove_operator` - Submits proofs
/// - `execute_operator` - Executes transactions
/// - `fee_account` - Receives fees
/// - `governor` - Governance operations
/// - `token_multiplier_setter` - Sets token multipliers
///
/// All fields are optional to support partial configurations.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Wallets {
    /// Deployer wallet - deploys contracts.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deployer: Option<Wallet>,

    /// Operator wallet - operates the chain.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operator: Option<Wallet>,

    /// Blob operator wallet - handles blob operations.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blob_operator: Option<Wallet>,

    /// Prove operator wallet - submits proofs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prove_operator: Option<Wallet>,

    /// Execute operator wallet - executes transactions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execute_operator: Option<Wallet>,

    /// Fee account wallet - receives fees.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fee_account: Option<Wallet>,

    /// Governor wallet - governance operations.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub governor: Option<Wallet>,

    /// Token multiplier setter wallet.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_multiplier_setter: Option<Wallet>,
}

impl Wallets {
    /// Creates an empty wallets collection.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the deployer wallet.
    pub fn with_deployer(mut self, wallet: Wallet) -> Self {
        self.deployer = Some(wallet);
        self
    }

    /// Sets the operator wallet.
    pub fn with_operator(mut self, wallet: Wallet) -> Self {
        self.operator = Some(wallet);
        self
    }

    /// Sets the governor wallet.
    pub fn with_governor(mut self, wallet: Wallet) -> Self {
        self.governor = Some(wallet);
        self
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_wallet_debug_redacts_key() {
        let wallet = Wallet {
            address: "0xe1d0e06d6e911d72e1f69ed9af358e2a67d766d9"
                .parse()
                .unwrap(),
            private_key: SecretString::from(
                "0x40424fcdd7f5189ae1a5929e9f156f76deb7f05ac5a36abbdc6f77716dc54b10".to_string(),
            ),
        };

        let debug_str = format!("{:?}", wallet);
        assert!(debug_str.contains("[REDACTED]"));
        assert!(!debug_str.contains("40424fcdd7f"));
    }

    #[test]
    fn test_wallets_deserialize() {
        let yaml = r#"
deployer:
  address: "0xe1d0e06d6e911d72e1f69ed9af358e2a67d766d9"
  private_key: "0x40424fcdd7f5189ae1a5929e9f156f76deb7f05ac5a36abbdc6f77716dc54b10"
governor:
  address: "0x1234567890123456789012345678901234567890"
  private_key: "0x1234567890123456789012345678901234567890123456789012345678901234"
"#;
        let wallets: Wallets = serde_yaml::from_str(yaml).unwrap();
        assert!(wallets.deployer.is_some());
        assert!(wallets.governor.is_some());
        assert!(wallets.operator.is_none());
    }

    #[test]
    fn test_wallet_expose_private_key() {
        let key = "0x40424fcdd7f5189ae1a5929e9f156f76deb7f05ac5a36abbdc6f77716dc54b10";
        let wallet = Wallet {
            address: "0xe1d0e06d6e911d72e1f69ed9af358e2a67d766d9"
                .parse()
                .unwrap(),
            private_key: SecretString::from(key.to_string()),
        };

        assert_eq!(wallet.expose_private_key(), key);
    }
}
