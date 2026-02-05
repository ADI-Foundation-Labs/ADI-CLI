//! Container lifecycle management.

use crate::config::ContainerConfig;
use crate::error::{DockerError, Result};
use crate::stream::OutputStreamer;
use bollard::container::{
    Config, CreateContainerOptions, RemoveContainerOptions, StartContainerOptions,
    WaitContainerOptions,
};
use bollard::models::{HostConfig, Mount, MountTypeEnum};
use bollard::Docker;
use futures_util::StreamExt;
use std::time::Duration;
use tokio::time::timeout;

/// Manages container lifecycle (create, start, wait, remove).
pub struct ContainerManager {
    docker: Docker,
}

impl ContainerManager {
    /// Create a new ContainerManager.
    pub fn new(docker: Docker) -> Self {
        Self { docker }
    }

    /// Run a container to completion and return exit code.
    ///
    /// Lifecycle: create -> start -> attach streams -> wait -> remove
    pub async fn run(&self, image_uri: &str, config: &ContainerConfig) -> Result<i64> {
        log::debug!("Starting container lifecycle for image: {}", image_uri);
        let container_id = self.create(image_uri, config).await?;

        // Ensure cleanup on any exit path
        let result = self
            .run_and_wait(&container_id, config.timeout_seconds)
            .await;

        // Always attempt to remove, even if run failed
        log::info!("Cleaning up container...");
        if let Err(e) = self.remove(&container_id).await {
            log::warn!("Failed to remove container {}: {}", container_id, e);
        }

        result
    }

    async fn create(&self, image_uri: &str, config: &ContainerConfig) -> Result<String> {
        // Docker requires absolute paths for bind mounts
        let state_dir_absolute = config.state_dir.canonicalize().map_err(|e| {
            DockerError::ContainerCreateFailed(format!(
                "Failed to resolve state directory '{}' to absolute path: {}",
                config.state_dir.display(),
                e
            ))
        })?;

        log::debug!(
            "Creating container: image={}, working_dir={}, mount={}:/workspace",
            image_uri,
            config.working_dir,
            state_dir_absolute.display()
        );

        let workspace_mount = Mount {
            target: Some("/workspace".to_string()),
            source: Some(state_dir_absolute.to_string_lossy().to_string()),
            typ: Some(MountTypeEnum::BIND),
            read_only: Some(false),
            ..Default::default()
        };

        // Mount Docker socket to allow container to communicate with host Docker daemon
        let docker_socket_mount = Mount {
            target: Some("/var/run/docker.sock".to_string()),
            source: Some("/var/run/docker.sock".to_string()),
            typ: Some(MountTypeEnum::BIND),
            read_only: Some(false),
            ..Default::default()
        };

        let host_config = HostConfig {
            mounts: Some(vec![workspace_mount, docker_socket_mount]),
            network_mode: if config.host_network {
                Some("host".to_string())
            } else {
                None
            },
            auto_remove: Some(false), // We remove manually for better control
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
            entrypoint: Some(vec![]), // Clear image entrypoint to run command directly
            working_dir: Some(config.working_dir.clone()),
            env: Some(env_vars),
            host_config: Some(host_config),
            attach_stdout: Some(true),
            attach_stderr: Some(true),
            tty: Some(false),
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

        log::info!("Container created: {}", container_name);
        log::debug!("Container ID: {}", response.id);
        Ok(response.id)
    }

    async fn run_and_wait(&self, container_id: &str, timeout_seconds: u64) -> Result<i64> {
        log::debug!(
            "Starting container: {} (timeout: {}s)",
            container_id,
            timeout_seconds
        );

        // Start container
        self.docker
            .start_container(container_id, None::<StartContainerOptions<String>>)
            .await
            .map_err(|e| DockerError::ContainerCreateFailed(e.to_string()))?;

        log::info!("Container started, streaming output...");

        // Stream output to terminal
        let streamer = OutputStreamer::new(self.docker.clone());

        // Run streaming and waiting with timeout
        let duration = Duration::from_secs(timeout_seconds);

        let stream_future = streamer.stream_logs(container_id);
        let wait_future = self.wait_for_exit(container_id);

        // Run streaming in background, wait for exit with timeout
        let (stream_result, wait_result) =
            tokio::join!(stream_future, timeout(duration, wait_future));

        // Check for timeout
        let exit_code = match wait_result {
            Ok(result) => result?,
            Err(_) => {
                // Timeout occurred, try to stop the container
                let _ = self.docker.stop_container(container_id, None).await;
                return Err(DockerError::Timeout {
                    seconds: timeout_seconds,
                });
            }
        };

        // Log any stream errors but don't fail
        if let Err(e) = stream_result {
            log::debug!("Stream ended with error (may be normal): {}", e);
        }

        Ok(exit_code)
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

        log::debug!("Removed container: {}", container_id);
        Ok(())
    }
}

/// Generate a unique container name.
fn generate_container_name() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);

    format!("adi-docker-{:x}", timestamp & 0xFFFF_FFFF)
}
