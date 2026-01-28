//! Ecosystem domain logic for ZkSync ecosystem management.
//!
//! This module contains types and functions for managing ZkSync ecosystems,
//! including configuration, contracts, and wallets.
//!
//! An ecosystem is the top-level container for ZkSync infrastructure and can
//! contain multiple chains. Each ecosystem has:
//!
//! - A unique name
//! - Settlement network configuration (Mainnet, Sepolia, Localhost)
//! - Deployed contracts (Bridgehub, Governance, Verifier, etc.)
//! - Wallet keypairs (deployer, governor)
//! - Protocol version tracking
//!
//! # Example
//!
//! ```rust
//! use adi_cli::ecosystem::{Ecosystem, SettlementNetwork};
//! use semver::Version;
//!
//! // Create an ecosystem on Sepolia
//! let network = SettlementNetwork::Sepolia;
//! let version = Version::new(29, 0, 11);
//!
//! // Convert version to on-chain hex format
//! let hex = version_to_hex(&version);
//! ```

pub mod config;
pub mod contracts;
pub mod wallets;

// Re-export commonly used types
// Note: Currently unused as commands are implemented in later phases (US1-US6)
#[allow(unused_imports)]
pub use config::{Ecosystem, SettlementNetwork, Upgrade, UpgradeCalldata, UpgradeStatus};
#[allow(unused_imports)]
pub use contracts::EcosystemContracts;
#[allow(unused_imports)]
pub use wallets::{EcosystemWallets, Wallet};

use alloy_primitives::U256;
use semver::Version;

/// Converts a semver [`Version`] to the on-chain hex representation.
///
/// The encoding format is: `((major << 32) | (minor << 24) | patch)`
///
/// # Examples
///
/// ```rust
/// use semver::Version;
/// use adi_cli::ecosystem::version_to_hex;
///
/// let v29 = Version::new(29, 0, 0);
/// assert_eq!(version_to_hex(&v29), U256::from(0x1d00000000u64));
///
/// let v30 = Version::new(30, 0, 0);
/// assert_eq!(version_to_hex(&v30), U256::from(0x1e00000000u64));
///
/// let v30_0_1 = Version::new(30, 0, 1);
/// assert_eq!(version_to_hex(&v30_0_1), U256::from(0x1e00000001u64));
/// ```
// Note: Currently unused as commands are implemented in later phases (US1-US6)
#[allow(dead_code)]
pub fn version_to_hex(version: &Version) -> U256 {
    let major = version.major;
    let minor = version.minor;
    let patch = version.patch;
    U256::from((major << 32) | (minor << 24) | patch)
}

/// Parses an on-chain hex representation back to a semver [`Version`].
///
/// The encoding format is: `((major << 32) | (minor << 24) | patch)`
///
/// # Examples
///
/// ```rust
/// use alloy_primitives::U256;
/// use semver::Version;
/// use adi_cli::ecosystem::hex_to_version;
///
/// let hex = U256::from(0x1d00000000u64);
/// assert_eq!(hex_to_version(hex), Version::new(29, 0, 0));
///
/// let hex = U256::from(0x1e00000001u64);
/// assert_eq!(hex_to_version(hex), Version::new(30, 0, 1));
/// ```
// Note: Currently unused as commands are implemented in later phases (US1-US6)
#[allow(dead_code)]
pub fn hex_to_version(hex: U256) -> Version {
    // Get the lowest limb (first 64 bits) - sufficient for version encoding
    let limbs = hex.as_limbs();
    let value = *limbs.first().unwrap_or(&0);

    let major = value >> 32;
    let minor = (value >> 24) & 0xFF;
    let patch = value & 0xFFFFFF;

    Version::new(major, minor, patch)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_to_hex() {
        // v29.0.0 -> 0x1d00000000
        let v29 = Version::new(29, 0, 0);
        assert_eq!(version_to_hex(&v29), U256::from(0x1d00000000u64));

        // v30.0.0 -> 0x1e00000000
        let v30 = Version::new(30, 0, 0);
        assert_eq!(version_to_hex(&v30), U256::from(0x1e00000000u64));

        // v30.0.1 -> 0x1e00000001
        let v30_0_1 = Version::new(30, 0, 1);
        assert_eq!(version_to_hex(&v30_0_1), U256::from(0x1e00000001u64));

        // v29.0.11 -> 0x1d0000000b
        let v29_0_11 = Version::new(29, 0, 11);
        assert_eq!(version_to_hex(&v29_0_11), U256::from(0x1d0000000bu64));
    }

    #[test]
    fn test_hex_to_version() {
        // 0x1d00000000 -> v29.0.0
        assert_eq!(
            hex_to_version(U256::from(0x1d00000000u64)),
            Version::new(29, 0, 0)
        );

        // 0x1e00000000 -> v30.0.0
        assert_eq!(
            hex_to_version(U256::from(0x1e00000000u64)),
            Version::new(30, 0, 0)
        );

        // 0x1e00000001 -> v30.0.1
        assert_eq!(
            hex_to_version(U256::from(0x1e00000001u64)),
            Version::new(30, 0, 1)
        );
    }

    #[test]
    fn test_roundtrip() {
        let versions = vec![
            Version::new(29, 0, 0),
            Version::new(29, 0, 11),
            Version::new(30, 0, 0),
            Version::new(30, 0, 1),
            Version::new(30, 1, 0),
            Version::new(31, 2, 3),
        ];

        for version in versions {
            let hex = version_to_hex(&version);
            let recovered = hex_to_version(hex);
            assert_eq!(version, recovered, "Roundtrip failed for {}", version);
        }
    }
}
