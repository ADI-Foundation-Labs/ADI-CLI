//! Container lifecycle management.

use crate::config::ContainerConfig;
use crate::error::{DockerError, Result};
use crate::stream::OutputStreamer;
use adi_types::{LogCrateLogger, Logger};
use bollard::container::{
    Config, CreateContainerOptions, RemoveContainerOptions, StartContainerOptions,
};
use bollard::models::{HostConfig, Mount, MountTypeEnum};
use bollard::Docker;
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

        let result = self.run_and_wait(&container_id, config).await;

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
        let state_dir_absolute = tokio::fs::canonicalize(&config.state_dir)
            .await
            .map_err(|e| {
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

        let state_dir_str = state_dir_absolute
            .to_str()
            .ok_or_else(|| {
                DockerError::ContainerCreateFailed(format!(
                    "State directory path is not valid UTF-8: {}",
                    state_dir_absolute.display()
                ))
            })?
            .to_string();

        let workspace_mount = Mount {
            target: Some("/workspace".to_string()),
            source: Some(state_dir_str),
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
        tokio::fs::create_dir_all(&tmp_dir).await.map_err(|e| {
            DockerError::ContainerCreateFailed(format!(
                "Failed to create tmp directory '{}': {}",
                tmp_dir.display(),
                e
            ))
        })?;

        let tmp_dir_str = tmp_dir
            .to_str()
            .ok_or_else(|| {
                DockerError::ContainerCreateFailed(format!(
                    "Tmp directory path is not valid UTF-8: {}",
                    tmp_dir.display()
                ))
            })?
            .to_string();

        let tmp_mount = Mount {
            target: Some("/tmp".to_string()),
            source: Some(tmp_dir_str),
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
            user: config.user.clone(),
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

    async fn run_and_wait(&self, container_id: &str, config: &ContainerConfig) -> Result<i64> {
        self.logger.debug(&format!(
            "Starting container: {} (timeout: {}s)",
            container_id, config.timeout_seconds
        ));

        self.docker
            .start_container(container_id, None::<StartContainerOptions<String>>)
            .await
            .map_err(|e| DockerError::ContainerCreateFailed(e.to_string()))?;

        self.logger.debug("Container started, streaming output...");

        let streamer = OutputStreamer::new(self.docker.clone(), Arc::clone(&self.logger));
        let duration = Duration::from_secs(config.timeout_seconds);

        // Stream logs with static header and updating log lines
        let stream_result = timeout(
            duration,
            streamer.stream_logs(
                container_id,
                &config.log_dir,
                &config.log_command,
                &config.log_label,
                config.quiet,
            ),
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
                    seconds: config.timeout_seconds,
                })
            }
        }
    }

    async fn wait_for_exit(&self, container_id: &str) -> Result<i64> {
        // Use inspect_container to get exit code from container state
        // This is more reliable than wait_container when the container has already exited
        let inspect = self
            .docker
            .inspect_container(container_id, None)
            .await
            .map_err(|e| DockerError::ContainerFailed {
                exit_code: -1,
                message: format!("Failed to inspect container: {}", e),
            })?;

        // Get exit code from container state
        let exit_code = inspect.state.and_then(|s| s.exit_code).ok_or_else(|| {
            DockerError::ContainerFailed {
                exit_code: -1,
                message: format!("Container {} has no exit code in state", container_id),
            }
        })?;

        Ok(exit_code)
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
        .map_or(0, |d| d.as_nanos());

    format!(
        "adi-docker-{}-{:x}",
        std::process::id(),
        timestamp & 0xFFFF_FFFF
    )
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_container_name_format() {
        let name = generate_container_name();
        assert!(name.starts_with("adi-docker-"));
        // Should contain pid and hex timestamp separated by '-'
        let parts: Vec<&str> = name
            .strip_prefix("adi-docker-")
            .unwrap()
            .splitn(2, '-')
            .collect();
        assert_eq!(parts.len(), 2);
        // First part is pid (decimal)
        parts.first().unwrap().parse::<u32>().unwrap();
        // Second part is hex timestamp
        u64::from_str_radix(parts.get(1).unwrap(), 16).unwrap();
    }

    #[test]
    fn test_generate_container_name_uniqueness() {
        let name1 = generate_container_name();
        std::thread::sleep(std::time::Duration::from_millis(1));
        let name2 = generate_container_name();
        assert_ne!(name1, name2);
    }
}
