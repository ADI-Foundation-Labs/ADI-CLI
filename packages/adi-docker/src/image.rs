//! Image pulling and management.

use crate::error::{DockerError, Result};
use adi_types::Logger;
use bollard::image::CreateImageOptions;
use bollard::Docker;
use console::style;
use futures_util::StreamExt;
use std::collections::HashMap;
use std::sync::Arc;

/// Extract short image name from full URI.
///
/// `harbor.example.com/namespace/image:tag` -> `image:tag`
fn short_image_name(image_uri: &str) -> &str {
    image_uri
        .rsplit_once('/')
        .map_or(image_uri, |(_, name)| name)
}

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
        let options = CreateImageOptions {
            from_image: image_uri.to_string(),
            ..Default::default()
        };

        // Get credentials from Docker's credential store for private registries
        let credentials = crate::auth::get_credentials_for_image(image_uri);

        let mut stream = self.docker.create_image(Some(options), None, credentials);

        // Track per-layer progress: layer_id -> (current_bytes, total_bytes)
        let mut layer_progress: HashMap<String, (u64, u64)> = HashMap::new();
        let progress = cliclack::progress_bar(0).with_download_template();
        let short_name = short_image_name(image_uri);
        progress.start(format!("Pulling {}", style(short_name).green()));

        while let Some(result) = stream.next().await {
            match result {
                Ok(info) => {
                    if let Some(error) = info.error {
                        progress.error(&error);
                        return Err(DockerError::PullFailed {
                            image: image_uri.to_string(),
                            reason: error,
                        });
                    }

                    // Update layer progress if we have progress detail
                    if let (Some(id), Some(detail)) = (info.id, info.progress_detail) {
                        if let (Some(current), Some(total)) = (detail.current, detail.total) {
                            // Only track positive values (negative shouldn't happen but be safe)
                            let current_bytes = u64::try_from(current).unwrap_or(0);
                            let total_bytes = u64::try_from(total).unwrap_or(0);
                            layer_progress.insert(id, (current_bytes, total_bytes));

                            // Sum all layers to get total progress
                            let (total_current, total_size): (u64, u64) = layer_progress
                                .values()
                                .fold((0, 0), |(c, t), (lc, lt)| (c + lc, t + lt));

                            if total_size > 0 {
                                progress.set_length(total_size);
                                progress.set_position(total_current);
                            }
                        }
                    }
                }
                Err(e) => {
                    // Extract the actual error message from DockerStreamError
                    let reason = match &e {
                        bollard::errors::Error::DockerStreamError { error } => error.clone(),
                        other => other.to_string(),
                    };
                    progress.error(&reason);
                    return Err(DockerError::PullFailed {
                        image: image_uri.to_string(),
                        reason,
                    });
                }
            }
        }

        progress.stop(format!("Pulled {}", style(short_name).green()));
        Ok(())
    }
}
