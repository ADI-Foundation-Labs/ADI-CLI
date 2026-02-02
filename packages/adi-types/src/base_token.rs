//! Base token configuration types.

use alloy_primitives::Address;
use serde::{Deserialize, Serialize};

/// ETH native token address constant.
///
/// This is the special address used to represent ETH as the base token.
/// Value: `0x0000000000000000000000000000000000000001`
pub const ETH_TOKEN_ADDRESS: Address = Address::with_last_byte(1);

/// ETH native token address as a string constant.
pub const ETH_TOKEN_ADDRESS_STR: &str = "0x0000000000000000000000000000000000000001";

/// Base token configuration for a chain.
///
/// Defines the native gas token for the chain.
/// ETH uses address `0x0000000000000000000000000000000000000001`.
///
/// # Example YAML
/// ```yaml
/// base_token:
///   address: "0x0000000000000000000000000000000000000001"
///   nominator: 1
///   denominator: 1
/// ```
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BaseToken {
    /// Token contract address on settlement layer.
    pub address: Address,

    /// Price nominator for gas calculations.
    pub nominator: u64,

    /// Price denominator for gas calculations.
    pub denominator: u64,
}

impl Default for BaseToken {
    fn default() -> Self {
        Self::eth()
    }
}

impl BaseToken {
    /// Creates a new base token configuration.
    pub fn new(address: Address, nominator: u64, denominator: u64) -> Self {
        Self {
            address,
            nominator,
            denominator,
        }
    }

    /// Creates a base token configuration for ETH.
    pub fn eth() -> Self {
        Self {
            address: ETH_TOKEN_ADDRESS,
            nominator: 1,
            denominator: 1,
        }
    }

    /// Returns true if this is the ETH base token.
    pub fn is_eth(&self) -> bool {
        self.address == ETH_TOKEN_ADDRESS
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_base_token_default_is_eth() {
        let token = BaseToken::default();
        assert!(token.is_eth());
        assert_eq!(token.nominator, 1);
        assert_eq!(token.denominator, 1);
    }

    #[test]
    fn test_base_token_serde() {
        let yaml = r#"
address: "0x0000000000000000000000000000000000000001"
nominator: 1
denominator: 1
"#;
        let token: BaseToken = serde_yaml::from_str(yaml).unwrap();
        assert!(token.is_eth());
    }

    #[test]
    fn test_custom_base_token() {
        let address: Address = "0x1234567890123456789012345678901234567890"
            .parse()
            .unwrap();
        let token = BaseToken::new(address, 2, 3);
        assert!(!token.is_eth());
        assert_eq!(token.nominator, 2);
        assert_eq!(token.denominator, 3);
    }

    #[test]
    fn test_eth_address_constant() {
        let parsed: Address = ETH_TOKEN_ADDRESS_STR.parse().unwrap();
        assert_eq!(parsed, ETH_TOKEN_ADDRESS);
    }
}
