//! Configuration types for Docker operations.

use semver::Version;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Default registry for ADI toolkit images.
pub const DEFAULT_REGISTRY: &str = "harbor.io/adi";

/// Default image name for ADI toolkit.
pub const DEFAULT_IMAGE_NAME: &str = "adi-toolkit";

/// Default timeout in seconds (30 minutes).
pub const DEFAULT_TIMEOUT_SECONDS: u64 = 1800;

/// Configuration for Docker toolkit image orchestration.
///
/// This configuration is used to specify where to pull toolkit images from
/// and how long to wait for container operations.
///
/// # Example
///
/// ```rust
/// use adi_docker::DockerConfig;
///
/// let config = DockerConfig::default();
/// assert_eq!(config.registry, "harbor.io/adi");
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockerConfig {
    /// Container registry URL (e.g., "harbor.io/adi").
    pub registry: String,

    /// Base image name (e.g., "adi-toolkit").
    pub image_name: String,

    /// Timeout for container operations in seconds.
    pub timeout_seconds: u64,
}

impl Default for DockerConfig {
    fn default() -> Self {
        Self {
            registry: DEFAULT_REGISTRY.to_string(),
            image_name: DEFAULT_IMAGE_NAME.to_string(),
            timeout_seconds: DEFAULT_TIMEOUT_SECONDS,
        }
    }
}

impl DockerConfig {
    /// Create a new DockerConfig with custom values.
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
        ImageReference {
            registry: self.registry.clone(),
            image_name: self.image_name.clone(),
            tag: format!("v{}.{}.{}", version.major, version.minor, version.patch),
        }
    }
}

/// A fully qualified Docker image reference.
///
/// # Example
///
/// ```rust
/// use adi_docker::{DockerConfig, ImageReference};
/// use semver::Version;
///
/// let config = DockerConfig::default();
/// let version = Version::new(29, 0, 11);
/// let image_ref = config.image_reference(&version);
///
/// assert_eq!(image_ref.full_uri(), "harbor.io/adi/adi-toolkit:v29.0.11");
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
    ///
    /// # Example
    ///
    /// ```rust
    /// use adi_docker::ImageReference;
    ///
    /// let image_ref = ImageReference {
    ///     registry: "harbor.io/adi".to_string(),
    ///     image_name: "adi-toolkit".to_string(),
    ///     tag: "v29.0.11".to_string(),
    /// };
    ///
    /// assert_eq!(image_ref.full_uri(), "harbor.io/adi/adi-toolkit:v29.0.11");
    /// ```
    #[must_use]
    pub fn full_uri(&self) -> String {
        format!("{}/{}:{}", self.registry, self.image_name, self.tag)
    }
}

/// Configuration for creating a container.
#[derive(Debug, Clone)]
pub struct ContainerConfig {
    /// Working directory inside the container.
    pub working_dir: String,

    /// Host directory to mount as /workspace.
    pub state_dir: PathBuf,

    /// Command to execute.
    pub command: Vec<String>,

    /// Environment variables as (key, value) pairs.
    pub env_vars: Vec<(String, String)>,

    /// Use host network mode.
    pub host_network: bool,

    /// Timeout in seconds.
    pub timeout_seconds: u64,
}

impl Default for ContainerConfig {
    fn default() -> Self {
        Self {
            working_dir: "/workspace".to_string(),
            state_dir: PathBuf::new(),
            command: Vec::new(),
            env_vars: Vec::new(),
            host_network: true,
            timeout_seconds: DEFAULT_TIMEOUT_SECONDS,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = DockerConfig::default();
        assert_eq!(config.registry, "harbor.io/adi");
        assert_eq!(config.image_name, "adi-toolkit");
        assert_eq!(config.timeout_seconds, 1800);
    }

    #[test]
    fn test_image_reference_format() {
        let config = DockerConfig::default();
        let version = Version::new(29, 0, 11);
        let image_ref = config.image_reference(&version);

        assert_eq!(image_ref.full_uri(), "harbor.io/adi/adi-toolkit:v29.0.11");
    }

    #[test]
    fn test_custom_config() {
        let config = DockerConfig::new("my-registry.io", "my-toolkit")
            .with_timeout(3600);

        assert_eq!(config.registry, "my-registry.io");
        assert_eq!(config.image_name, "my-toolkit");
        assert_eq!(config.timeout_seconds, 3600);
    }
}
