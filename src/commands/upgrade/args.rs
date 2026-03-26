//! Upgrade command arguments and types.

use clap::{Args, ValueEnum};
use serde::{Deserialize, Serialize};

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

    /// L2 RPC URL for chain upgrades (defaults to http://127.0.0.1:3050)
    #[arg(long)]
    pub l2_rpc_url: Option<url::Url>,
}
