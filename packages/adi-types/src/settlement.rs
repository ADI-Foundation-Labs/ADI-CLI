//! Settlement layer of an ecosystem's chains.

use clap::ValueEnum;
use serde::{de::Deserializer, Deserialize, Serialize, Serializer};
use strum::{Display, EnumString};

/// The layer an ecosystem's chains settle on.
///
/// This axis is orthogonal to the data-availability transport ([`crate::PubdataMode`]):
/// a chain can post blobs or calldata regardless of where it settles. It exists
/// because the L1-sender fee tier depends on the settlement layer, not the DA
/// transport — settling on Ethereum L1 has a very different gas market than
/// settling on an L2.
///
/// - [`Self::L1`] — settles on Ethereum L1. The chain is itself an L2.
/// - [`Self::L2`] — settles on an L2 (gateway). The chain is an L3.
///
/// There is intentionally no `L3` value: settling on an L2 is exactly what makes
/// a chain an L3.
///
/// # Serde Behavior
/// - Serializes to `"l1"` / `"l2"`.
/// - Deserializes case-insensitively (`L1`, `l1`, ...).
#[derive(Clone, Copy, Debug, Default, Display, EnumString, PartialEq, Eq, ValueEnum)]
#[strum(serialize_all = "lowercase")]
pub enum SettlementLayer {
    /// Settles on Ethereum L1 (the chain is an L2).
    L1,
    /// Settles on an L2 gateway (the chain is an L3). Default.
    #[default]
    L2,
}

impl SettlementLayer {
    /// Whether the chain settles on Ethereum L1 (i.e. is itself an L2).
    #[must_use]
    pub fn is_l1(&self) -> bool {
        matches!(self, Self::L1)
    }
}

impl Serialize for SettlementLayer {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(match self {
            Self::L1 => "l1",
            Self::L2 => "l2",
        })
    }
}

impl<'de> Deserialize<'de> for SettlementLayer {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.to_lowercase().as_str() {
            "l1" => Ok(Self::L1),
            "l2" => Ok(Self::L2),
            _ => Err(serde::de::Error::unknown_variant(&s, &["l1", "l2"])),
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn default_is_l2() {
        assert_eq!(SettlementLayer::default(), SettlementLayer::L2);
    }

    #[test]
    fn is_l1_only_for_l1() {
        assert!(SettlementLayer::L1.is_l1());
        assert!(!SettlementLayer::L2.is_l1());
    }

    #[test]
    fn deserialize_case_insensitive() {
        assert_eq!(
            serde_yaml::from_str::<SettlementLayer>("l1").unwrap(),
            SettlementLayer::L1
        );
        assert_eq!(
            serde_yaml::from_str::<SettlementLayer>("L2").unwrap(),
            SettlementLayer::L2
        );
    }

    #[test]
    fn serialize_roundtrip() {
        for layer in [SettlementLayer::L1, SettlementLayer::L2] {
            let s = serde_yaml::to_string(&layer).unwrap();
            let back: SettlementLayer = serde_yaml::from_str(&s).unwrap();
            assert_eq!(layer, back);
        }
    }

    #[test]
    fn from_str_lowercase() {
        assert_eq!(
            "l1".parse::<SettlementLayer>().unwrap(),
            SettlementLayer::L1
        );
        assert_eq!(
            "l2".parse::<SettlementLayer>().unwrap(),
            SettlementLayer::L2
        );
    }
}
