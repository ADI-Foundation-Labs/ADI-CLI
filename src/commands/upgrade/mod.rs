//! Upgrade command for ecosystem and chain contracts.

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

/// Execute the upgrade command.
pub async fn run(args: UpgradeArgs, context: &Context) -> Result<()> {
    use crate::commands::helpers::resolve_ecosystem_name;
    use crate::error::WrapErr;
    use crate::ui;
    use adi_toolkit::ProtocolVersion;
    use adi_upgrade::{get_handler, is_supported};

    let ecosystem_name = resolve_ecosystem_name(args.ecosystem_name.as_ref(), context.config())?;

    ui::intro(format!(
        "Upgrading {} to {}",
        ui::green(&ecosystem_name),
        ui::green(&args.protocol_version)
    ))?;

    // Parse and validate protocol version
    let version =
        ProtocolVersion::parse(&args.protocol_version).wrap_err("Invalid protocol version")?;

    if !is_supported(&version) {
        return Err(eyre::eyre!(
            "Protocol version {} is not supported for upgrades",
            version
        ));
    }

    let handler =
        get_handler(&version).ok_or_else(|| eyre::eyre!("No handler for version {}", version))?;

    ui::info(format!(
        "Using upgrade script: {}",
        ui::green(handler.upgrade_script())
    ))?;

    let hooks = handler.post_upgrade_hooks();
    if !hooks.is_empty() {
        ui::info(format!("Post-upgrade hooks: {:?}", hooks))?;
    }

    ui::note(
        "Upgrade Target",
        format!(
            "Target: {:?}\nChain: {}\nSkip simulation: {}",
            args.target,
            args.chain.as_deref().unwrap_or("(all)"),
            args.skip_simulation
        ),
    )?;

    ui::outro("Upgrade command registered (implementation pending)")?;

    Ok(())
}
