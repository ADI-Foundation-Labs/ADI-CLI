//! Configuration types for toolkit operations.

use adi_types::Logger;
use semver::Version;
use serde::{Deserialize, Serialize};

/// Default registry for ADI toolkit images.
pub const DEFAULT_REGISTRY: &str = "harbor-v2.dev.internal.adifoundation.ai/adi-chain/cli";

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
/// assert_eq!(config.registry, "harbor-v2.dev.internal.adifoundation.ai/adi-chain/cli");
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolkitConfig {
    /// Container registry URL.
    pub registry: String,

    /// Base image name.
    pub image_name: String,

    /// Timeout for container operations in seconds.
    pub timeout_seconds: u64,

    /// Optional tag override. When set, bypasses protocol version-derived tag.
    pub tag_override: Option<String>,
}

impl Default for ToolkitConfig {
    fn default() -> Self {
        Self {
            registry: DEFAULT_REGISTRY.to_string(),
            image_name: DEFAULT_IMAGE_NAME.to_string(),
            timeout_seconds: DEFAULT_TIMEOUT_SECONDS,
            tag_override: None,
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
            tag_override: None,
        }
    }

    /// Set the timeout for container operations.
    #[must_use]
    pub fn with_timeout(mut self, timeout_seconds: u64) -> Self {
        self.timeout_seconds = timeout_seconds;
        self
    }

    /// Set a custom image tag override.
    ///
    /// When set, this overrides the protocol version-derived tag.
    #[must_use]
    pub fn with_tag_override(mut self, tag: impl Into<String>) -> Self {
        self.tag_override = Some(tag.into());
        self
    }

    /// Build an image reference for a specific protocol version.
    ///
    /// If `tag_override` is set, it will be used instead of the version-derived tag.
    ///
    /// # Arguments
    ///
    /// * `version` - The protocol version (e.g., 29.0.11).
    /// * `logger` - Logger for debug output.
    ///
    /// # Returns
    ///
    /// An `ImageReference` with the full image URI.
    pub fn image_reference(&self, version: &Version, logger: &dyn Logger) -> ImageReference {
        let tag = self
            .tag_override
            .clone()
            .unwrap_or_else(|| format!("v{}.{}.{}", version.major, version.minor, version.patch));
        logger.debug(&format!(
            "Building image reference: registry={}, image={}, tag={} (override={})",
            self.registry,
            self.image_name,
            tag,
            self.tag_override.is_some()
        ));
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
/// use adi_types::NoopLogger;
/// use semver::Version;
///
/// let config = ToolkitConfig::default();
/// let version = Version::new(30, 0, 2);
/// let logger = NoopLogger;
/// let image_ref = config.image_reference(&version, &logger);
///
/// assert_eq!(image_ref.full_uri(), "harbor-v2.dev.internal.adifoundation.ai/adi-chain/cli/adi-toolkit:v30.0.2");
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
    use adi_types::NoopLogger;

    #[test]
    fn test_default_config() {
        let config = ToolkitConfig::default();
        assert_eq!(
            config.registry,
            "harbor-v2.dev.internal.adifoundation.ai/adi-chain/cli"
        );
        assert_eq!(config.image_name, "adi-toolkit");
        assert_eq!(config.timeout_seconds, 1800);
    }

    #[test]
    fn test_image_reference_format() {
        let logger = NoopLogger;
        let config = ToolkitConfig::default();
        let version = Version::new(30, 0, 2);
        let image_ref = config.image_reference(&version, &logger);

        assert_eq!(
            image_ref.full_uri(),
            "harbor-v2.dev.internal.adifoundation.ai/adi-chain/cli/adi-toolkit:v30.0.2"
        );
    }

    #[test]
    fn test_custom_config() {
        let config = ToolkitConfig::new("my-registry.io", "my-toolkit").with_timeout(3600);

        assert_eq!(config.registry, "my-registry.io");
        assert_eq!(config.image_name, "my-toolkit");
        assert_eq!(config.timeout_seconds, 3600);
    }

    #[test]
    fn test_tag_override() {
        let logger = NoopLogger;
        let config = ToolkitConfig::default().with_tag_override("custom-tag");
        let version = Version::new(30, 0, 2);
        let image_ref = config.image_reference(&version, &logger);

        assert_eq!(image_ref.tag, "custom-tag");
        assert_eq!(
            image_ref.full_uri(),
            "harbor-v2.dev.internal.adifoundation.ai/adi-chain/cli/adi-toolkit:custom-tag"
        );
    }

    #[test]
    fn test_no_tag_override_uses_version() {
        let logger = NoopLogger;
        let config = ToolkitConfig::default();
        let version = Version::new(30, 0, 2);
        let image_ref = config.image_reference(&version, &logger);

        assert_eq!(image_ref.tag, "v30.0.2");
    }
}
