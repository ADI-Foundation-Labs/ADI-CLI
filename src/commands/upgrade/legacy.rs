//! The forge-script upgrade engine for pre-v31 protocol versions.

use super::args::UpgradeArgs;
use crate::error::Result;

/// Run ecosystem-level upgrade phases.
pub(super) async fn run_ecosystem_upgrade<R, P>(
    args: &UpgradeArgs,
    orchestrator: &adi_upgrade::UpgradeOrchestrator<'_, R, P>,
    state_manager: &adi_state::StateManager,
    state_dir: &std::path::Path,
    handler: &dyn adi_upgrade::VersionHandler,
    provider: &P,
) -> Result<()>
where
    R: adi_upgrade::ToolkitRunnerTrait,
    P: alloy_provider::Provider + Clone,
{
    use crate::error::WrapErr;
    use crate::ui;
    use adi_upgrade::load_previous_upgrade_values;

    ui::section("L1 Ecosystem Upgrade")?;

    // Load previous upgrade values
    let previous_values = load_previous_upgrade_values(
        args.previous_upgrade_yaml.as_deref(),
        state_dir,
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

        let proceed: bool = args.yes
            || ui::confirm("Proceed with broadcast?")
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

    Ok(())
}

/// Context for chain-level upgrade operations.
pub(super) struct ChainUpgradeContext<'a, R, P> {
    /// Ecosystem state manager.
    pub state_manager: &'a adi_state::StateManager,
    /// Ecosystem state directory.
    pub state_dir: &'a std::path::Path,
    /// Version handler for the target protocol version.
    pub handler: &'a dyn adi_upgrade::VersionHandler,
    /// Resolved upgrade config (bridgehub, wallets, gas).
    pub upgrade_config: &'a adi_upgrade::UpgradeConfig,
    /// Toolkit runner wrapper.
    pub wrapper: &'a R,
    /// Alloy provider for on-chain queries.
    pub provider: &'a P,
    /// Settlement-layer RPC URL.
    pub rpc_url: &'a url::Url,
    /// Target protocol version.
    pub version: &'a adi_toolkit::ProtocolVersion,
}

/// Run chain-level upgrades.
pub(super) async fn run_chain_upgrades<R, P>(
    args: &UpgradeArgs,
    ctx: &ChainUpgradeContext<'_, R, P>,
) -> Result<()>
where
    R: adi_upgrade::ToolkitRunnerTrait,
    P: alloy_provider::Provider + Clone,
{
    use crate::error::WrapErr;
    use crate::ui;

    ui::section("L2 Chain Upgrades")?;

    let chain_names = ctx.state_manager.list_chains().await?;

    if chain_names.is_empty() {
        ui::warning("No chains found in ecosystem, skipping chain upgrade")?;
        return Ok(());
    }

    let selected_chains = super::prompts::select_chains(&chain_names, args.chain.as_ref())?;

    for chain_name in &selected_chains {
        ui::info(format!("Upgrading chain: {}", ui::green(chain_name)))?;

        let chain_meta = ctx
            .state_manager
            .chain(chain_name)
            .metadata()
            .await
            .map_err(|e| eyre::eyre!("Failed to load chain metadata for {chain_name}: {e}"))?;

        let upgrade_yaml_source = ctx
            .state_dir
            .join("l1-contracts")
            .join("script-out")
            .join(ctx.handler.upgrade_output_yaml());

        // Copy YAML to state_dir root so zkstack finds it at /workspace/<filename>
        let upgrade_yaml_path = ctx.state_dir.join(ctx.handler.upgrade_output_yaml());
        tokio::fs::copy(&upgrade_yaml_source, &upgrade_yaml_path)
            .await
            .wrap_err(format!(
                "Failed to copy upgrade YAML from {} to {}",
                upgrade_yaml_source.display(),
                upgrade_yaml_path.display(),
            ))?;

        // Load chain governor key (chain admin owner, different from ecosystem governor)
        let chain_wallets = ctx
            .state_manager
            .chain(chain_name)
            .wallets()
            .await
            .map_err(|e| eyre::eyre!("Failed to load chain wallets for {chain_name}: {e}"))?;
        let chain_governor = chain_wallets
            .governor
            .ok_or_else(|| eyre::eyre!("Chain '{chain_name}' has no governor wallet"))?;

        let semver = ctx.version.to_semver();
        let l2_rpc = args
            .l2_rpc_url
            .as_ref()
            .map(|u| u.to_string())
            .unwrap_or_else(|| "http://127.0.0.1:3050".to_string());

        let params = adi_upgrade::ChainUpgradeParams {
            chain_name,
            chain_id: chain_meta.chain_id,
            bridgehub: ctx.upgrade_config.bridgehub_address,
            governor_key: &chain_governor.private_key,
            upgrade_name: ctx.handler.upgrade_name(),
            upgrade_yaml_path: &upgrade_yaml_path,
            l1_rpc_url: ctx.rpc_url.as_str(),
            l2_rpc_url: &l2_rpc,
            state_dir: ctx.state_dir,
            protocol_version: &semver,
        };

        let result = adi_upgrade::run_chain_upgrade(ctx.wrapper, ctx.provider, &params).await?;

        if result.versions_match {
            ui::success(format!("Chain '{}' upgraded successfully", chain_name))?;
        } else {
            ui::warning(format!(
                "Chain '{}' upgrade completed but protocol versions don't match",
                chain_name
            ))?;
        }
    }

    Ok(())
}
