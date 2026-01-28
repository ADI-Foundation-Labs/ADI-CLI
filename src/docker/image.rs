//! Docker image management.
//!
// Note: This module is part of the Docker orchestration API (T094-T098).
// It will be used by the toolkit runner and future command implementations.
#![allow(dead_code)]
//!
//! Provides functionality for pulling, checking, and managing Docker images
//! needed for toolkit operations.
//!
//! # Example
//!
//! ```rust,ignore
//! use adi_cli::docker::{DockerClient, ImageManager};
//!
//! #[tokio::main]
//! async fn main() -> eyre::Result<()> {
//!     let client = DockerClient::connect()?;
//!     let images = ImageManager::new(client);
//!
//!     // Check if image exists
//!     if !images.exists("harbor.io/adi/adi-toolkit:v29.0.11").await? {
//!         // Pull with progress callback
//!         images.pull("harbor.io/adi/adi-toolkit:v29.0.11", |progress| {
//!             println!("{}", progress);
//!         }).await?;
//!     }
//!
//!     Ok(())
//! }
//! ```

use crate::error::{Result, WrapErr};
use bollard::image::{CreateImageOptions, ListImagesOptions};
use futures_util::StreamExt;
use std::collections::HashMap;

use super::client::DockerClient;

/// Manages Docker images for the toolkit.
///
/// Provides methods for pulling, checking existence, and removing
/// Docker images needed for toolkit operations.
#[derive(Debug, Clone)]
pub struct ImageManager {
    client: DockerClient,
}

/// Progress information during image pull.
#[derive(Debug, Clone)]
pub struct PullProgress {
    /// Current status message.
    pub status: String,
    /// Progress details (e.g., layer ID).
    pub id: Option<String>,
    /// Current progress value (bytes downloaded).
    pub current: Option<u64>,
    /// Total progress value (total bytes).
    pub total: Option<u64>,
}

impl std::fmt::Display for PullProgress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match (&self.id, self.current, self.total) {
            (Some(id), Some(current), Some(total)) if total > 0 => {
                let percent = (current as f64 / total as f64) * 100.0;
                write!(f, "{}: {} ({:.1}%)", id, self.status, percent)
            }
            (Some(id), _, _) => write!(f, "{}: {}", id, self.status),
            (None, _, _) => write!(f, "{}", self.status),
        }
    }
}

/// Information about a Docker image.
#[derive(Debug, Clone)]
pub struct ImageInfo {
    /// Image ID.
    pub id: String,
    /// Image tags (e.g., ["harbor.io/adi/adi-toolkit:v29.0.11"]).
    pub tags: Vec<String>,
    /// Image size in bytes.
    pub size: u64,
    /// Image creation timestamp (Unix epoch).
    pub created: i64,
}

impl ImageManager {
    /// Create a new image manager.
    ///
    /// # Arguments
    ///
    /// * `client` - Docker client to use for operations.
    pub fn new(client: DockerClient) -> Self {
        Self { client }
    }

    /// Check if an image exists locally.
    ///
    /// # Arguments
    ///
    /// * `image` - Image reference (e.g., "harbor.io/adi/adi-toolkit:v29.0.11").
    ///
    /// # Returns
    ///
    /// `true` if the image exists locally, `false` otherwise.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// if !images.exists("myimage:latest").await? {
    ///     images.pull("myimage:latest", |_| {}).await?;
    /// }
    /// ```
    pub async fn exists(&self, image: &str) -> Result<bool> {
        let filters: HashMap<String, Vec<String>> =
            [("reference".to_string(), vec![image.to_string()])]
                .into_iter()
                .collect();

        let options = ListImagesOptions {
            filters,
            ..Default::default()
        };

        let images = self
            .client
            .inner()
            .list_images(Some(options))
            .await
            .wrap_err_with(|| format!("Failed to check if image exists: {image}"))?;

        Ok(!images.is_empty())
    }

