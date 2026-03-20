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
    use secrecy::ExposeSecret;

    log::info!("Running upgrade simulation");

    let script_path = format!("deploy-scripts/upgrade/{}", handler.upgrade_script());
    let rpc_url = config.l1_rpc_url.to_string();
    let deployer_pk = config.deployer_private_key.expose_secret().to_string();

    // Gas price in wei from multiplier (default ~20 gwei * multiplier)
    let gas_price = format!("{}", compute_gas_price(config.gas_multiplier));

    // Container env vars for forge script
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
    ];

    // Only add gas price if non-zero
    if config.gas_multiplier > 0.0 {
        args.push("--with-gas-price");
        args.push(&gas_price);
    }

    let exit_code = runner
        .run_command(&args, state_dir, protocol_version, &env_vars)
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
        summary,
    })
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
