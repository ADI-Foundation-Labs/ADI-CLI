//! Configuration types for toolkit operations.

use semver::Version;
use serde::{Deserialize, Serialize};

/// Default registry for ADI toolkit images.
pub const DEFAULT_REGISTRY: &str = "harbor.sde.adifoundation.ai/adi-chain/cli";

/// Default image name for ADI toolkit.
pub const DEFAULT_IMAGE_NAME: &str = "adi-toolkit";

/// Default timeout in seconds (30 minutes).
pub const DEFAULT_TIMEOUT_SECONDS: u64 = 1800;

/// Configuration for toolkit image orchestration.
///
/// This configuration specifies where to pull toolkit images from
/// and how long to wait for container operations.
///
/// # Example
///
/// ```rust
/// use adi_toolkit::ToolkitConfig;
///
/// let config = ToolkitConfig::default();
/// assert_eq!(config.registry, "harbor.sde.adifoundation.ai/adi-chain/cli");
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolkitConfig {
    /// Container registry URL.
    pub registry: String,

    /// Base image name.
    pub image_name: String,

    /// Timeout for container operations in seconds.
    pub timeout_seconds: u64,
}

impl Default for ToolkitConfig {
    fn default() -> Self {
        Self {
            registry: DEFAULT_REGISTRY.to_string(),
            image_name: DEFAULT_IMAGE_NAME.to_string(),
            timeout_seconds: DEFAULT_TIMEOUT_SECONDS,
        }
    }
}

impl ToolkitConfig {
    /// Create a new ToolkitConfig with custom values.
    ///
    /// # Arguments
    ///
    /// * `registry` - Container registry URL.
    /// * `image_name` - Base image name.
    pub fn new(registry: impl Into<String>, image_name: impl Into<String>) -> Self {
        Self {
            registry: registry.into(),
            image_name: image_name.into(),
            timeout_seconds: DEFAULT_TIMEOUT_SECONDS,
        }
    }

    /// Set the timeout for container operations.
    #[must_use]
    pub fn with_timeout(mut self, timeout_seconds: u64) -> Self {
        self.timeout_seconds = timeout_seconds;
        self
    }

    /// Build an image reference for a specific protocol version.
    ///
    /// # Arguments
    ///
    /// * `version` - The protocol version (e.g., 29.0.11).
    ///
    /// # Returns
    ///
    /// An `ImageReference` with the full image URI.
    pub fn image_reference(&self, version: &Version) -> ImageReference {
        let tag = format!("v{}.{}.{}", version.major, version.minor, version.patch);
        log::debug!(
            "Building image reference: registry={}, image={}, tag={}",
            self.registry,
            self.image_name,
            tag
        );
        ImageReference {
            registry: self.registry.clone(),
            image_name: self.image_name.clone(),
            tag,
        }
    }
}

/// A fully qualified Docker image reference.
///
/// # Example
///
/// ```rust
/// use adi_toolkit::{ToolkitConfig, ImageReference};
/// use semver::Version;
///
/// let config = ToolkitConfig::default();
/// let version = Version::new(30, 0, 2);
/// let image_ref = config.image_reference(&version);
///
/// assert_eq!(image_ref.full_uri(), "harbor.sde.adifoundation.ai/adi-chain/cli/adi-toolkit:v30.0.2");
/// ```
#[derive(Debug, Clone)]
pub struct ImageReference {
    /// Container registry URL.
    pub registry: String,

    /// Image name.
    pub image_name: String,

    /// Image tag.
    pub tag: String,
}

impl ImageReference {
    /// Returns the full image URI.
    ///
    /// Format: `{registry}/{image_name}:{tag}`
    #[must_use]
    pub fn full_uri(&self) -> String {
        format!("{}/{}:{}", self.registry, self.image_name, self.tag)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ToolkitConfig::default();
        assert_eq!(config.registry, "harbor.sde.adifoundation.ai/adi-chain/cli");
        assert_eq!(config.image_name, "adi-toolkit");
        assert_eq!(config.timeout_seconds, 1800);
    }

    #[test]
    fn test_image_reference_format() {
        let config = ToolkitConfig::default();
        let version = Version::new(30, 0, 2);
        let image_ref = config.image_reference(&version);

        assert_eq!(
            image_ref.full_uri(),
            "harbor.sde.adifoundation.ai/adi-chain/cli/adi-toolkit:v30.0.2"
        );
    }

    #[test]
    fn test_custom_config() {
        let config = ToolkitConfig::new("my-registry.io", "my-toolkit").with_timeout(3600);

        assert_eq!(config.registry, "my-registry.io");
        assert_eq!(config.image_name, "my-toolkit");
        assert_eq!(config.timeout_seconds, 3600);
    }
}
