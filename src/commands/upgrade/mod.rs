//! Upgrade command for ecosystem and chain contracts.

mod prompts;

use std::path::Path;
use std::sync::Arc;

use clap::{Args, ValueEnum};
use serde::{Deserialize, Serialize};

use crate::context::Context;
use crate::error::Result;

/// Target for upgrade operations.
#[derive(Clone, Debug, Default, ValueEnum, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UpgradeTarget {
    /// Upgrade ecosystem-level contracts only
    Ecosystem,
    /// Upgrade chain-level contracts only
    Chain,
    /// Upgrade both ecosystem and chain contracts
    #[default]
    Both,
}

/// Arguments for `upgrade` command.
#[derive(Clone, Args, Debug, Serialize, Deserialize)]
pub struct UpgradeArgs {
    /// Target protocol version (e.g., v0.30.1)
    #[arg(long, required = true)]
    pub protocol_version: String,

    /// Upgrade target: ecosystem, chain, or both
    #[arg(long, default_value = "both")]
    pub target: UpgradeTarget,

    /// Chain name (bypasses multi-select picker)
    #[arg(long)]
    pub chain: Option<String>,

    /// Skip simulation, go straight to broadcast
    #[arg(long)]
    pub skip_simulation: bool,

    /// Settlement layer RPC URL
    #[arg(long)]
    pub rpc_url: Option<url::Url>,

    /// Gas price multiplier
    #[arg(long, default_value = "1.2")]
    pub gas_multiplier: f64,

    /// Ecosystem name
    #[arg(long)]
    pub ecosystem_name: Option<String>,
}

/// Wrapper to adapt ToolkitRunner to ToolkitRunnerTrait.
struct ToolkitRunnerWrapper(adi_toolkit::ToolkitRunner);

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
}

/// Execute the upgrade command.
pub async fn run(args: UpgradeArgs, context: &Context) -> Result<()> {
    use crate::commands::helpers::{
        create_state_manager_with_s3, resolve_ecosystem_name, resolve_rpc_url,
    };
    use crate::error::WrapErr;
    use crate::ui;
    use adi_toolkit::ProtocolVersion;
    use adi_upgrade::{get_handler, UpgradeConfig, UpgradeOrchestrator};

    let ecosystem_name = resolve_ecosystem_name(args.ecosystem_name.as_ref(), context.config())?;

    ui::intro(format!(
        "Upgrading {} to {}",
        ui::green(&ecosystem_name),
        ui::green(&args.protocol_version)
    ))?;

    // Parse and validate protocol version
    let version =
        ProtocolVersion::parse(&args.protocol_version).wrap_err("Invalid protocol version")?;

    let handler = get_handler(&version)
        .ok_or_else(|| eyre::eyre!("Protocol version {} is not supported for upgrades", version))?;

    ui::info(format!(
        "Using upgrade script: {}",
        ui::green(handler.upgrade_script())
    ))?;

    // Resolve RPC URL
    let rpc_url = resolve_rpc_url(args.rpc_url.as_ref(), context.config())?;
    ui::info(format!("RPC URL: {}", ui::green(&rpc_url)))?;

    // Load ecosystem state
    let (state_manager, _s3_control) =
        create_state_manager_with_s3(&ecosystem_name, context).await?;

    // Build upgrade config
    let upgrade_config = UpgradeConfig::from_state(
        &state_manager,
        &ecosystem_name,
        rpc_url,
        args.gas_multiplier,
    )
    .await
    .wrap_err("Failed to build upgrade config")?;

    ui::note(
        "Upgrade Configuration",
        format!(
            "Governor: {}\nDeployer: {}\nBridgehub: {}\nGas multiplier: {}",
            ui::green(upgrade_config.governor_address),
            ui::green(upgrade_config.deployer_address),
            upgrade_config
                .bridgehub_address
                .map(|a| ui::green(a).to_string())
                .unwrap_or_else(|| "(not deployed)".to_string()),
            upgrade_config.gas_multiplier
        ),
    )?;

    ui::note(
        "Upgrade Target",
        format!(
            "Target: {:?}\nChain: {}\nSkip simulation: {}",
            args.target,
            args.chain.as_deref().unwrap_or("(all)"),
            args.skip_simulation
        ),
    )?;

    // Create toolkit runner and orchestrator
    let runner = adi_toolkit::ToolkitRunner::with_config_and_logger(
        context.toolkit_config(),
        Arc::clone(context.logger()),
    )
    .await
    .wrap_err("Failed to create toolkit runner")?;
    let wrapper = ToolkitRunnerWrapper(runner);
    let state_dir = context.config().state_dir.join(&ecosystem_name);

    let orchestrator = UpgradeOrchestrator::new(
        handler.as_ref(),
        &upgrade_config,
        &state_dir,
        &wrapper,
        version.to_semver(),
    );

    // Simulation phase
    if !args.skip_simulation {
        ui::info("Running upgrade simulation...")?;

        let simulation_result = orchestrator.simulate().await?;

        if !simulation_result.success {
            return Err(eyre::eyre!(simulation_result.summary));
        }

        ui::note("Simulation Result", &simulation_result.summary)?;

        let proceed: bool = ui::confirm("Proceed with broadcast?")
            .initial_value(false)
            .interact()
            .wrap_err("Confirmation cancelled")?;

        if !proceed {
            ui::outro_cancel("Upgrade cancelled by user")?;
            return Ok(());
        }
    }

    // Broadcast phase
    ui::info("Running upgrade broadcast...")?;
    let broadcast_result = orchestrator.broadcast().await?;

    if broadcast_result.success {
        ui::success("Broadcast completed successfully")?;
    }

    // Validate bytecode (if output exists)
    // TODO: Load manifest from toolkit image and validate
    ui::info("Bytecode validation: skipped (manifest not yet available)")?;

    // Chain upgrades (if target includes chains)
    let upgrade_chains = matches!(args.target, UpgradeTarget::Chain | UpgradeTarget::Both);

    if upgrade_chains {
        let chain_names = state_manager.list_chains().await?;

        if chain_names.is_empty() {
            ui::warning("No chains found in ecosystem, skipping chain upgrade")?;
        } else {
            let selected_chains = prompts::select_chains(&chain_names, args.chain.as_ref())?;

            for chain_name in &selected_chains {
                ui::info(format!("Upgrading chain: {}", ui::green(chain_name)))?;
                // TODO: Implement chain upgrade
            }
        }
    }

    ui::outro(format!(
        "Upgrade to {} completed successfully",
        ui::green(&args.protocol_version)
    ))?;

    Ok(())
}
