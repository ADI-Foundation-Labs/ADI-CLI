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
    let gas_price = format!("{}", compute_gas_price(config.gas_multiplier));

    let env_input = format!("/{}/chain.toml", handler.upgrade_env_dir());
    let env_output = format!("/script-out/{}", handler.upgrade_output_toml());

    let env_vars: Vec<(&str, &str)> = vec![
        ("UPGRADE_ECOSYSTEM_INPUT", &env_input),
        ("UPGRADE_ECOSYSTEM_OUTPUT", &env_output),
    ];

    let mut args = vec![
        "forge",
        "script",
        &script_path,
        "--ffi",
        "--rpc-url",
        &rpc_url,
        "--private-key",
        &deployer_pk,
        "--broadcast",
    ];

    if config.gas_multiplier > 0.0 {
        args.push("--with-gas-price");
        args.push(&gas_price);
    }

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

/// Compute gas price in wei from multiplier.
///
/// Uses a base gas price and applies the multiplier.
/// For example, multiplier 1.2 with base 20 gwei = 24 gwei.
fn compute_gas_price(multiplier: f64) -> u128 {
    const BASE_GAS_PRICE_GWEI: u128 = 20;
    const GWEI_TO_WEI: u128 = 1_000_000_000;

    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let price = (BASE_GAS_PRICE_GWEI as f64 * multiplier * GWEI_TO_WEI as f64) as u128;
    price
}
