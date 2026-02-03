//! Signer creation from SecretString.

use crate::error::{FundingError, Result};
use alloy_primitives::Address;
use alloy_signer_local::PrivateKeySigner;
use secrecy::{ExposeSecret, SecretString};

/// Create a signer from a SecretString private key.
///
/// # Arguments
///
/// * `private_key` - Private key as SecretString (with or without 0x prefix).
///
/// # Returns
///
/// A local wallet signer.
///
/// # Errors
///
/// Returns error if the private key is invalid.
pub fn create_signer(private_key: &SecretString) -> Result<PrivateKeySigner> {
    let key_str = private_key.expose_secret();

    // Remove 0x prefix if present
    let key_hex = key_str.strip_prefix("0x").unwrap_or(key_str);

    // Parse hex to bytes
    let key_bytes =
        hex::decode(key_hex).map_err(|e| FundingError::InvalidPrivateKey(format!("Invalid hex: {e}")))?;

    // Create signer
    PrivateKeySigner::from_slice(&key_bytes).map_err(|e| FundingError::InvalidPrivateKey(e.to_string()))
}

/// Get the address for a signer.
pub fn signer_address(signer: &PrivateKeySigner) -> Address {
    signer.address()
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_create_signer_with_0x_prefix() {
        let key = SecretString::from(
            "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80".to_string(),
        );
        let signer = create_signer(&key).unwrap();
        assert_eq!(
            signer.address().to_string().to_lowercase(),
            "0xf39fd6e51aad88f6f4ce6ab8827279cfffb92266"
        );
    }

    #[test]
    fn test_create_signer_without_prefix() {
        let key = SecretString::from(
            "ac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80".to_string(),
        );
        let signer = create_signer(&key).unwrap();
        assert_eq!(
            signer.address().to_string().to_lowercase(),
            "0xf39fd6e51aad88f6f4ce6ab8827279cfffb92266"
        );
    }

    #[test]
    fn test_create_signer_invalid_hex() {
        let key = SecretString::from("not-valid-hex".to_string());
        let result = create_signer(&key);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Invalid hex"));
    }

    #[test]
    fn test_create_signer_invalid_length() {
        let key = SecretString::from("0x1234".to_string());
        let result = create_signer(&key);
        assert!(result.is_err());
    }

    #[test]
    fn test_signer_address() {
        let key = SecretString::from(
            "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80".to_string(),
        );
        let signer = create_signer(&key).unwrap();
        let address = signer_address(&signer);
        assert_eq!(
            address.to_string().to_lowercase(),
            "0xf39fd6e51aad88f6f4ce6ab8827279cfffb92266"
        );
    }
}
