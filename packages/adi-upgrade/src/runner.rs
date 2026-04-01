//! Trait for toolkit container operations.
//!
//! Defines the interface for running forge, zkstack, and arbitrary
//! commands inside toolkit Docker containers.

use std::path::Path;

/// Trait for toolkit runner to enable testing.
#[async_trait::async_trait]
pub trait ToolkitRunnerTrait: Send + Sync {
    /// Run forge command in toolkit container.
    async fn run_forge(
        &self,
        args: &[&str],
        state_dir: &Path,
        protocol_version: &semver::Version,
    ) -> std::result::Result<i64, Box<dyn std::error::Error + Send + Sync>>;

    /// Run arbitrary command with env vars in toolkit container.
    async fn run_command(
        &self,
        command: &[&str],
        state_dir: &Path,
        protocol_version: &semver::Version,
        env_vars: &[(&str, &str)],
    ) -> std::result::Result<i64, Box<dyn std::error::Error + Send + Sync>>;

    /// Run zkstack command in toolkit container.
    async fn run_zkstack(
        &self,
        args: &[&str],
        state_dir: &Path,
        log_dir: &Path,
        protocol_version: &semver::Version,
    ) -> std::result::Result<i64, Box<dyn std::error::Error + Send + Sync>>;
}
