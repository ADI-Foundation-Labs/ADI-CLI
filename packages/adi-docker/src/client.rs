//! Docker client wrapper with helper methods.

use crate::error::{DockerError, Result};
use crate::image::ImageManager;
use adi_types::{LogCrateLogger, Logger};
use bollard::Docker;
use std::sync::Arc;

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
/// client.is_daemon_running().await?;
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct DockerClient {
    inner: Docker,
    logger: Arc<dyn Logger>,
}

impl DockerClient {
    /// Create a new DockerClient by connecting to the Docker daemon.
    ///
    /// Uses the default `LogCrateLogger` for logging.
    ///
    /// # Errors
    ///
    /// Returns an error if connection to Docker daemon fails.
    pub async fn new() -> Result<Self> {
        Self::with_logger(Arc::new(LogCrateLogger)).await
    }

    /// Create a new DockerClient with a custom logger.
    ///
    /// # Errors
    ///
    /// Returns an error if connection to Docker daemon fails.
    pub async fn with_logger(logger: Arc<dyn Logger>) -> Result<Self> {
        logger.debug("Connecting to Docker daemon...");
        // Use socket connection with longer timeout for large image pulls
        let docker = Docker::connect_with_socket_defaults()
            .map_err(|e: bollard::errors::Error| DockerError::DaemonNotRunning(e.to_string()))?;

        let client = Self {
            inner: docker,
            logger,
        };

        // Verify connection works
        client.is_daemon_running().await?;

        client
            .logger
            .debug("Successfully connected to Docker daemon");
        Ok(client)
    }

    /// Verify Docker daemon is running and accessible.
    ///
    /// # Errors
    ///
    /// Returns an error if daemon is not accessible.
    pub async fn is_daemon_running(&self) -> Result<()> {
        self.logger.debug("Pinging Docker daemon...");
        self.inner
            .ping()
            .await
            .map_err(|e| DockerError::DaemonNotRunning(e.to_string()))?;
        self.logger.debug("Docker daemon ping successful");
        Ok(())
    }

    /// Pull an image from registry, ensuring the latest version is used.
    ///
    /// Always pulls from the registry to check for updates, even if the
    /// image exists locally. Docker handles layer caching, so this is
    /// fast when the image is already up to date.
    ///
    /// # Arguments
    ///
    /// * `image_uri` - The full image URI (e.g., "registry/image:tag").
    ///
    /// # Errors
    ///
    /// Returns an error if pull fails. Common causes include:
    /// - Registry authentication required (run `docker login` first)
    /// - Network issues
    /// - Image does not exist in registry
    pub async fn pull_image(&self, image_uri: &str) -> Result<()> {
        self.logger
            .debug(&format!("Pulling latest image: {}", image_uri));
        let image_manager = ImageManager::new(self.inner.clone());
        image_manager.pull(image_uri).await
    }

    /// Get a reference to the inner bollard Docker client.
    #[must_use]
    pub fn inner(&self) -> &Docker {
        &self.inner
    }

    /// Get a reference to the logger.
    #[must_use]
    pub fn logger(&self) -> &Arc<dyn Logger> {
        &self.logger
    }
}
