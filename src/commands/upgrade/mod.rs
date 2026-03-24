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
    #[arg(long)]
    pub protocol_version: Option<String>,

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

    /// Ecosystem name
    #[arg(long)]
    pub ecosystem_name: Option<String>,

    /// Path to previous upgrade YAML (for [state_transition] values)
    #[arg(long)]
    pub previous_upgrade_yaml: Option<std::path::PathBuf>,
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

/// Execute the upgrade command.
pub async fn run(args: UpgradeArgs, context: &Context) -> Result<()> {
    use crate::commands::helpers::{
        create_state_manager_with_s3, resolve_ecosystem_name, resolve_rpc_url,
    };
    use crate::error::WrapErr;
    use crate::ui;
    use adi_toolkit::ProtocolVersion;
    use adi_upgrade::{
        get_handler, load_previous_upgrade_values, onchain, UpgradeConfig, UpgradeOrchestrator,
    };
    use alloy_provider::Provider;

    let ecosystem_name = resolve_ecosystem_name(args.ecosystem_name.as_ref(), context.config())?;

    // Resolve protocol version from arg, config, or interactive picker
    let protocol_version_str = match args.protocol_version.as_ref() {
        Some(v) => v.clone(),
        None => {
            if let Some(v) = context
                .config()
                .protocol_version
                .as_ref()
                .filter(|s| !s.is_empty())
            {
                v.clone()
            } else {
                use strum::IntoEnumIterator;
                let versions: Vec<_> = ProtocolVersion::iter().collect();
                match versions.len() {
                    0 => return Err(eyre::eyre!("No supported protocol versions available")),
                    1 => {
                        let v = versions.first().ok_or_else(|| eyre::eyre!("No versions"))?;
                        ui::info(format!("Auto-selected version: {}", ui::green(v)))?;
                        v.to_string()
                    }
                    _ => {
                        let items: Vec<(String, String, String)> = versions
                            .iter()
                            .map(|v: &ProtocolVersion| {
                                (v.to_string(), v.to_string(), String::new())
                            })
                            .collect();
                        ui::select("Select protocol version")
                            .items(&items)
                            .interact()
                            .wrap_err("Version selection cancelled")?
                    }
                }
            }
        }
    };

    let version =
        ProtocolVersion::parse(&protocol_version_str).wrap_err("Invalid protocol version")?;

    let handler = get_handler(&version)
        .ok_or_else(|| eyre::eyre!("Protocol version {} is not supported for upgrades", version))?;

    ui::intro(format!(
        "Upgrading {} to {}",
        ui::green(&ecosystem_name),
        ui::green(version)
    ))?;

    ui::info(format!(
        "Using upgrade script: {}",
        ui::green(handler.upgrade_script())
    ))?;

    // Resolve RPC URL
    let rpc_url = resolve_rpc_url(args.rpc_url.as_ref(), context.config())?;
    // Normalize for host-side on-chain queries (host.docker.internal → localhost)
    let normalized_rpc = adi_types::normalize_rpc_url(rpc_url.as_str());
    let normalized_url: url::Url = normalized_rpc
        .parse()
        .wrap_err("Failed to parse normalized RPC URL")?;
    ui::info(format!("RPC URL: {}", ui::green(&rpc_url)))?;

    // Load ecosystem state
    let (state_manager, _s3_control) =
        create_state_manager_with_s3(&ecosystem_name, context).await?;

    // Validate state paths
    let state_dir = context.config().state_dir.join(&ecosystem_name);
    crate::commands::state_paths::validate_and_fix_state_paths(&state_manager, &state_dir).await?;

    // Build upgrade config

    // Skip gas price for localhost (anvil), use config multiplier otherwise
    let gas_multiplier = if adi_types::is_localhost_rpc(rpc_url.as_str()) {
        None
    } else {
        Some(context.config().gas_multiplier)
    };

    let upgrade_config = UpgradeConfig::from_state(
        &state_manager,
        &ecosystem_name,
        rpc_url.clone(),
        gas_multiplier,
        state_dir.clone(),
    )
    .await
    .wrap_err("Failed to build upgrade config")?;

    ui::note(
        "Upgrade Configuration",
        format!(
            "Governor: {}\nDeployer: {}\nBridgehub: {}\nGas multiplier: {}",
            ui::green(upgrade_config.governor_address),
            ui::green(upgrade_config.deployer_address),
            ui::green(upgrade_config.bridgehub_address),
            upgrade_config
                .gas_multiplier
                .map_or("disabled (localhost)".to_string(), |m| format!("{}%", m))
        ),
    )?;

    // Create alloy provider for on-chain queries (using normalized URL for host)
    let provider = onchain::create_provider(&normalized_url);

    // Create toolkit runner
    let runner = adi_toolkit::ToolkitRunner::with_config_and_logger(
        context.toolkit_config(),
        Arc::clone(context.logger()),
    )
    .await
    .wrap_err("Failed to create toolkit runner")?;
    let wrapper = ToolkitRunnerWrapper(runner);

    // Create orchestrator
    let orchestrator = UpgradeOrchestrator::new(
        handler.as_ref(),
        &upgrade_config,
        &state_dir,
        &wrapper,
        &provider,
        version.to_semver(),
    );

    // Determine upgrade targets
    let upgrade_ecosystem = matches!(args.target, UpgradeTarget::Ecosystem | UpgradeTarget::Both);
    let upgrade_chains = matches!(args.target, UpgradeTarget::Chain | UpgradeTarget::Both);

    if upgrade_ecosystem {
        ui::section("L1 Ecosystem Upgrade")?;

        // Load previous upgrade values
        let previous_values = load_previous_upgrade_values(
            args.previous_upgrade_yaml.as_deref(),
            &state_dir,
            handler.previous_upgrade_yaml(),
        )?;

        // Get chain ID for chain.toml generation (use first chain)
        let chain_names = state_manager.list_chains().await?;
        let chain_id = if let Some(first_chain) = chain_names.first() {
            let chain_meta = state_manager
                .chain(first_chain)
                .metadata()
                .await
                .map_err(|e| eyre::eyre!("Failed to load chain metadata: {e}"))?;
            chain_meta.chain_id
        } else {
            return Err(eyre::eyre!("No chains found in ecosystem state"));
        };

        // Phase 1: Prepare config
        ui::info("Preparing upgrade configuration...")?;
        orchestrator
            .prepare_config(chain_id, &previous_values)
            .await?;
        ui::success("chain.toml generated")?;

        // Phase 2: Simulation
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

        // Phase 3: Broadcast
        ui::info("Running upgrade broadcast...")?;
        let broadcast_result = orchestrator.broadcast().await?;

        if broadcast_result.success {
            ui::success("Broadcast completed successfully")?;
        }

        // Phase 4: Generate upgrade YAML
        ui::info("Generating upgrade YAML...")?;
        let l1_chain_id = provider
            .get_chain_id()
            .await
            .map_err(|e| eyre::eyre!("Failed to get L1 chain ID: {e}"))?;
        orchestrator.generate_upgrade_yaml(l1_chain_id)?;
        ui::success("Upgrade YAML generated")?;

        // Phase 5: Governance execution
        ui::info("Executing governance transactions...")?;
        let gov_result = orchestrator.execute_governance().await?;
        ui::success(format!(
            "Governance executed: schedule={}, execute={}",
            gov_result.schedule_tx_hash, gov_result.execute_tx_hash,
        ))?;

        // Save upgrade YAML for future use
        match orchestrator.save_upgrade_yaml() {
            Ok(path) => ui::success(format!("Upgrade YAML saved to {}", path.display()))?,
            Err(e) => ui::warning(format!("Failed to save upgrade YAML: {e}"))?,
        }
    }

    // Chain upgrades
    if upgrade_chains {
        ui::section("L2 Chain Upgrades")?;

        let chain_names = state_manager.list_chains().await?;

        if chain_names.is_empty() {
            ui::warning("No chains found in ecosystem, skipping chain upgrade")?;
        } else {
            let selected_chains = prompts::select_chains(&chain_names, args.chain.as_ref())?;

            for chain_name in &selected_chains {
                ui::info(format!("Upgrading chain: {}", ui::green(chain_name)))?;

                let chain_meta = state_manager
                    .chain(chain_name)
                    .metadata()
                    .await
                    .map_err(|e| {
                        eyre::eyre!("Failed to load chain metadata for {chain_name}: {e}")
                    })?;

                let upgrade_yaml_source = state_dir
                    .join("l1-contracts")
                    .join("script-out")
                    .join(handler.upgrade_output_yaml());

                // Copy YAML to state_dir root so zkstack finds it at /workspace/<filename>
                let upgrade_yaml_path = state_dir.join(handler.upgrade_output_yaml());
                std::fs::copy(&upgrade_yaml_source, &upgrade_yaml_path).wrap_err(format!(
                    "Failed to copy upgrade YAML from {} to {}",
                    upgrade_yaml_source.display(),
                    upgrade_yaml_path.display()
                ))?;

                // Load chain governor key (chain admin owner, different from ecosystem governor)
                let chain_wallets =
                    state_manager
                        .chain(chain_name)
                        .wallets()
                        .await
                        .map_err(|e| {
                            eyre::eyre!("Failed to load chain wallets for {chain_name}: {e}")
                        })?;
                let chain_governor = chain_wallets
                    .governor
                    .ok_or_else(|| eyre::eyre!("Chain '{chain_name}' has no governor wallet"))?;

                let result = adi_upgrade::run_chain_upgrade(
                    &wrapper,
                    &provider,
                    chain_name,
                    chain_meta.chain_id,
                    upgrade_config.bridgehub_address,
                    &chain_governor.private_key,
                    handler.upgrade_name(),
                    &upgrade_yaml_path,
                    rpc_url.as_str(),
                    "http://127.0.0.1:3050",
                    &state_dir,
                    &version.to_semver(),
                )
                .await?;

                if result.versions_match {
                    ui::success(format!("Chain '{}' upgraded successfully", chain_name))?;
                } else {
                    ui::warning(format!(
                        "Chain '{}' upgrade completed but protocol versions don't match",
                        chain_name
                    ))?;
                }
            }
        }
    }

    ui::outro(format!(
        "Upgrade to {} completed successfully",
        ui::green(version)
    ))?;

    Ok(())
}
