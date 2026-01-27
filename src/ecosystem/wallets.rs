//! Wallet types for ecosystem operations.
//!
//! This module provides wallet structures for managing keypairs used in
//! ecosystem deployment and governance operations.

use alloy_primitives::Address;
use secrecy::SecretString;
use serde::{Deserialize, Serialize};

/// A wallet with an address and optional private key.
///
/// The private key is wrapped in [`SecretString`] for secure handling:
/// - Zeroized on drop
/// - Excluded from Debug/Display
/// - Never serialized (security)
///
/// # Example
///
/// ```rust
/// use alloy_primitives::Address;
/// use secrecy::SecretString;
///
/// let wallet = Wallet {
///     address: Address::ZERO,
///     private_key: Some(SecretString::new("0x...".to_string())),
/// };
/// ```
// Note: Currently unused as commands are implemented in later phases (US1-US6)
#[allow(dead_code)]
#[derive(Clone, Serialize, Deserialize)]
pub struct Wallet {
    /// The wallet's Ethereum address.
    pub address: Address,

    /// The wallet's private key (optional, secret).
    ///
    /// This field is never serialized for security reasons.
    /// Private keys should be stored separately with restricted permissions.
    #[serde(skip)]
    pub private_key: Option<SecretString>,
}

#[allow(dead_code)]
impl Wallet {
    /// Creates a new wallet with just an address (no private key).
    ///
    /// # Arguments
    ///
    /// * `address` - The wallet's Ethereum address
    ///
    /// # Example
    ///
    /// ```rust
    /// use alloy_primitives::Address;
    ///
    /// let wallet = Wallet::from_address(Address::ZERO);
    /// assert!(wallet.private_key.is_none());
    /// ```
    pub fn from_address(address: Address) -> Self {
        Self {
            address,
            private_key: None,
        }
    }

    /// Creates a new wallet with both address and private key.
    ///
    /// # Arguments
    ///
    /// * `address` - The wallet's Ethereum address
    /// * `private_key` - The wallet's private key as a secret string
    ///
    /// # Example
    ///
    /// ```rust
    /// use alloy_primitives::Address;
    /// use secrecy::SecretString;
    ///
    /// let wallet = Wallet::new(
    ///     Address::ZERO,
    ///     SecretString::new("0x...".to_string()),
    /// );
    /// assert!(wallet.private_key.is_some());
    /// ```
    pub fn new(address: Address, private_key: SecretString) -> Self {
        Self {
            address,
            private_key: Some(private_key),
        }
    }

    /// Checks if this wallet has a private key.
    pub fn has_private_key(&self) -> bool {
        self.private_key.is_some()
    }
}

impl std::fmt::Debug for Wallet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Wallet")
            .field("address", &self.address)
            .field(
                "private_key",
                &self.private_key.as_ref().map(|_| "[REDACTED]"),
            )
            .finish()
    }
}

/// Wallets used for ecosystem-level operations.
///
/// These wallets are used during ecosystem initialization and deployment:
/// - `deployer`: Deploys ecosystem contracts to the settlement layer
/// - `governor`: Manages governance operations and ownership
///
/// # Funding Requirements
///
/// | Role     | ETH Required |
/// |----------|--------------|
/// | deployer | 1 ETH        |
/// | governor | 1 ETH        |
// Note: Currently unused as commands are implemented in later phases (US1-US6)
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EcosystemWallets {
    /// The deployer wallet for contract deployment.
    pub deployer: Wallet,

    /// The governor wallet for governance operations.
    pub governor: Wallet,
    // TODO: Include additional wallets for database storage (not currently used by CLI):
    // - operator: Wallet              // Operator wallet
    // - blob_operator: Wallet         // Blob operator wallet
    // - prove_operator: Wallet        // Prove operator wallet
    // - execute_operator: Wallet      // Execute operator wallet
    // - fee_account: Wallet           // Fee account wallet
    // - token_multiplier_setter: Wallet // Token multiplier setter
    // - security_council: Option<Wallet> // Security council wallet
}

#[allow(dead_code)]
impl EcosystemWallets {
    /// Creates new ecosystem wallets from deployer and governor wallets.
    pub fn new(deployer: Wallet, governor: Wallet) -> Self {
        Self { deployer, governor }
    }
}
