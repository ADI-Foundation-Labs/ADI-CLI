//! Protocol version handling.
//!
//! Defines supported protocol versions and provides parsing utilities.

use serde::{Deserialize, Serialize};
use std::fmt;
use strum::{EnumIter, EnumString};
use thiserror::Error;

/// Error type for version parsing failures.
#[derive(Error, Debug)]
pub enum ParseError {
    /// Unknown protocol version.
    #[error("Unknown protocol version '{version}'. Supported versions: {supported}", version = .0, supported = ProtocolVersion::supported_versions_string())]
    UnknownVersion(String),
}

/// Supported protocol versions.
///
/// Each version maps to a specific toolkit Docker image tag.
///
/// # Parsing
///
/// Versions can be parsed from strings in several formats:
/// - With prefix: `v0.30.1`
/// - Without prefix: `0.30.1`
///
/// # Example
///
/// ```rust
/// use adi_toolkit::ProtocolVersion;
///
/// let version = ProtocolVersion::parse("v0.30.1").unwrap();
/// assert_eq!(version.to_semver(), semver::Version::new(0, 30, 1));
/// ```
#[derive(
    Clone, Copy, Debug, Default, EnumString, EnumIter, Serialize, Deserialize, PartialEq, Eq,
)]
pub enum ProtocolVersion {
    /// Protocol version 0.30.0
    #[strum(serialize = "v0.30.0", serialize = "0.30.0")]
    V0_30_0,
    /// Protocol version 0.30.1
    #[default]
    #[strum(serialize = "v0.30.1", serialize = "0.30.1")]
    V0_30_1,
}

impl ProtocolVersion {
    /// Parse a protocol version from a string.
    ///
    /// Accepts both `vX.Y.Z` and `X.Y.Z` formats.
    ///
    /// # Arguments
    ///
    /// * `s` - The version string to parse.
    ///
    /// # Errors
    ///
    /// Returns `ParseError::UnknownVersion` if the version is not recognized.
    ///
    /// # Example
    ///
    /// ```rust
    /// use adi_toolkit::ProtocolVersion;
    ///
    /// let version = ProtocolVersion::parse("v0.30.1").unwrap();
    /// let version2 = ProtocolVersion::parse("0.30.1").unwrap();
    /// assert_eq!(version, version2);
    /// ```
    pub fn parse(s: &str) -> Result<Self, ParseError> {
        // Normalize: remove 'v' prefix if present
        let normalized = s.trim().to_lowercase();

        // Try direct parsing
        if let Ok(version) = normalized.parse::<ProtocolVersion>() {
            return Ok(version);
        }

        // Try with 'v' prefix
        let with_prefix = format!("v{}", normalized.trim_start_matches('v'));
        if let Ok(version) = with_prefix.parse::<ProtocolVersion>() {
            return Ok(version);
        }

        Err(ParseError::UnknownVersion(s.to_string()))
    }

    /// Convert to semver::Version for image tagging.
    ///
    /// # Example
    ///
    /// ```rust
    /// use adi_toolkit::ProtocolVersion;
    ///
    /// let version = ProtocolVersion::V0_30_1;
    /// let semver = version.to_semver();
    /// assert_eq!(semver.major, 0);
    /// assert_eq!(semver.minor, 30);
    /// assert_eq!(semver.patch, 1);
    /// ```
    #[must_use]
    pub fn to_semver(self) -> semver::Version {
        match self {
            ProtocolVersion::V0_30_0 => semver::Version::new(0, 30, 0),
            ProtocolVersion::V0_30_1 => semver::Version::new(0, 30, 1),
        }
    }

    /// Get a string listing all supported versions.
    ///
    /// Used for error messages.
    #[must_use]
    pub fn supported_versions_string() -> String {
        use strum::IntoEnumIterator;
        Self::iter()
            .map(|v| format!("v{}", v.to_semver()))
            .collect::<Vec<_>>()
            .join(", ")
    }
}

impl fmt::Display for ProtocolVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "v{}", self.to_semver())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_with_prefix() {
        let version = ProtocolVersion::parse("v0.30.1").unwrap();
        assert_eq!(version, ProtocolVersion::V0_30_1);
    }

    #[test]
    fn test_parse_without_prefix() {
        let version = ProtocolVersion::parse("0.30.1").unwrap();
        assert_eq!(version, ProtocolVersion::V0_30_1);
    }

    #[test]
    fn test_parse_unknown() {
        let result = ProtocolVersion::parse("1.0.0");
        assert!(result.is_err());
    }

    #[test]
    fn test_to_semver() {
        let version = ProtocolVersion::V0_30_1;
        let semver = version.to_semver();
        assert_eq!(semver, semver::Version::new(0, 30, 1));
    }

    #[test]
    fn test_display() {
        let version = ProtocolVersion::V0_30_1;
        assert_eq!(format!("{}", version), "v0.30.1");
    }
}
