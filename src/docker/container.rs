//! Docker container lifecycle management.
//!
// Note: This module is part of the Docker orchestration API (T094-T098).
// It will be used by the toolkit runner and future command implementations.
#![allow(dead_code)]
//!
//! Provides functionality for creating, running, and managing ephemeral
//! Docker containers for toolkit operations.
//!
//! # Container Lifecycle
//!
//! ```text
//! create → start → wait for completion → remove
//! ```
//!
//! # Example
//!
//! ```rust,ignore
//! use adi_cli::docker::{DockerClient, ContainerManager, ContainerConfig};
//! use std::path::PathBuf;
//!
//! #[tokio::main]
//! async fn main() -> eyre::Result<()> {
//!     let client = DockerClient::connect()?;
//!     let containers = ContainerManager::new(client);
//!
//!     let config = ContainerConfig::new("alpine:latest")
//!         .with_cmd(vec!["echo", "hello"])
//!         .with_working_dir("/workspace");
//!
//!     let id = containers.create(&config).await?;
//!     containers.start(&id).await?;
//!     let exit_code = containers.wait(&id).await?;
//!     containers.remove(&id).await?;
//!
//!     Ok(())
//! }
//! ```

use crate::error::{Result, WrapErr};
use bollard::container::{
    Config, CreateContainerOptions, RemoveContainerOptions, StartContainerOptions,
    WaitContainerOptions,
};
use bollard::models::{HostConfig, Mount, MountTypeEnum};
use futures_util::StreamExt;
use std::collections::HashMap;
use std::path::PathBuf;

use super::client::DockerClient;

/// Manages Docker container lifecycle.
///
/// Handles creation, starting, waiting, and removal of ephemeral
/// containers used for toolkit operations.
#[derive(Debug, Clone)]
pub struct ContainerManager {
    client: DockerClient,
}

/// Volume mount configuration.
#[derive(Debug, Clone)]
pub struct VolumeMount {
    /// Source path on host.
    pub source: PathBuf,
    /// Target path in container.
    pub target: String,
    /// Mount as read-only.
    pub read_only: bool,
}

impl VolumeMount {
    /// Create a read-write volume mount.
    pub fn new(source: impl Into<PathBuf>, target: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            target: target.into(),
            read_only: false,
        }
    }

    /// Create a read-only volume mount.
    #[allow(dead_code)] // May be used for read-only mounts
    pub fn read_only(source: impl Into<PathBuf>, target: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            target: target.into(),
            read_only: true,
        }
    }
}

/// Environment variable for container.
#[derive(Debug, Clone)]
pub struct EnvVar {
    /// Variable name.
    pub name: String,
    /// Variable value.
    pub value: String,
}

impl EnvVar {
    /// Create a new environment variable.
    pub fn new(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
        }
    }

    /// Format as Docker environment variable string.
    fn to_docker_format(&self) -> String {
        format!("{}={}", self.name, self.value)
    }
}

/// Container configuration for creation.
#[derive(Debug, Clone)]
pub struct ContainerConfig {
    /// Docker image to use.
    pub image: String,
    /// Command to run.
    pub cmd: Option<Vec<String>>,
    /// Working directory inside container.
    pub working_dir: Option<String>,
    /// Volume mounts.
    pub mounts: Vec<VolumeMount>,
    /// Environment variables.
    pub env: Vec<EnvVar>,
    /// Use host network mode.
    pub host_network: bool,
    /// Container name (optional).
    pub name: Option<String>,
    /// Labels for the container.
    pub labels: HashMap<String, String>,
}

impl ContainerConfig {
    /// Create a new container configuration.
    ///
    /// # Arguments
    ///
    /// * `image` - Docker image to use.
    pub fn new(image: impl Into<String>) -> Self {
        Self {
            image: image.into(),
            cmd: None,
            working_dir: None,
            mounts: Vec::new(),
            env: Vec::new(),
            host_network: false,
            name: None,
            labels: HashMap::new(),
        }
    }

