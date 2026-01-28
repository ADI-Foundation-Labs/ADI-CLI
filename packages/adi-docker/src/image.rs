//! Image pulling and management.

use crate::config::ImageReference;
use crate::error::{DockerError, Result};
use bollard::image::CreateImageOptions;
use bollard::Docker;
use futures_util::StreamExt;

/// Manages Docker images (pull, check existence).
pub(crate) struct ImageManager {
    docker: Docker,
}

impl ImageManager {
    /// Create a new ImageManager.
    pub fn new(docker: Docker) -> Self {
        Self { docker }
    }

    /// Check if an image exists locally.
    pub async fn exists(&self, image_ref: &ImageReference) -> Result<bool> {
        let full_uri = image_ref.full_uri();
        match self.docker.inspect_image(&full_uri).await {
            Ok(_) => Ok(true),
            Err(bollard::errors::Error::DockerResponseServerError {
                status_code: 404, ..
            }) => Ok(false),
            Err(e) => Err(DockerError::ConnectionFailed(e)),
        }
    }

    /// Pull an image from registry if not available locally.
    pub async fn pull_if_missing(&self, image_ref: &ImageReference) -> Result<()> {
        if self.exists(image_ref).await? {
            log::debug!("Image {} already exists locally", image_ref.full_uri());
            return Ok(());
        }

        self.pull(image_ref).await
    }

    /// Pull an image from registry.
    async fn pull(&self, image_ref: &ImageReference) -> Result<()> {
        let full_uri = image_ref.full_uri();
        log::info!("Pulling image: {}", full_uri);

        let options = CreateImageOptions {
            from_image: full_uri.clone(),
            ..Default::default()
        };

        let mut stream = self.docker.create_image(Some(options), None, None);

        while let Some(result) = stream.next().await {
            match result {
                Ok(info) => {
                    if let Some(status) = info.status {
                        log::debug!("Pull status: {}", status);
                    }
                    if let Some(error) = info.error {
                        return Err(DockerError::PullFailed {
                            image: full_uri,
                            reason: error,
                        });
                    }
                }
                Err(e) => {
                    return Err(DockerError::PullFailed {
                        image: full_uri,
                        reason: e.to_string(),
                    });
                }
            }
        }

        log::info!("Successfully pulled image: {}", image_ref.full_uri());
        Ok(())
    }
}
