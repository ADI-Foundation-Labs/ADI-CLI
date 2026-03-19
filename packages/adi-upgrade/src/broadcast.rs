//! Broadcast phase for upgrade operations.
//!
//! Runs forge script with --broadcast to deploy contracts.

use std::path::Path;

use crate::config::UpgradeConfig;
use crate::error::{Result, UpgradeError};
use crate::simulation::ToolkitRunnerTrait;
use crate::versions::VersionHandler;

/// Result of a broadcast run.
#[derive(Debug)]
pub struct BroadcastResult {
    /// Whether broadcast succeeded
    pub success: bool,

    /// Exit code from forge
    pub exit_code: i64,

    /// Path to output YAML file
    pub output_path: Option<std::path::PathBuf>,
}

/// Run upgrade broadcast (forge script with --broadcast).
///
/// # Arguments
///
/// * `handler` - Version-specific handler
/// * `config` - Upgrade configuration
/// * `state_dir` - Ecosystem state directory
/// * `runner` - Toolkit runner for Docker execution
/// * `protocol_version` - Protocol version for image selection
pub async fn run_broadcast<R>(
    handler: &dyn VersionHandler,
    config: &UpgradeConfig,
    state_dir: &Path,
    runner: &R,
    protocol_version: &semver::Version,
) -> Result<BroadcastResult>
where
    R: ToolkitRunnerTrait,
{
    log::info!("Running upgrade broadcast");
    log::debug!("Upgrade script: {}", handler.upgrade_script());

    let script_path = handler.upgrade_script();

    // Build forge command args with --broadcast
    let rpc_url = config.l1_rpc_url.to_string();
    let args = vec![
        "script",
        script_path,
        "--rpc-url",
        &rpc_url,
        "--broadcast",
        "-vvv",
    ];

    let exit_code = runner
        .run_forge(&args, state_dir, protocol_version)
        .await
        .map_err(|e| UpgradeError::BroadcastFailed(e.to_string()))?;

    let success = exit_code == 0;

    if !success {
        return Err(UpgradeError::BroadcastFailed(format!(
            "Forge script failed with exit code {}",
            exit_code
        )));
    }

    Ok(BroadcastResult {
        success,
        exit_code,
        output_path: None, // TODO: parse output path
    })
}
