//! Container lifecycle management.

use crate::config::ContainerConfig;
use crate::error::{DockerError, Result};
use crate::stream::OutputStreamer;
use adi_types::{LogCrateLogger, Logger};
use bollard::container::{
    Config, CreateContainerOptions, RemoveContainerOptions, StartContainerOptions,
    WaitContainerOptions,
};
use bollard::models::{HostConfig, Mount, MountTypeEnum};
use bollard::Docker;
use futures_util::StreamExt;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;

/// Manages container lifecycle (create, start, wait, remove).
pub struct ContainerManager {
    docker: Docker,
    logger: Arc<dyn Logger>,
}

impl ContainerManager {
    /// Create a new ContainerManager with default logger.
    pub fn new(docker: Docker) -> Self {
        Self::with_logger(docker, Arc::new(LogCrateLogger))
    }

    /// Create a new ContainerManager with custom logger.
    pub fn with_logger(docker: Docker, logger: Arc<dyn Logger>) -> Self {
        Self { docker, logger }
    }

    /// Run a container to completion and return exit code.
    ///
    /// Lifecycle: create -> start -> stream logs -> wait -> remove
    pub async fn run(&self, image_uri: &str, config: &ContainerConfig) -> Result<i64> {
        self.logger.debug(&format!(
            "Starting container lifecycle for image: {}",
            image_uri
        ));
        let container_id = self.create(image_uri, config).await?;

        let result = self
            .run_and_wait(
                &container_id,
                config.timeout_seconds,
                &config.log_dir,
                &config.log_command,
                &config.log_label,
            )
            .await;

        self.logger.debug("Cleaning up container...");
        if let Err(e) = self.remove(&container_id).await {
            self.logger.warning(&format!(
                "Failed to remove container {}: {}",
                container_id, e
            ));
        }

        result
    }

    async fn create(&self, image_uri: &str, config: &ContainerConfig) -> Result<String> {
        let state_dir_absolute = config.state_dir.canonicalize().map_err(|e| {
            DockerError::ContainerCreateFailed(format!(
                "Failed to resolve state directory '{}' to absolute path: {}",
                config.state_dir.display(),
                e
            ))
        })?;

        self.logger.debug(&format!(
            "Creating container: image={}, working_dir={}, mount={}:/workspace",
            image_uri,
            config.working_dir,
            state_dir_absolute.display()
        ));

        let workspace_mount = Mount {
            target: Some("/workspace".to_string()),
            source: Some(state_dir_absolute.to_string_lossy().to_string()),
            typ: Some(MountTypeEnum::BIND),
            read_only: Some(false),
            ..Default::default()
        };

        let docker_socket_mount = Mount {
            target: Some("/var/run/docker.sock".to_string()),
            source: Some("/var/run/docker.sock".to_string()),
            typ: Some(MountTypeEnum::BIND),
            read_only: Some(false),
            ..Default::default()
        };

        let tmp_dir = state_dir_absolute.join(".tmp");
        std::fs::create_dir_all(&tmp_dir).map_err(|e| {
            DockerError::ContainerCreateFailed(format!(
                "Failed to create tmp directory '{}': {}",
                tmp_dir.display(),
                e
            ))
        })?;

        let tmp_mount = Mount {
            target: Some("/tmp".to_string()),
            source: Some(tmp_dir.to_string_lossy().to_string()),
            typ: Some(MountTypeEnum::BIND),
            read_only: Some(false),
            ..Default::default()
        };

        let host_config = HostConfig {
            mounts: Some(vec![workspace_mount, docker_socket_mount, tmp_mount]),
            network_mode: if config.host_network {
                Some("host".to_string())
            } else {
                None
            },
            auto_remove: Some(false),
            ..Default::default()
        };

        let env_vars: Vec<String> = config
            .env_vars
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect();

        let container_config = Config {
            image: Some(image_uri.to_string()),
            cmd: Some(config.command.clone()),
            entrypoint: Some(vec![]),
            working_dir: Some(config.working_dir.clone()),
            env: Some(env_vars),
            host_config: Some(host_config),
            attach_stdout: Some(true),
            attach_stderr: Some(true),
            tty: Some(true),
            ..Default::default()
        };

        let container_name = generate_container_name();
        let options = CreateContainerOptions {
            name: container_name.clone(),
            platform: None,
        };

        let response = self
            .docker
            .create_container(Some(options), container_config)
            .await
            .map_err(|e| DockerError::ContainerCreateFailed(e.to_string()))?;

        self.logger
            .debug(&format!("Container created: {}", container_name));
        self.logger.debug(&format!("Container ID: {}", response.id));
        Ok(response.id)
    }

    async fn run_and_wait(
        &self,
        container_id: &str,
        timeout_seconds: u64,
        log_dir: &Path,
        log_command: &str,
        log_label: &str,
    ) -> Result<i64> {
        self.logger.debug(&format!(
            "Starting container: {} (timeout: {}s)",
            container_id, timeout_seconds
        ));

        self.docker
            .start_container(container_id, None::<StartContainerOptions<String>>)
            .await
            .map_err(|e| DockerError::ContainerCreateFailed(e.to_string()))?;

        self.logger.debug("Container started, streaming output...");

        let streamer = OutputStreamer::new(self.docker.clone(), Arc::clone(&self.logger));
        let duration = Duration::from_secs(timeout_seconds);

        // Stream logs with static header and updating log lines
        let stream_result = timeout(
            duration,
            streamer.stream_logs(container_id, log_dir, log_command, log_label),
        )
        .await;

        match stream_result {
            Ok(Ok(())) => {
                // Streaming completed normally, get exit code
                match timeout(Duration::from_secs(10), self.wait_for_exit(container_id)).await {
                    Ok(result) => result,
                    Err(_) => Ok(0), // Container already exited, assume success
                }
            }
            Ok(Err(e)) => {
                // Stream was interrupted (CTRL+C)
                self.logger.warning(&format!("Stream interrupted: {}", e));
                self.logger.info("Stopping container...");
                let _ = self.docker.stop_container(container_id, None).await;
                Err(DockerError::StreamError("Interrupted by user".to_string()))
            }
            Err(_) => {
                // Timeout
                let _ = self.docker.stop_container(container_id, None).await;
                Err(DockerError::Timeout {
                    seconds: timeout_seconds,
                })
            }
        }
    }

    async fn wait_for_exit(&self, container_id: &str) -> Result<i64> {
        let mut wait_stream = self
            .docker
            .wait_container(container_id, None::<WaitContainerOptions<String>>);

        match wait_stream.next().await {
            Some(Ok(response)) => Ok(response.status_code),
            Some(Err(e)) => Err(DockerError::ContainerFailed {
                exit_code: -1,
                message: e.to_string(),
            }),
            None => Err(DockerError::ContainerFailed {
                exit_code: -1,
                message: "No wait response received".to_string(),
            }),
        }
    }

    async fn remove(&self, container_id: &str) -> Result<()> {
        let options = RemoveContainerOptions {
            force: true,
            ..Default::default()
        };

        self.docker
            .remove_container(container_id, Some(options))
            .await
            .map_err(DockerError::ConnectionFailed)?;

        self.logger
            .debug(&format!("Removed container: {}", container_id));
        Ok(())
    }
}

fn generate_container_name() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);

    format!("adi-docker-{:x}", timestamp & 0xFFFF_FFFF)
}