    /// Set the command to run.
    pub fn with_cmd(mut self, cmd: Vec<impl Into<String>>) -> Self {
        self.cmd = Some(cmd.into_iter().map(Into::into).collect());
        self
    }

    /// Set the working directory.
    pub fn with_working_dir(mut self, dir: impl Into<String>) -> Self {
        self.working_dir = Some(dir.into());
        self
    }

    /// Add a volume mount.
    pub fn with_mount(mut self, mount: VolumeMount) -> Self {
        self.mounts.push(mount);
        self
    }

    /// Add an environment variable.
    pub fn with_env(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.push(EnvVar::new(name, value));
        self
    }

    /// Enable host network mode.
    ///
    /// This allows the container to access the host's network directly,
    /// which is needed for RPC access to settlement layer nodes.
    pub fn with_host_network(mut self) -> Self {
        self.host_network = true;
        self
    }

    /// Set a container name.
    #[allow(dead_code)] // May be used for named containers
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Add a label to the container.
    pub fn with_label(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.labels.insert(key.into(), value.into());
        self
    }
}

/// Container execution result.
#[derive(Debug, Clone)]
pub struct ContainerResult {
    /// Container ID.
    pub id: String,
    /// Exit code (0 = success).
    pub exit_code: i64,
}

impl ContainerManager {
    /// Create a new container manager.
    ///
    /// # Arguments
    ///
    /// * `client` - Docker client to use.
    pub fn new(client: DockerClient) -> Self {
        Self { client }
    }

    /// Create a container without starting it.
    ///
    /// # Arguments
    ///
    /// * `config` - Container configuration.
    ///
    /// # Returns
    ///
    /// Container ID.
    pub async fn create(&self, config: &ContainerConfig) -> Result<String> {
        // Build mounts
        let mounts: Vec<Mount> = config
            .mounts
            .iter()
            .map(|m| Mount {
                target: Some(m.target.clone()),
                source: Some(m.source.to_string_lossy().to_string()),
                typ: Some(MountTypeEnum::BIND),
                read_only: Some(m.read_only),
                ..Default::default()
            })
            .collect();

        // Build host config
        let host_config = HostConfig {
            mounts: if mounts.is_empty() {
                None
            } else {
                Some(mounts)
            },
            network_mode: if config.host_network {
                Some("host".to_string())
            } else {
                None
            },
            auto_remove: Some(false), // We handle removal manually
            ..Default::default()
        };

        // Build environment variables
        let env: Vec<String> = config.env.iter().map(EnvVar::to_docker_format).collect();

        // Build container config
        let container_config = Config {
            image: Some(config.image.clone()),
            cmd: config.cmd.clone(),
            working_dir: config.working_dir.clone(),
            env: if env.is_empty() { None } else { Some(env) },
            host_config: Some(host_config),
            labels: if config.labels.is_empty() {
                None
            } else {
                Some(config.labels.clone())
            },
            attach_stdout: Some(true),
            attach_stderr: Some(true),
            tty: Some(false),
            ..Default::default()
        };

        // Create container options (with optional name)
        let options = config.name.as_ref().map(|name| CreateContainerOptions {
            name: name.as_str(),
            platform: None,
        });

        let response = self
            .client
            .inner()
            .create_container(options, container_config)
            .await
            .wrap_err_with(|| format!("Failed to create container from image: {}", config.image))?;

        Ok(response.id)
    }

    /// Start a created container.
    ///
    /// # Arguments
    ///
    /// * `id` - Container ID.
    pub async fn start(&self, id: &str) -> Result<()> {
        self.client
            .inner()
            .start_container(id, None::<StartContainerOptions<String>>)
            .await
            .wrap_err_with(|| format!("Failed to start container: {id}"))
    }

