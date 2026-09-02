//! ZkSync OS server version selection.

use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};

/// ZkSync OS server version to generate parameters for.
///
/// Unknown values are rejected by clap during argument parsing (via
/// [`ValueEnum`]) before this type is ever constructed.
///
/// # Serde Behavior
/// - Serializes to the dotted version string, e.g. `"v0.21.1"`.
/// - Deserializes from the same dotted version string.
#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    Display,
    EnumString,
    PartialEq,
    Eq,
    ValueEnum,
    Serialize,
    Deserialize,
)]
pub enum ServerVersion {
    /// Server v0.21.1
    #[default]
    #[strum(serialize = "v0.21.1")]
    #[value(name = "v0.21.1")]
    #[serde(rename = "v0.21.1")]
    V0211,
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn default_is_v0211() {
        assert_eq!(ServerVersion::default(), ServerVersion::V0211);
    }

    #[test]
    fn displays_as_dotted_version_string() {
        assert_eq!(ServerVersion::V0211.to_string(), "v0.21.1");
    }

    #[test]
    fn parses_known_version_via_value_enum() {
        let parsed = ServerVersion::from_str("v0.21.1", false).expect("should parse");
        assert_eq!(parsed, ServerVersion::V0211);
    }

    #[test]
    fn rejects_unknown_version_via_value_enum() {
        let result = ServerVersion::from_str("v9.9.9", false);
        assert!(result.is_err());
    }

    #[test]
    fn round_trips_through_json() {
        let json = serde_json::to_string(&ServerVersion::V0211).expect("should serialize");
        assert_eq!(json, "\"v0.21.1\"");
        let parsed: ServerVersion = serde_json::from_str(&json).expect("should deserialize");
        assert_eq!(parsed, ServerVersion::V0211);
    }
}
