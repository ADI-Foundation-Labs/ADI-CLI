//! Docker client wrapper with helper methods.

use crate::config::ImageReference;
use crate::error::{DockerError, Result};
use crate::image::ImageManager;
use bollard::Docker;

/// Wrapper around bollard::Docker with helper methods.
///
/// Provides a high-level interface for common Docker operations.
///
/// # Example
///
/// ```rust,no_run
/// use adi_docker::DockerClient;
///
/// # async fn example() -> adi_docker::Result<()> {
/// let client = DockerClient::new().await?;
///
/// // Check if Docker daemon is running
/// let is_running = client.is_daemon_running().await?;
/// assert!(is_running);
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct DockerClient {
    inner: Docker,
}

impl DockerClient {
    /// Create a new DockerClient by connecting to the Docker daemon.
    ///
    /// Attempts to connect using the default socket location.
    /// On Unix, this uses the Unix socket at `/var/run/docker.sock`.
    /// On Windows, this uses the named pipe.
    ///
    /// # Errors
    ///
    /// Returns an error if connection to Docker daemon fails.
    pub async fn new() -> Result<Self> {
        let docker = Docker::connect_with_defaults()
            .map_err(|e: bollard::errors::Error| DockerError::DaemonNotRunning(e.to_string()))?;

        let client = Self { inner: docker };

        // Verify connection works
        client.is_daemon_running().await?;

        Ok(client)
    }

    /// Check if Docker daemon is running and accessible.
    ///
    /// # Errors
    ///
    /// Returns an error if daemon is not accessible.
    pub async fn is_daemon_running(&self) -> Result<bool> {
        self.inner
            .ping()
            .await
            .map_err(|e| DockerError::DaemonNotRunning(e.to_string()))?;
        Ok(true)
    }

    /// Check if an image exists locally.
    ///
    /// # Arguments
    ///
    /// * `image_ref` - The image reference to check.
    pub async fn image_exists(&self, image_ref: &ImageReference) -> Result<bool> {
        let image_manager = ImageManager::new(self.inner.clone());
        image_manager.exists(image_ref).await
    }

    /// Pull an image from registry if not available locally.
    ///
    /// # Arguments
    ///
    /// * `image_ref` - The image reference to pull.
    ///
    /// # Errors
    ///
    /// Returns an error if pull fails. Common causes include:
    /// - Registry authentication required (run `docker login` first)
    /// - Network issues
    /// - Image does not exist in registry
    pub async fn pull_image(&self, image_ref: &ImageReference) -> Result<()> {
        let image_manager = ImageManager::new(self.inner.clone());
        image_manager.pull_if_missing(image_ref).await
    }

    /// Get a reference to the inner bollard Docker client.
    #[must_use]
    pub(crate) fn inner(&self) -> &Docker {
        &self.inner
    }
}
