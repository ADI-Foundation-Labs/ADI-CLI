//! Toolkit image configuration.
//!
// Note: This module is part of the Toolkit API (T099-T101).
// It will be used by command implementations that execute in Docker containers.
#![allow(dead_code)]
//!
//! Builds Docker image references for different protocol versions
//! of the ADI toolkit.
//!
//! # Image Naming Convention
//!
//! Images follow the format: `{registry}/{image_name}:v{major}.{minor}.{patch}`
//!
//! Example: `harbor.io/adi/adi-toolkit:v29.0.11`
//!
//! # Example
//!
//! ```rust,ignore
//! use adi_cli::toolkit::ToolkitConfig;
//! use semver::Version;
//!
//! let config = ToolkitConfig::default();
//! let image = config.image_for_version(&Version::new(29, 0, 11));
//! assert_eq!(image, "harbor.io/adi/adi-toolkit:v29.0.11");
//! ```

use crate::config::DockerConfig;
use semver::Version;

/// Toolkit image configuration.
///
/// Manages the image naming and versioning for the ADI toolkit
/// Docker images.
#[derive(Debug, Clone)]
pub struct ToolkitConfig {
    /// Docker registry URL.
    registry: String,
    /// Toolkit image name.
    image_name: String,
}

impl Default for ToolkitConfig {
    fn default() -> Self {
        Self {
            registry: crate::config::DEFAULT_DOCKER_REGISTRY.to_string(),
            image_name: crate::config::DEFAULT_DOCKER_IMAGE_NAME.to_string(),
        }
    }
}

impl From<&DockerConfig> for ToolkitConfig {
    fn from(docker: &DockerConfig) -> Self {
        Self {
            registry: docker.registry.clone(),
            image_name: docker.image_name.clone(),
        }
    }
}

impl ToolkitConfig {
    /// Create a new toolkit configuration.
    ///
    /// # Arguments
    ///
    /// * `registry` - Docker registry URL (e.g., "harbor.io/adi").
    /// * `image_name` - Toolkit image name (e.g., "adi-toolkit").
    pub fn new(registry: impl Into<String>, image_name: impl Into<String>) -> Self {
        Self {
            registry: registry.into(),
            image_name: image_name.into(),
        }
    }

    /// Get the full image reference for a protocol version.
    ///
    /// # Arguments
    ///
    /// * `version` - Protocol version.
    ///
    /// # Returns
    ///
    /// Full image reference (e.g., "harbor.io/adi/adi-toolkit:v29.0.11").
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let config = ToolkitConfig::default();
    /// let image = config.image_for_version(&Version::new(29, 0, 11));
    /// ```
    pub fn image_for_version(&self, version: &Version) -> String {
        format!(
            "{}/{}:v{}.{}.{}",
            self.registry, self.image_name, version.major, version.minor, version.patch
        )
    }

    /// Get the full image reference from a version string.
    ///
    /// # Arguments
    ///
    /// * `version_str` - Version string (e.g., "29.0.11").
    ///
    /// # Returns
    ///
    /// Full image reference (e.g., "harbor.io/adi/adi-toolkit:v29.0.11").
    pub fn image_for_version_str(&self, version_str: &str) -> String {
        format!("{}/{}:v{}", self.registry, self.image_name, version_str)
    }

    /// Get the registry URL.
    pub fn registry(&self) -> &str {
        &self.registry
    }

    /// Get the image name.
    pub fn image_name(&self) -> &str {
        &self.image_name
    }

    /// Parse a version string into a semver Version.
    ///
    /// # Arguments
    ///
    /// * `version_str` - Version string (e.g., "29.0.11" or "v29.0.11").
    ///
    /// # Returns
    ///
    /// Parsed Version or error if invalid.
    pub fn parse_version(version_str: &str) -> Result<Version, semver::Error> {
        let cleaned = version_str.strip_prefix('v').unwrap_or(version_str);
        Version::parse(cleaned)
    }
}

/// Container paths for the toolkit.
///
/// These are the standard paths inside the toolkit container
/// for various artifacts and working directories.
#[derive(Debug, Clone)]
pub struct ContainerPaths;

impl ContainerPaths {
    /// Working directory inside the container.
    pub const WORKSPACE: &'static str = "/workspace";

    /// Era contracts directory inside the container.
    pub const ERA_CONTRACTS: &'static str = "/era-contracts";

    /// zkstack CLI binary path.
    pub const ZKSTACK_BIN: &'static str = "zkstack";

    /// Forge binary path.
    pub const FORGE_BIN: &'static str = "forge";

    /// Cast binary path.
    pub const CAST_BIN: &'static str = "cast";
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ToolkitConfig::default();
        assert_eq!(config.registry(), "harbor.io/adi");
        assert_eq!(config.image_name(), "adi-toolkit");
    }

    #[test]
    fn test_image_for_version() {
        let config = ToolkitConfig::default();
        let version = Version::new(29, 0, 11);
        let image = config.image_for_version(&version);
        assert_eq!(image, "harbor.io/adi/adi-toolkit:v29.0.11");
    }

    #[test]
    fn test_image_for_version_str() {
        let config = ToolkitConfig::default();
        let image = config.image_for_version_str("30.0.0");
        assert_eq!(image, "harbor.io/adi/adi-toolkit:v30.0.0");
    }

    #[test]
    fn test_custom_config() {
        let config = ToolkitConfig::new("my-registry.io/team", "custom-toolkit");
        let version = Version::new(29, 0, 11);
        let image = config.image_for_version(&version);
        assert_eq!(image, "my-registry.io/team/custom-toolkit:v29.0.11");
    }

    #[test]
    fn test_parse_version() {
        assert_eq!(
            ToolkitConfig::parse_version("29.0.11").unwrap(),
            Version::new(29, 0, 11)
        );
        assert_eq!(
            ToolkitConfig::parse_version("v29.0.11").unwrap(),
            Version::new(29, 0, 11)
        );
    }

    #[test]
    fn test_from_docker_config() {
        let docker = DockerConfig {
            registry: "custom.io".to_string(),
            image_name: "my-toolkit".to_string(),
        };
        let toolkit = ToolkitConfig::from(&docker);
        assert_eq!(toolkit.registry(), "custom.io");
        assert_eq!(toolkit.image_name(), "my-toolkit");
    }
}
