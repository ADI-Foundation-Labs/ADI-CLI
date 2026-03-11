//! Operator address overrides for on-chain role assignment.
//!
//! When users provide custom operator addresses via CLI/config,
//! these are stored in operators.yaml and used for on-chain role
//! assignment instead of the addresses from wallets.yaml.

use alloy_primitives::Address;
use serde::{Deserialize, Serialize};

/// Operator address overrides for on-chain role assignment.
///
/// These addresses override the default operator addresses from wallets.yaml
/// when assigning validator roles via `ValidatorTimelock`.
///
/// # Example YAML
/// ```yaml
/// operator: "0x1234567890123456789012345678901234567890"
/// prove_operator: "0xabcdefabcdefabcdefabcdefabcdefabcdefabcd"
/// execute_operator: "0x9876543210987654321098765432109876543210"
/// ```
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Operators {
    /// Commit operator address - receives PRECOMMITTER, COMMITTER, REVERTER roles.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operator: Option<Address>,

    /// Prove operator address - receives PROVER role.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prove_operator: Option<Address>,

    /// Execute operator address - receives EXECUTOR role.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub execute_operator: Option<Address>,

    /// Blob operator address - receives PRECOMMITTER, COMMITTER, REVERTER roles by default from zkstack.
    /// This field is used for revoking these default roles.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub blob_operator: Option<Address>,
}

impl Operators {
    /// Creates an empty operators configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if all required operators are configured.
    pub fn is_complete(&self) -> bool {
        self.operator.is_some() && self.prove_operator.is_some() && self.execute_operator.is_some()
    }

    /// Check if any operator is configured.
    pub fn has_any(&self) -> bool {
        self.operator.is_some()
            || self.prove_operator.is_some()
            || self.execute_operator.is_some()
            || self.blob_operator.is_some()
    }

    /// Get all configured operator addresses.
    pub fn all_addresses(&self) -> Vec<Address> {
        [
            self.operator,
            self.prove_operator,
            self.execute_operator,
            self.blob_operator,
        ]
        .into_iter()
        .flatten()
        .collect()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_operators_default() {
        let ops = Operators::default();
        assert!(ops.operator.is_none());
        assert!(ops.prove_operator.is_none());
        assert!(ops.execute_operator.is_none());
        assert!(ops.blob_operator.is_none());
        assert!(!ops.is_complete());
        assert!(!ops.has_any());
    }

    #[test]
    fn test_operators_is_complete() {
        let mut ops = Operators::default();
        assert!(!ops.is_complete());

        ops.operator = Some(
            "0x1234567890123456789012345678901234567890"
                .parse()
                .unwrap(),
        );
        assert!(!ops.is_complete());

        ops.prove_operator = Some(
            "0xabcdefabcdefabcdefabcdefabcdefabcdefabcd"
                .parse()
                .unwrap(),
        );
        assert!(!ops.is_complete());

        ops.execute_operator = Some(
            "0x9876543210987654321098765432109876543210"
                .parse()
                .unwrap(),
        );
        assert!(ops.is_complete());
    }

    #[test]
    fn test_operators_has_any() {
        let mut ops = Operators::default();
        assert!(!ops.has_any());

        ops.operator = Some(
            "0x1234567890123456789012345678901234567890"
                .parse()
                .unwrap(),
        );
        assert!(ops.has_any());
    }

    #[test]
    fn test_operators_all_addresses() {
        let ops = Operators {
            operator: Some(
                "0x1234567890123456789012345678901234567890"
                    .parse()
                    .unwrap(),
            ),
            prove_operator: Some(
                "0xabcdefabcdefabcdefabcdefabcdefabcdefabcd"
                    .parse()
                    .unwrap(),
            ),
            execute_operator: None,
            ..Default::default()
        };

        let addrs = ops.all_addresses();
        assert_eq!(addrs.len(), 2);
    }

    #[test]
    fn test_operators_deserialize() {
        let yaml = r#"
operator: "0x1234567890123456789012345678901234567890"
prove_operator: "0xabcdefabcdefabcdefabcdefabcdefabcdefabcd"
"#;
        let ops: Operators = serde_yaml::from_str(yaml).unwrap();
        assert!(ops.operator.is_some());
        assert!(ops.prove_operator.is_some());
        assert!(ops.execute_operator.is_none());
    }

    #[test]
    fn test_operators_serialize_skips_none() {
        let ops = Operators {
            operator: Some(
                "0x1234567890123456789012345678901234567890"
                    .parse()
                    .unwrap(),
            ),
            prove_operator: None,
            execute_operator: None,
            ..Default::default()
        };

        let yaml = serde_yaml::to_string(&ops).unwrap();
        assert!(yaml.contains("operator:"));
        assert!(!yaml.contains("prove_operator:"));
        assert!(!yaml.contains("execute_operator:"));
    }
}
