//! Real-time output streaming from containers.

use crate::error::{DockerError, Result};
use bollard::container::LogsOptions;
use bollard::Docker;
use futures_util::StreamExt;
use std::io::{self, Write};

/// Streams container output to terminal in real-time.
pub(crate) struct OutputStreamer {
    docker: Docker,
}

impl OutputStreamer {
    /// Create a new OutputStreamer.
    pub fn new(docker: Docker) -> Self {
        Self { docker }
    }

    /// Stream container logs to stdout/stderr in real-time.
    ///
    /// This method attaches to the container's log stream and writes
    /// output to the terminal as it's produced.
    pub async fn stream_logs(&self, container_id: &str) -> Result<()> {
        let options = LogsOptions::<String> {
            follow: true,
            stdout: true,
            stderr: true,
            ..Default::default()
        };

        let mut stream = self.docker.logs(container_id, Some(options));
        let mut stdout = io::stdout();

        while let Some(result) = stream.next().await {
            match result {
                Ok(output) => {
                    let bytes = output.into_bytes();
                    stdout
                        .write_all(&bytes)
                        .map_err(|e| DockerError::StreamError(e.to_string()))?;
                    stdout
                        .flush()
                        .map_err(|e| DockerError::StreamError(e.to_string()))?;
                }
                Err(e) => {
                    // Log but don't fail - container may have exited
                    log::debug!("Log stream ended: {}", e);
                    break;
                }
            }
        }

        Ok(())
    }
}
