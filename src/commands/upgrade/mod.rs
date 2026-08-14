//! Upgrade command for ecosystem and chain contracts.

mod args;
mod legacy;
mod prompts;
mod runner_wrapper;
mod v31;

use std::sync::Arc;

pub use args::{UpgradeArgs, UpgradeTarget};
use legacy::{run_chain_upgrades, run_ecosystem_upgrade, ChainUpgradeContext};
use runner_wrapper::ToolkitRunnerWrapper;

use crate::context::Context;
use crate::error::Result;

/// Execute the upgrade command.
pub async fn run(args: UpgradeArgs, context: &Context) -> Result<()> {
    use crate::commands::helpers::{
        create_state_manager_with_s3, resolve_ecosystem_name, resolve_gas_multiplier,
        resolve_rpc_url,
    };
    use crate::error::WrapErr;
    use crate::ui;
    use adi_toolkit::ProtocolVersion;
    use adi_upgrade::{get_handler, onchain, UpgradeConfig, UpgradeOrchestrator};
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

    // The output-only / rehearsal flags are wired only through the v31 path; the
    // legacy engine ignores them, so reject them here rather than silently broadcast.
    if version != ProtocolVersion::V0_31_0
        && (args.safe || args.calldata || args.unlocked || args.fork || args.sign_deployer)
    {
        return Err(eyre::eyre!(
            "--safe/--calldata/--unlocked/--fork/--sign-deployer apply only to the v31 upgrade"
        ));
    }

    // v31 uses its own orchestration (protocol_ops), not the forge-script
    // VersionHandler engine — dispatch before get_handler (which returns None).
    if version == ProtocolVersion::V0_31_0 {
        return v31::run_v31(&args, context, &ecosystem_name).await;
    }

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

    // Build upgrade config — skip gas price for localhost (anvil)
    let gas_multiplier = if adi_types::is_localhost_rpc(rpc_url.as_str()) {
        None
    } else {
        Some(resolve_gas_multiplier(None, context.config()))
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
    let orchestrator = UpgradeOrchestrator::new(adi_upgrade::OrchestratorParams {
        handler: handler.as_ref(),
        config: &upgrade_config,
        state_dir: &state_dir,
        runner: &wrapper,
        provider: &provider,
        protocol_version: version.to_semver(),
    });

    // Determine upgrade targets
    let upgrade_ecosystem = matches!(args.target, UpgradeTarget::Ecosystem | UpgradeTarget::Both);
    let upgrade_chains = matches!(args.target, UpgradeTarget::Chain | UpgradeTarget::Both);

    if upgrade_ecosystem {
        run_ecosystem_upgrade(
            &args,
            &orchestrator,
            &state_manager,
            &state_dir,
            handler.as_ref(),
            &provider,
        )
        .await?;
    }

    if upgrade_chains {
        let chain_ctx = ChainUpgradeContext {
            state_manager: &state_manager,
            state_dir: &state_dir,
            handler: handler.as_ref(),
            upgrade_config: &upgrade_config,
            wrapper: &wrapper,
            provider: &provider,
            rpc_url: &rpc_url,
            version: &version,
        };
        run_chain_upgrades(&args, &chain_ctx).await?;
    }

    ui::outro(format!(
        "Upgrade to {} completed successfully",
        ui::green(version)
    ))?;

    Ok(())
}
