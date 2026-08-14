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

    /// Answer yes to confirmation prompts (non-interactive mode)
    #[arg(long, short = 'y')]
    pub yes: bool,

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

    /// v31 upgrade timestamp in unix seconds (`1` = immediately). On a real net,
    /// pass a coordinated future time.
    #[arg(long)]
    pub upgrade_timestamp: Option<u64>,

    /// v31 output-only: print each operation's Safe Transaction Builder JSON and
    /// wait for you to execute it, instead of broadcasting.
    #[arg(long)]
    pub safe: bool,

    /// v31 output-only: print each operation's raw calldata (`to`/`value`/`data`)
    /// and wait for you to execute it, instead of broadcasting.
    #[arg(long)]
    pub calldata: bool,

    /// v31 fork rehearsal: broadcast every phase via node impersonation
    /// (`protocol_ops --unlocked`) instead of signing with keys. Use against an
    /// anvil fork started with `--auto-impersonate`; no private keys are needed.
    #[arg(long)]
    pub unlocked: bool,

    /// v31 L1-only fork rehearsal: skip the steps that need the L2 server on
    /// v0.20.x — the readiness gate and the pre-v31 total supply (which
    /// reads L2BaseToken at 0x800a). Against a live pre-upgrade L2 (v0.13) those
    /// hang on finality or find no 0x800a. Never pass on a real upgrade.
    #[arg(long)]
    pub fork: bool,

    /// v31 output-only (`--calldata`/`--safe`): broadcast the deployer-signed
    /// phases (the ~40 prepare deploys + stage3) directly with the deployer key
    /// from state, instead of printing them as calldata. Governance and
    /// chain-owner operations still print calldata/Safe. The deployer must be
    /// funded.
    #[arg(long)]
    pub sign_deployer: bool,
}
