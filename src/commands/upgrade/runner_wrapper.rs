//! Adapter between adi-toolkit's ToolkitRunner and adi-upgrade's ToolkitRunnerTrait.

use std::path::Path;

/// Wrapper to adapt ToolkitRunner to ToolkitRunnerTrait.
pub(super) struct ToolkitRunnerWrapper(pub adi_toolkit::ToolkitRunner);

#[async_trait::async_trait]
impl adi_upgrade::ToolkitRunnerTrait for ToolkitRunnerWrapper {
    async fn run_forge(
        &self,
        args: &[&str],
        state_dir: &Path,
        protocol_version: &semver::Version,
    ) -> std::result::Result<i64, Box<dyn std::error::Error + Send + Sync>> {
        self.0
            .run_forge(args, state_dir, protocol_version)
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
    }

    async fn run_command(
        &self,
        command: &[&str],
        state_dir: &Path,
        protocol_version: &semver::Version,
        env_vars: &[(&str, &str)],
    ) -> std::result::Result<i64, Box<dyn std::error::Error + Send + Sync>> {
        self.0
            .run_command(
                command,
                state_dir,
                protocol_version,
                env_vars,
                "upgrade",
                "Running upgrade...",
            )
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
    }

    async fn run_zkstack(
        &self,
        args: &[&str],
        state_dir: &Path,
        log_dir: &Path,
        protocol_version: &semver::Version,
    ) -> std::result::Result<i64, Box<dyn std::error::Error + Send + Sync>> {
        self.0
            .run_zkstack(args, state_dir, log_dir, protocol_version)
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
    }
}
