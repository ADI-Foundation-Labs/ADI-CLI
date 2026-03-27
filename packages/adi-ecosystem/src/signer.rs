//! Signer creation from private key.

use crate::error::{EcosystemError, Result};
use alloy_signer_local::PrivateKeySigner;
use secrecy::{ExposeSecret, SecretString};

/// Create a signer from a private key.
///
/// Accepts hex-encoded private keys with or without `0x` prefix.
///
/// # Errors
///
/// Returns error if the key is not valid hex or not 32 bytes.
pub(crate) fn create_signer(key: &SecretString) -> Result<PrivateKeySigner> {
    let key_str = key.expose_secret();

    // Strip 0x prefix if present
    let key_hex = key_str.strip_prefix("0x").unwrap_or(key_str);

    let key_bytes: [u8; 32] = hex::decode(key_hex)
        .map_err(|e| EcosystemError::InvalidConfig(format!("Invalid private key hex: {}", e)))?
        .try_into()
        .map_err(|_| EcosystemError::InvalidConfig("Private key must be 32 bytes".to_string()))?;

    PrivateKeySigner::from_bytes(&key_bytes.into())
        .map_err(|e| EcosystemError::InvalidConfig(format!("Invalid private key: {}", e)))
}
