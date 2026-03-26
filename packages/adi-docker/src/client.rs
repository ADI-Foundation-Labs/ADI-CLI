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

    /// Check if an image exists locally by full URI.
    ///
    /// # Arguments
    ///
    /// * `image_uri` - The full image URI (e.g., "registry/image:tag").
    pub async fn image_exists(&self, image_uri: &str) -> Result<bool> {
        self.logger
            .debug(&format!("Checking if image exists: {}", image_uri));
        let image_manager = ImageManager::new(self.inner.clone(), Arc::clone(&self.logger));
        let exists = image_manager.exists(image_uri).await?;
        self.logger
            .debug(&format!("Image {} exists: {}", image_uri, exists));
        Ok(exists)
    }

    /// Pull an image from registry if not available locally.
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
            .debug(&format!("Ensuring image is available: {}", image_uri));
        let image_manager = ImageManager::new(self.inner.clone(), Arc::clone(&self.logger));
        image_manager.pull_if_missing(image_uri).await
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
