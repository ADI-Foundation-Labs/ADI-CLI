//! Real-time output streaming from containers.

use crate::error::{DockerError, Result};
use bollard::container::LogsOptions;
use bollard::Docker;
use futures_util::StreamExt;
use std::path::Path;
use std::time::Instant;

/// Streams container output with progress spinner.
pub(crate) struct OutputStreamer {
    docker: Docker,
}

impl OutputStreamer {
    /// Create a new OutputStreamer.
    pub fn new(docker: Docker) -> Self {
        Self { docker }
    }

    /// Stream container logs with spinner progress.
    ///
    /// Shows a spinner with elapsed time while streaming.
    /// Full output is saved to a log file.
    pub async fn stream_logs(
        &self,
        container_id: &str,
        log_dir: &Path,
        command: &str,
        label: &str,
    ) -> Result<()> {
        let options = LogsOptions::<String> {
            follow: true,
            stdout: true,
            stderr: true,
            ..Default::default()
        };

        let mut stream = self.docker.logs(container_id, Some(options));

        let mut buffer: Vec<u8> = Vec::new();
        let start = Instant::now();

        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        let log_path = log_dir
            .join("logs")
            .join(format!("{}_{}.log", command, timestamp));

        let spinner = cliclack::spinner();
        spinner.start(label);

        let stream_result: Result<()> = loop {
            tokio::select! {
                _ = tokio::signal::ctrl_c() => {
                    spinner.stop("Interrupted");
                    Self::save_log(&buffer, &log_path)?;
                    break Err(DockerError::StreamError("Interrupted by CTRL+C".to_string()));
                }

                result = stream.next() => {
                    match result {
                        Some(Ok(output)) => {
                            buffer.extend(output.into_bytes());
                            spinner.set_message(format!("[{}s]", start.elapsed().as_secs()));
                        }
                        Some(Err(e)) => {
                            log::debug!("Log stream ended: {}", e);
                            break Ok(());
                        }
                        None => {
                            break Ok(());
                        }
                    }
                }
            }
        };

        // Normal completion - save log
        if stream_result.is_ok() {
            spinner.stop(format!("Completed in {}s", start.elapsed().as_secs()));
            Self::save_log(&buffer, &log_path)?;
        }

        stream_result
    }

    fn save_log(buffer: &[u8], log_path: &Path) -> Result<()> {
        if let Some(parent) = log_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                DockerError::StreamError(format!("Failed to create log dir: {}", e))
            })?;
        }
        std::fs::write(log_path, buffer)
            .map_err(|e| DockerError::StreamError(format!("Failed to write log: {}", e)))?;
        cliclack::log::info(format!("Full output saved to: {}", log_path.display()))
            .map_err(|e| DockerError::StreamError(format!("Failed to log: {}", e)))?;
        Ok(())
    }
}
