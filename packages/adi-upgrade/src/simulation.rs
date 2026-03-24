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

    // Gas price in wei from multiplier (None = localhost, skip gas price)
    let gas_price = config.gas_multiplier.map(compute_gas_price);

    // Container env vars for forge script (paths relative to era-contracts/l1-contracts cwd)
    let env_input = format!("/{}/chain.toml", handler.upgrade_env_dir());
    let env_output = format!("/script-out/{}", handler.upgrade_output_toml());

    let env_vars: Vec<(&str, &str)> = vec![
        ("UPGRADE_ECOSYSTEM_INPUT", &env_input),
        ("UPGRADE_ECOSYSTEM_OUTPUT", &env_output),
    ];

    // Build shell command to cd into l1-contracts and run forge
    let mut forge_cmd = format!(
        "forge script {} --ffi --rpc-url {} --private-key {}",
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
         cp -r /deps/era-contracts/l1-contracts/script-out/. /workspace/l1-contracts/script-out/"
    );
    let args = vec!["sh", "-c", &shell_cmd];

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

/// Compute gas price in wei from percentage multiplier.
///
/// Uses a base gas price of 20 gwei and applies the percentage.
/// For example, multiplier 200 (= 2x) with base 20 gwei = 40 gwei.
fn compute_gas_price(multiplier: u64) -> u128 {
    const BASE_GAS_PRICE_GWEI: u128 = 20;
    const GWEI_TO_WEI: u128 = 1_000_000_000;

    BASE_GAS_PRICE_GWEI * GWEI_TO_WEI * u128::from(multiplier) / 100
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
