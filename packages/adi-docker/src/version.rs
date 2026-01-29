//! Protocol version definitions for ADI toolkit images.
//!
//! This module defines the supported protocol versions that determine
//! which Docker image tag to use for toolkit operations.

use std::str::FromStr;

use strum::{Display, EnumIter, EnumString, IntoEnumIterator};
use thiserror::Error;

/// Error type for protocol version parsing.
#[derive(Error, Debug)]
#[error(
    "Unsupported protocol version: {input}. Supported versions: {}",
    ProtocolVersion::supported_versions_string()
)]
pub struct ParseError {
    input: String,
}

/// Supported protocol versions for ADI toolkit images.
///
/// Each variant corresponds to a specific toolkit image tag.
/// The version determines which era-contracts, zkstack, and foundry-zksync
/// versions are bundled in the toolkit image.
///
/// # Example
///
/// ```rust
/// use adi_docker::ProtocolVersion;
/// use std::str::FromStr;
///
/// let version = ProtocolVersion::from_str("v29.0.11").unwrap();
/// assert_eq!(version, ProtocolVersion::V29_0_11);
/// assert_eq!(version.to_string(), "v29.0.11");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Display, EnumString, EnumIter)]
pub enum ProtocolVersion {
    /// Protocol version 29.0.11
    #[strum(serialize = "v29.0.11", serialize = "29.0.11")]
    V29_0_11,
}

impl ProtocolVersion {
    /// Converts the protocol version to a semver::Version.
    #[must_use]
    pub fn to_semver(&self) -> semver::Version {
        match self {
            ProtocolVersion::V29_0_11 => semver::Version::new(29, 0, 11),
        }
    }

    /// Returns a comma-separated string of all supported versions.
    #[must_use]
    pub fn supported_versions_string() -> String {
        Self::iter()
            .map(|v| v.to_string())
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// Parse from string with custom error type.
    ///
    /// # Errors
    ///
    /// Returns `ParseError` if the version string is not recognized.
    pub fn parse(s: &str) -> Result<Self, ParseError> {
        log::debug!("Parsing protocol version from: {}", s);

        let result = Self::from_str(s.trim());

        match &result {
            Ok(version) => log::debug!("Matched protocol version: {:?}", version),
            Err(_) => log::debug!("No match for version: {}", s),
        }

        result.map_err(|_| ParseError {
            input: s.to_string(),
        })
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_with_v_prefix() {
        let version = ProtocolVersion::parse("v29.0.11").unwrap();
        assert_eq!(version, ProtocolVersion::V29_0_11);
    }

    #[test]
    fn test_parse_without_prefix() {
        let version = ProtocolVersion::parse("29.0.11").unwrap();
        assert_eq!(version, ProtocolVersion::V29_0_11);
    }

    #[test]
    fn test_parse_unsupported_version() {
        let result = ProtocolVersion::parse("v99.0.0");
        assert!(result.is_err());
    }

    #[test]
    fn test_to_semver() {
        let version = ProtocolVersion::V29_0_11;
        let semver = version.to_semver();
        assert_eq!(semver.major, 29);
        assert_eq!(semver.minor, 0);
        assert_eq!(semver.patch, 11);
    }

    #[test]
    fn test_display() {
        assert_eq!(ProtocolVersion::V29_0_11.to_string(), "v29.0.11");
    }
}
