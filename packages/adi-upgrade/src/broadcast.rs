//! Broadcast phase for upgrade operations.
//!
//! Runs forge script with --broadcast to deploy contracts.

use std::path::Path;

use crate::config::{compute_gas_price, UpgradeConfig};
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
    use secrecy::ExposeSecret;

    log::info!("Running upgrade broadcast");

    let script_path = format!("deploy-scripts/upgrade/{}", handler.upgrade_script());
    let rpc_url = config.l1_rpc_url.to_string();
    let deployer_pk = config.deployer_private_key.expose_secret().to_string();
    let gas_price = config.gas_multiplier.map(compute_gas_price);

    let env_input = format!("/{}/chain.toml", handler.upgrade_env_dir());
    let env_output = format!("/script-out/{}", handler.upgrade_output_toml());

    let env_vars: Vec<(&str, &str)> = vec![
        ("UPGRADE_ECOSYSTEM_INPUT", &env_input),
        ("UPGRADE_ECOSYSTEM_OUTPUT", &env_output),
    ];

    // Build shell command to cd into l1-contracts and run forge with --broadcast
    let mut forge_cmd = format!(
        "forge script {} --ffi --rpc-url {} --private-key {} --broadcast",
        script_path, rpc_url, deployer_pk
    );

    if let Some(price) = gas_price {
        forge_cmd.push_str(&format!(" --with-gas-price {price}"));
    }

    let upgrade_env_dir = handler.upgrade_env_dir();
    let shell_cmd = format!(
        "mkdir -p /deps/era-contracts/l1-contracts/{upgrade_env_dir} && \
         cp -r /workspace/l1-contracts/{upgrade_env_dir}/. /deps/era-contracts/l1-contracts/{upgrade_env_dir}/ && \
         mkdir -p /deps/era-contracts/l1-contracts/script-out && \
         cd /deps/era-contracts/l1-contracts && {forge_cmd} && \
         mkdir -p /workspace/l1-contracts/script-out && \
         cp -r /deps/era-contracts/l1-contracts/script-out/. /workspace/l1-contracts/script-out/ && \
         mkdir -p /workspace/l1-contracts/broadcast && \
         cp -r /deps/era-contracts/l1-contracts/broadcast/. /workspace/l1-contracts/broadcast/"
    );
    let args = vec!["sh", "-c", &shell_cmd];

    let exit_code = runner
        .run_command(&args, state_dir, protocol_version, &env_vars)
        .await
        .map_err(|e| UpgradeError::BroadcastFailed(e.to_string()))?;

    let success = exit_code == 0;

    if !success {
        return Err(UpgradeError::BroadcastFailed(format!(
            "Forge script failed with exit code {}",
            exit_code
        )));
    }

    Ok(BroadcastResult { success, exit_code })
}