    /// Pull an image from a registry.
    ///
    /// # Arguments
    ///
    /// * `image` - Image reference to pull (e.g., "harbor.io/adi/adi-toolkit:v29.0.11").
    /// * `progress_callback` - Callback function receiving progress updates.
    ///
    /// # Errors
    ///
    /// Returns an error if the image cannot be pulled.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// images.pull("harbor.io/adi/adi-toolkit:v29.0.11", |progress| {
    ///     log::info!("{}", progress);
    /// }).await?;
    /// ```
    pub async fn pull<F>(&self, image: &str, progress_callback: F) -> Result<()>
    where
        F: Fn(PullProgress),
    {
        let options = CreateImageOptions {
            from_image: image,
            ..Default::default()
        };

        let mut stream = self.client.inner().create_image(Some(options), None, None);

        while let Some(result) = stream.next().await {
            match result {
                Ok(info) => {
                    // Convert i64 progress values to u64, treating negative as 0
                    let current = info
                        .progress_detail
                        .as_ref()
                        .and_then(|p| p.current)
                        .and_then(|v| u64::try_from(v).ok());
                    let total = info
                        .progress_detail
                        .and_then(|p| p.total)
                        .and_then(|v| u64::try_from(v).ok());

                    let progress = PullProgress {
                        status: info.status.unwrap_or_else(|| "Unknown".to_string()),
                        id: info.id,
                        current,
                        total,
                    };
                    progress_callback(progress);
                }
                Err(e) => {
                    return Err(e).wrap_err_with(|| format!("Failed to pull image: {image}"));
                }
            }
        }

        Ok(())
    }

    /// Pull an image if it doesn't exist locally.
    ///
    /// # Arguments
    ///
    /// * `image` - Image reference to pull.
    /// * `progress_callback` - Callback function receiving progress updates.
    ///
    /// # Returns
    ///
    /// `true` if the image was pulled, `false` if it already existed.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let was_pulled = images.ensure("myimage:latest", |p| println!("{}", p)).await?;
    /// ```
    pub async fn ensure<F>(&self, image: &str, progress_callback: F) -> Result<bool>
    where
        F: Fn(PullProgress),
    {
        if self.exists(image).await? {
            return Ok(false);
        }

        self.pull(image, progress_callback).await?;
        Ok(true)
    }

    /// Get information about a local image.
    ///
    /// # Arguments
    ///
    /// * `image` - Image reference to inspect.
    ///
    /// # Returns
    ///
    /// `Some(ImageInfo)` if the image exists, `None` otherwise.
    #[allow(dead_code)] // May be used for debugging/info display
    pub async fn inspect(&self, image: &str) -> Result<Option<ImageInfo>> {
        match self.client.inner().inspect_image(image).await {
            Ok(info) => {
                // Handle potential negative size values by using try_from
                let size = info.size.and_then(|s| u64::try_from(s).ok()).unwrap_or(0);

                Ok(Some(ImageInfo {
                    id: info.id.unwrap_or_else(|| "unknown".to_string()),
                    tags: info.repo_tags.unwrap_or_default(),
                    size,
                    created: info
                        .created
                        .and_then(|c| chrono::DateTime::parse_from_rfc3339(&c).ok())
                        .map(|dt| dt.timestamp())
                        .unwrap_or(0),
                }))
            }
            Err(bollard::errors::Error::DockerResponseServerError {
                status_code: 404, ..
            }) => Ok(None),
            Err(e) => Err(e).wrap_err_with(|| format!("Failed to inspect image: {image}")),
        }
    }

    /// Remove a local image.
    ///
    /// # Arguments
    ///
    /// * `image` - Image reference to remove.
    /// * `force` - Force removal even if containers are using the image.
    ///
    /// # Errors
    ///
    /// Returns an error if the image cannot be removed.
    #[allow(dead_code)] // May be used for cleanup operations
    pub async fn remove(&self, image: &str, force: bool) -> Result<()> {
        use bollard::image::RemoveImageOptions;

        let options = RemoveImageOptions {
            force,
            ..Default::default()
        };

        self.client
            .inner()
            .remove_image(image, Some(options), None)
            .await
            .wrap_err_with(|| format!("Failed to remove image: {image}"))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pull_progress_display() {
        let progress = PullProgress {
            status: "Downloading".to_string(),
            id: Some("abc123".to_string()),
            current: Some(50),
            total: Some(100),
        };
        let display = format!("{progress}");
        assert!(display.contains("abc123"));
        assert!(display.contains("Downloading"));
        assert!(display.contains("50.0%"));
    }

    #[test]
    fn test_pull_progress_display_no_total() {
        let progress = PullProgress {
            status: "Pulling".to_string(),
            id: Some("abc123".to_string()),
            current: None,
            total: None,
        };
        let display = format!("{progress}");
        assert_eq!(display, "abc123: Pulling");
    }

    #[test]
    fn test_pull_progress_display_no_id() {
        let progress = PullProgress {
            status: "Waiting".to_string(),
            id: None,
            current: None,
            total: None,
        };
        let display = format!("{progress}");
        assert_eq!(display, "Waiting");
    }
}
