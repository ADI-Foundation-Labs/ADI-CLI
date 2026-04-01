//! Shared signing helpers for upgrade operations.

use alloy_network::EthereumWallet;
use alloy_provider::{Provider, ProviderBuilder};
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

/// Build a signing provider by wrapping an existing provider with a wallet
/// derived from the given secret key.
///
/// # Errors
///
/// Returns [`UpgradeError::GovernanceFailed`] if the key is invalid.
pub(crate) fn build_signing_provider<P: Provider + Clone>(
    provider: &P,
    key: &secrecy::SecretString,
) -> Result<impl Provider + Clone> {
    let signer = signer_from_secret(key)?;
    let wallet = EthereumWallet::from(signer);
    Ok(ProviderBuilder::new()
        .wallet(wallet)
        .connect_provider(provider.clone()))
}
