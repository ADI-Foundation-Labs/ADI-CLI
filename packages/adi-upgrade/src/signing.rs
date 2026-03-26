//! Shared signing helpers for upgrade operations.

use alloy_signer_local::PrivateKeySigner;
use secrecy::ExposeSecret;

use crate::error::{Result, UpgradeError};

/// Create a [`PrivateKeySigner`] from a hex-encoded secret key.
///
/// Handles optional `0x` prefix and validates key length.
///
/// # Errors
///
/// Returns [`UpgradeError::GovernanceFailed`] if the hex is invalid,
/// the key is not 32 bytes, or the key is otherwise invalid.
pub(crate) fn signer_from_secret(key: &secrecy::SecretString) -> Result<PrivateKeySigner> {
    let key_str = key.expose_secret();
    let key_hex = key_str.strip_prefix("0x").unwrap_or(key_str);
    let key_bytes: [u8; 32] = hex::decode(key_hex)
        .map_err(|e| UpgradeError::GovernanceFailed(format!("Invalid key hex: {e}")))?
        .try_into()
        .map_err(|_| UpgradeError::GovernanceFailed("Key must be 32 bytes".into()))?;

    PrivateKeySigner::from_bytes(&key_bytes.into())
        .map_err(|e| UpgradeError::GovernanceFailed(format!("Invalid key: {e}")))
}
