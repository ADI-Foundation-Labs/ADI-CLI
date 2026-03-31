//! Image pulling and management.

use crate::error::{DockerError, Result};
use adi_types::Logger;
use bollard::image::CreateImageOptions;
use bollard::models::CreateImageInfo;
use bollard::Docker;
use cliclack::ProgressBar;
use console::style;
use futures_util::StreamExt;
use std::collections::HashMap;
use std::sync::Arc;

/// Extract a human-readable error reason from a bollard error.
fn pull_error_reason(e: &bollard::errors::Error) -> String {
    match e {
        bollard::errors::Error::DockerStreamError { error } => error.clone(),
        other => other.to_string(),
    }
}

/// Update aggregate layer progress from a single stream event.
fn update_layer_progress(
    info: &CreateImageInfo,
    layer_progress: &mut HashMap<String, (u64, u64)>,
    progress: &ProgressBar,
) {
    let Some(id) = info.id.as_ref() else { return };
    let Some(detail) = info.progress_detail.as_ref() else {
        return;
    };
    let (Some(current), Some(total)) = (detail.current, detail.total) else {
        return;
    };

    let current_bytes = u64::try_from(current).unwrap_or(0);
    let total_bytes = u64::try_from(total).unwrap_or(0);
    layer_progress.insert(id.clone(), (current_bytes, total_bytes));

    let (total_current, total_size): (u64, u64) = layer_progress
        .values()
        .fold((0, 0), |(c, t), (lc, lt)| (c + lc, t + lt));

    if total_size > 0 {
        progress.set_length(total_size);
        progress.set_position(total_current);
    }
}

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
            let info = result.map_err(|e| {
                let reason = pull_error_reason(&e);
                progress.error(&reason);
                DockerError::PullFailed {
                    image: image_uri.to_string(),
                    reason,
                }
            })?;

            if let Some(error) = info.error {
                progress.error(&error);
                return Err(DockerError::PullFailed {
                    image: image_uri.to_string(),
                    reason: error,
                });
            }

            update_layer_progress(&info, &mut layer_progress, &progress);
        }

        progress.stop(format!("Pulled {}", style(short_name).green()));
        Ok(())
    }
}
