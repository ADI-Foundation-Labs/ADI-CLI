//! Image pulling and management.

use crate::error::{DockerError, Result};
use adi_types::Logger;
use bollard::image::CreateImageOptions;
use bollard::Docker;
use futures_util::StreamExt;
use std::sync::Arc;

/// Manages Docker images (pull, check existence).
pub(crate) struct ImageManager {
    docker: Docker,
    logger: Arc<dyn Logger>,
}

impl ImageManager {
    /// Create a new ImageManager.
    pub fn new(docker: Docker, logger: Arc<dyn Logger>) -> Self {
        Self { docker, logger }
    }

    /// Check if an image exists locally.
    pub async fn exists(&self, image_uri: &str) -> Result<bool> {
        match self.docker.inspect_image(image_uri).await {
            Ok(_) => Ok(true),
            Err(bollard::errors::Error::DockerResponseServerError {
                status_code: 404, ..
            }) => Ok(false),
            Err(e) => Err(DockerError::ConnectionFailed(e)),
        }
    }

    /// Pull an image from registry if not available locally.
    pub async fn pull_if_missing(&self, image_uri: &str) -> Result<()> {
        if self.exists(image_uri).await? {
            self.logger
                .debug(&format!("Image {} already exists locally", image_uri));
            return Ok(());
        }

        self.pull(image_uri).await
    }

    /// Pull an image from registry.
    async fn pull(&self, image_uri: &str) -> Result<()> {
        self.logger.info(&format!("Pulling image: {}", image_uri));

        let options = CreateImageOptions {
            from_image: image_uri.to_string(),
            ..Default::default()
        };

        let mut stream = self.docker.create_image(Some(options), None, None);

        while let Some(result) = stream.next().await {
            match result {
                Ok(info) => {
                    if let Some(status) = info.status {
                        self.logger.debug(&format!("Pull status: {}", status));
                    }
                    if let Some(error) = info.error {
                        return Err(DockerError::PullFailed {
                            image: image_uri.to_string(),
                            reason: error,
                        });
                    }
                }
                Err(e) => {
                    return Err(DockerError::PullFailed {
                        image: image_uri.to_string(),
                        reason: e.to_string(),
                    });
                }
            }
        }

        self.logger.success(&format!("Pulled image: {}", image_uri));
        Ok(())
    }
}
