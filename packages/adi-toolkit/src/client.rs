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
/// use adi_toolkit::DockerClient;
///
/// # async fn example() -> adi_toolkit::Result<()> {
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
        log::debug!("Connecting to Docker daemon...");
        let docker = Docker::connect_with_defaults()
            .map_err(|e: bollard::errors::Error| DockerError::DaemonNotRunning(e.to_string()))?;

        let client = Self { inner: docker };

        // Verify connection works
        client.is_daemon_running().await?;

        log::debug!("Successfully connected to Docker daemon");
        Ok(client)
    }

    /// Check if Docker daemon is running and accessible.
    ///
    /// # Errors
    ///
    /// Returns an error if daemon is not accessible.
    pub async fn is_daemon_running(&self) -> Result<bool> {
        log::debug!("Pinging Docker daemon...");
        self.inner
            .ping()
            .await
            .map_err(|e| DockerError::DaemonNotRunning(e.to_string()))?;
        log::debug!("Docker daemon ping successful");
        Ok(true)
    }

    /// Check if an image exists locally.
    ///
    /// # Arguments
    ///
    /// * `image_ref` - The image reference to check.
    pub async fn image_exists(&self, image_ref: &ImageReference) -> Result<bool> {
        log::debug!("Checking if image exists: {}", image_ref.full_uri());
        let image_manager = ImageManager::new(self.inner.clone());
        let exists = image_manager.exists(image_ref).await?;
        log::debug!("Image {} exists: {}", image_ref.full_uri(), exists);
        Ok(exists)
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
        log::debug!("Ensuring image is available: {}", image_ref.full_uri());
        let image_manager = ImageManager::new(self.inner.clone());
        image_manager.pull_if_missing(image_ref).await
    }

    /// Get a reference to the inner bollard Docker client.
    #[must_use]
    pub(crate) fn inner(&self) -> &Docker {
        &self.inner
    }
}
