//! Simulation phase for upgrade operations.
//!
//! Runs forge script without --broadcast to validate upgrade.

use std::path::Path;

use crate::config::UpgradeConfig;
use crate::error::{Result, UpgradeError};
use crate::versions::VersionHandler;

/// Result of a simulation run.
#[derive(Debug)]
pub struct SimulationResult {
    /// Whether simulation succeeded
    pub success: bool,

    /// Exit code from forge
    pub exit_code: i64,

    /// Path to output YAML file
    pub output_path: Option<std::path::PathBuf>,

    /// Summary of what will be deployed
    pub summary: String,
}

/// Run upgrade simulation (forge script without --broadcast).
///
/// # Arguments
///
/// * `handler` - Version-specific handler
/// * `config` - Upgrade configuration
/// * `state_dir` - Ecosystem state directory
/// * `runner` - Toolkit runner for Docker execution
/// * `protocol_version` - Protocol version for image selection
pub async fn run_simulation<R>(
    handler: &dyn VersionHandler,
    config: &UpgradeConfig,
    state_dir: &Path,
    runner: &R,
    protocol_version: &semver::Version,
) -> Result<SimulationResult>
where
    R: ToolkitRunnerTrait,
{
    log::info!("Running upgrade simulation");
    log::debug!("Upgrade script: {}", handler.upgrade_script());

    let script_path = handler.upgrade_script();

    // Build forge command args
    let rpc_url = config.l1_rpc_url.to_string();
    let args = vec!["script", script_path, "--rpc-url", &rpc_url, "-vvv"];

    let exit_code = runner
        .run_forge(&args, state_dir, protocol_version)
        .await
        .map_err(|e| UpgradeError::SimulationFailed(e.to_string()))?;

    let success = exit_code == 0;

    let summary = if success {
        "Simulation completed successfully. Review the output above.".to_string()
    } else {
        format!("Simulation failed with exit code {}", exit_code)
    };

    Ok(SimulationResult {
        success,
        exit_code,
        output_path: None, // TODO: parse output path
        summary,
    })
}

/// Trait for toolkit runner to enable testing.
#[async_trait::async_trait]
pub trait ToolkitRunnerTrait: Send + Sync {
    /// Run forge command.
    async fn run_forge(
        &self,
        args: &[&str],
        state_dir: &Path,
        protocol_version: &semver::Version,
    ) -> std::result::Result<i64, Box<dyn std::error::Error + Send + Sync>>;
}