    /// Wait for a container to finish.
    ///
    /// # Arguments
    ///
    /// * `id` - Container ID.
    ///
    /// # Returns
    ///
    /// Exit code of the container.
    pub async fn wait(&self, id: &str) -> Result<i64> {
        let options = WaitContainerOptions {
            condition: "not-running",
        };

        let mut stream = self.client.inner().wait_container(id, Some(options));

        // Get the first (and only) result
        if let Some(result) = stream.next().await {
            let response =
                result.wrap_err_with(|| format!("Failed to wait for container: {id}"))?;
            Ok(response.status_code)
        } else {
            Err(eyre::eyre!("Container wait stream ended unexpectedly"))
        }
    }

    /// Remove a container.
    ///
    /// # Arguments
    ///
    /// * `id` - Container ID.
    pub async fn remove(&self, id: &str) -> Result<()> {
        let options = RemoveContainerOptions {
            force: true,
            v: true, // Remove volumes
            ..Default::default()
        };

        self.client
            .inner()
            .remove_container(id, Some(options))
            .await
            .wrap_err_with(|| format!("Failed to remove container: {id}"))
    }

    /// Run a container and wait for it to complete.
    ///
    /// This is a convenience method that:
    /// 1. Creates the container
    /// 2. Starts the container
    /// 3. Waits for completion
    /// 4. Removes the container
    ///
    /// # Arguments
    ///
    /// * `config` - Container configuration.
    ///
    /// # Returns
    ///
    /// Container result with exit code.
    pub async fn run(&self, config: &ContainerConfig) -> Result<ContainerResult> {
        let id = self.create(config).await?;

        // Start container
        if let Err(e) = self.start(&id).await {
            // Clean up on error
            let _ = self.remove(&id).await;
            return Err(e);
        }

        // Wait for completion
        let exit_code = match self.wait(&id).await {
            Ok(code) => code,
            Err(e) => {
                // Clean up on error
                let _ = self.remove(&id).await;
                return Err(e);
            }
        };

        // Remove container
        self.remove(&id).await?;

        Ok(ContainerResult { id, exit_code })
    }

    /// Check if a container exists.
    ///
    /// # Arguments
    ///
    /// * `id` - Container ID.
    #[allow(dead_code)] // May be used for checking container state
    pub async fn exists(&self, id: &str) -> Result<bool> {
        match self.client.inner().inspect_container(id, None).await {
            Ok(_) => Ok(true),
            Err(bollard::errors::Error::DockerResponseServerError {
                status_code: 404, ..
            }) => Ok(false),
            Err(e) => Err(e).wrap_err_with(|| format!("Failed to check container: {id}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_container_config_builder() {
        let config = ContainerConfig::new("alpine:latest")
            .with_cmd(vec!["echo", "hello"])
            .with_working_dir("/workspace")
            .with_env("MY_VAR", "value")
            .with_host_network()
            .with_label("app", "test");

        assert_eq!(config.image, "alpine:latest");
        assert_eq!(
            config.cmd,
            Some(vec!["echo".to_string(), "hello".to_string()])
        );
        assert_eq!(config.working_dir, Some("/workspace".to_string()));
        assert!(config.host_network);
        assert_eq!(config.env.len(), 1);
        assert_eq!(config.env.first().map(|e| e.name.as_str()), Some("MY_VAR"));
        assert_eq!(config.labels.get("app"), Some(&"test".to_string()));
    }

    #[test]
    fn test_volume_mount() {
        let mount = VolumeMount::new("/host/path", "/container/path");
        assert_eq!(mount.source, PathBuf::from("/host/path"));
        assert_eq!(mount.target, "/container/path");
        assert!(!mount.read_only);

        let ro_mount = VolumeMount::read_only("/host/path", "/container/path");
        assert!(ro_mount.read_only);
    }

    #[test]
    fn test_env_var_format() {
        let env = EnvVar::new("MY_VAR", "my_value");
        assert_eq!(env.to_docker_format(), "MY_VAR=my_value");
    }
}
