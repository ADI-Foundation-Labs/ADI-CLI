//! Constructor argument encoding for proxy verification.

use alloy_primitives::{Address, Bytes};
use alloy_sol_types::SolValue;

/// Encode TransparentUpgradeableProxy constructor arguments as hex string.
///
/// Constructor signature: `constructor(address _logic, address initialOwner, bytes memory _data)`
///
/// # Arguments
/// * `impl_addr` - Implementation contract address (_logic)
/// * `proxy_admin` - Proxy admin address (initialOwner)
/// * `init_data` - Initialization calldata (_data)
///
/// # Returns
/// Hex-encoded constructor arguments (without 0x prefix) for forge verify-contract.
pub fn encode_proxy_constructor_args(
    impl_addr: Address,
    proxy_admin: Address,
    init_data: &Bytes,
) -> String {
    let encoded = (impl_addr, proxy_admin, init_data.clone()).abi_encode_params();
    hex::encode(encoded)
}

/// Encode ZKsyncOSDualVerifier constructor arguments as hex string.
///
/// Constructor signature: `constructor(IVerifierV2 _fflonkVerifier, IVerifier _plonkVerifier, address _initialOwner)`
///
/// # Arguments
/// * `fflonk` - Fflonk verifier address
/// * `plonk` - Plonk verifier address
/// * `owner` - Initial owner address
///
/// # Returns
/// Hex-encoded constructor arguments (without 0x prefix) for forge verify-contract.
pub fn encode_verifier_constructor_args(fflonk: Address, plonk: Address, owner: Address) -> String {
    let encoded = (fflonk, plonk, owner).abi_encode_params();
    hex::encode(encoded)
}

/// Encode EraDualVerifier constructor arguments as hex string.
///
/// Constructor signature: `constructor(IVerifierV2 _fflonkVerifier, IVerifier _plonkVerifier)`
///
/// # Arguments
/// * `fflonk` - Fflonk verifier address
/// * `plonk` - Plonk verifier address
///
/// # Returns
/// Hex-encoded constructor arguments (without 0x prefix) for forge verify-contract.
pub fn encode_era_verifier_constructor_args(fflonk: Address, plonk: Address) -> String {
    let encoded = (fflonk, plonk).abi_encode_params();
    hex::encode(encoded)
}

/// Encode ChainAdminOwnable constructor arguments as hex string.
///
/// Constructor signature: `constructor(address _initialOwner, address _initialTokenMultiplierSetter)`
///
/// # Arguments
/// * `owner` - Initial owner address
/// * `token_multiplier_setter` - Token multiplier setter address (typically zero)
///
/// # Returns
/// Hex-encoded constructor arguments (without 0x prefix) for forge verify-contract.
pub fn encode_chain_admin_constructor_args(
    owner: Address,
    token_multiplier_setter: Address,
) -> String {
    let encoded = (owner, token_multiplier_setter).abi_encode_params();
    hex::encode(encoded)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_encode_proxy_constructor_args() {
        let impl_addr = Address::from_str("0x1234567890123456789012345678901234567890").unwrap();
        let proxy_admin = Address::from_str("0xabcdefabcdefabcdefabcdefabcdefabcdefabcd").unwrap();
        let init_data = Bytes::new();

        let encoded = encode_proxy_constructor_args(impl_addr, proxy_admin, &init_data);

        // Should be hex string without 0x prefix
        assert!(!encoded.starts_with("0x"));
        // ABI encoding of (address, address, bytes) with empty bytes
        // = 32 bytes for impl + 32 bytes for admin + 32 bytes offset + 32 bytes length + 0 bytes data
        assert_eq!(encoded.len(), 256); // 128 bytes = 256 hex chars
    }

    #[test]
    fn test_encode_verifier_constructor_args() {
        let fflonk = Address::from_str("0x1111111111111111111111111111111111111111").unwrap();
        let plonk = Address::from_str("0x2222222222222222222222222222222222222222").unwrap();
        let owner = Address::from_str("0x3333333333333333333333333333333333333333").unwrap();

        let encoded = encode_verifier_constructor_args(fflonk, plonk, owner);

        // Should be hex string without 0x prefix
        assert!(!encoded.starts_with("0x"));
        // ABI encoding of (address, address, address) = 3 * 32 bytes = 96 bytes = 192 hex chars
        assert_eq!(encoded.len(), 192);
    }

    #[test]
    fn test_encode_chain_admin_constructor_args() {
        let owner = Address::from_str("0x1111111111111111111111111111111111111111").unwrap();
        let token_multiplier_setter = Address::ZERO;

        let encoded = encode_chain_admin_constructor_args(owner, token_multiplier_setter);

        // Should be hex string without 0x prefix
        assert!(!encoded.starts_with("0x"));
        // ABI encoding of (address, address) = 2 * 32 bytes = 64 bytes = 128 hex chars
        assert_eq!(encoded.len(), 128);
    }
}
