//! Docker client wrapper using Bollard.
//!
// Note: This module is part of the Docker orchestration API (T094-T098).
// It will be used by the toolkit runner and future command implementations.
#![allow(dead_code)]
//!
//! Provides a safe wrapper around the Bollard Docker client with:
//! - Connection to Docker daemon (Unix socket or HTTP)
//! - Daemon availability checking
//! - Version information retrieval
//!
//! # Example
//!
//! ```rust,ignore
//! use adi_cli::docker::DockerClient;
//!
//! #[tokio::main]
//! async fn main() -> eyre::Result<()> {
//!     let client = DockerClient::connect()?;
//!
//!     if client.is_available().await? {
//!         let version = client.version().await?;
//!         println!("Docker version: {}", version.version);
//!     }
//!
//!     Ok(())
//! }
//! ```

use crate::error::{Result, WrapErr};

/// Docker client wrapper providing safe access to Docker daemon.
///
/// This wrapper handles connection management and provides typed methods
/// for common Docker operations needed by the toolkit.
#[derive(Debug, Clone)]
pub struct DockerClient {
    inner: bollard::Docker,
}

/// Docker daemon version information.
#[derive(Debug, Clone)]
pub struct DockerVersion {
    /// Docker version string (e.g., "24.0.7").
    pub version: String,
    /// API version string (e.g., "1.43").
    pub api_version: String,
    /// Operating system (e.g., "linux").
    pub os: String,
    /// Architecture (e.g., "amd64").
    pub arch: String,
}

impl DockerClient {
    /// Connect to the Docker daemon.
    ///
    /// Attempts to connect using the platform-specific default:
    /// - Unix: `/var/run/docker.sock`
    /// - macOS: `/var/run/docker.sock` or `~/.docker/run/docker.sock`
    /// - Windows: Named pipe `//./pipe/docker_engine`
    ///
    /// # Errors
    ///
    /// Returns an error if the Docker daemon is not accessible.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let client = DockerClient::connect()?;
    /// ```
    pub fn connect() -> Result<Self> {
        let docker = bollard::Docker::connect_with_defaults()
            .wrap_err("Failed to connect to Docker daemon")?;

        Ok(Self { inner: docker })
    }

    /// Connect to a Docker daemon at a specific HTTP URL.
    ///
    /// # Arguments
    ///
    /// * `url` - Docker daemon HTTP URL (e.g., "tcp://localhost:2375" or "http://localhost:2375")
    ///
    /// # Errors
    ///
    /// Returns an error if the connection cannot be established.
    #[allow(dead_code)] // May be used for remote Docker daemons
    pub fn connect_with_http(url: &str) -> Result<Self> {
        let docker = bollard::Docker::connect_with_http(url, 120, bollard::API_DEFAULT_VERSION)
            .wrap_err_with(|| format!("Failed to connect to Docker daemon at {url}"))?;

        Ok(Self { inner: docker })
    }

    /// Check if the Docker daemon is available and responding.
    ///
    /// # Returns
    ///
    /// `true` if the daemon responds to a ping, `false` otherwise.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// if client.is_available().await? {
    ///     println!("Docker is running");
    /// }
    /// ```
    pub async fn is_available(&self) -> Result<bool> {
        match self.inner.ping().await {
            Ok(_) => Ok(true),
            Err(bollard::errors::Error::DockerResponseServerError {
                status_code: 500, ..
            }) => Ok(false),
            Err(e) => {
                // Check for connection errors which indicate daemon is not running
                let err_str = e.to_string();
                if err_str.contains("connection refused")
                    || err_str.contains("No such file or directory")
                    || err_str.contains("permission denied")
                {
                    Ok(false)
                } else {
                    Err(e).wrap_err("Failed to ping Docker daemon")
                }
            }
        }
    }

    /// Get Docker daemon version information.
    ///
    /// # Errors
    ///
    /// Returns an error if the version cannot be retrieved.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let version = client.version().await?;
    /// println!("Docker {}", version.version);
    /// ```
    pub async fn version(&self) -> Result<DockerVersion> {
        let version_info = self
            .inner
            .version()
            .await
            .wrap_err("Failed to get Docker version")?;

        Ok(DockerVersion {
            version: version_info
                .version
                .unwrap_or_else(|| "unknown".to_string()),
            api_version: version_info
                .api_version
                .unwrap_or_else(|| "unknown".to_string()),
            os: version_info.os.unwrap_or_else(|| "unknown".to_string()),
            arch: version_info.arch.unwrap_or_else(|| "unknown".to_string()),
        })
    }

    /// Get the inner Bollard client for advanced operations.
    ///
    /// This method provides direct access to the underlying Bollard client
    /// for operations not covered by the wrapper methods.
    pub fn inner(&self) -> &bollard::Docker {
        &self.inner
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_docker_version_debug() {
        let version = DockerVersion {
            version: "24.0.7".to_string(),
            api_version: "1.43".to_string(),
            os: "linux".to_string(),
            arch: "amd64".to_string(),
        };
        let debug = format!("{version:?}");
        assert!(debug.contains("24.0.7"));
        assert!(debug.contains("1.43"));
    }
}
