//! Shared helper functions for CLI commands.
//!
//! This module contains common utilities used across multiple commands,
//! reducing code duplication.

use adi_ecosystem::{
    CalldataOutput, OwnershipResult, OwnershipState, OwnershipStatusSummary, OwnershipSummary,
};
use adi_state::StateManager;
use alloy_primitives::Address;
use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use url::Url;

use crate::config::Config;
use crate::context::Context;
use crate::error::{Result, WrapErr};
use crate::ui;

/// Scope for ownership operations (accept/transfer).
///
/// Determines which contracts are included in ownership operations.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, ValueEnum, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OwnershipScope {
    /// Ecosystem-level contracts only (Governance, ValidatorTimelock, etc.)
    Ecosystem,

    /// Chain-level contracts only (Chain Governance, Chain ChainAdmin)
    Chain,

    /// All contracts (ecosystem + chain) - default behavior
    #[default]
    All,
}

impl std::fmt::Display for OwnershipScope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ecosystem => write!(f, "ecosystem"),
            Self::Chain => write!(f, "chain"),
            Self::All => write!(f, "all"),
        }
    }
}

/// Result of chain selection from config.
///
/// Distinguishes between selecting an existing chain (with config defaults)
/// versus creating a new chain that isn't in the config file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChainSelection {
    /// User selected an existing chain defined in config.
    /// The chain's defaults from `ecosystem.chains[]` should be used.
    Existing(String),

    /// User wants to create a new chain not in config.
    /// Command should use default values or prompt for configuration.
    New(String),
}

impl ChainSelection {
    /// Get the chain name regardless of selection type.
    #[must_use]
    pub fn name(&self) -> &str {
        match self {
            Self::Existing(name) | Self::New(name) => name,
        }
    }
}

/// Category of ownership result for display purposes.
pub enum ResultCategory<'a> {
    /// Transaction was successful and has a hash.
    SuccessWithTx(String),
    /// Success without a transaction hash.
    SuccessNoTx,
    /// Operation was skipped with reason.
    Skipped(&'a str),
    /// Operation failed with error.
    Failed(&'a str),
}

/// Categorize an ownership result for display.
pub fn categorize_result(result: &OwnershipResult) -> ResultCategory<'_> {
    if result.success {
        match &result.tx_hash {
            Some(tx) => ResultCategory::SuccessWithTx(tx.to_string()),
            None => ResultCategory::SuccessNoTx,
        }
    } else {
        match &result.error {
            Some(e) if e.starts_with("Skipped: ") => {
                ResultCategory::Skipped(e.strip_prefix("Skipped: ").unwrap_or(e))
            }
            Some(e) => ResultCategory::Failed(e),
            None => ResultCategory::Failed("unknown error"),
        }
    }
}

/// Display the ownership summary in a note box.
pub fn display_summary(title: &str, summary: &OwnershipSummary) -> Result<()> {
    let mut lines = vec![
        format!(
            "Successful: {}  Skipped: {}  Failed: {}",
            ui::green(summary.successful_count()),
            ui::cyan(summary.skipped_count()),
            ui::yellow(summary.failed_count())
        ),
        String::new(),
    ];

    for result in &summary.results {
        let line = match categorize_result(result) {
            ResultCategory::SuccessWithTx(tx) => {
                format!("{}: {}", result.name, ui::green(tx))
            }
            ResultCategory::SuccessNoTx => {
                format!("{}: {}", result.name, ui::green("success"))
            }
            ResultCategory::Skipped(reason) => {
                format!("{}: {}", result.name, ui::cyan(reason))
            }
            ResultCategory::Failed(error) => {
                format!("{}: {}", result.name, ui::yellow(error))
            }
        };
        lines.push(line);
    }

    ui::note(title, lines.join("\n"))?;
    Ok(())
}

/// Display ownership status for contracts in a note box.
pub fn display_ownership_status(title: &str, summary: &OwnershipStatusSummary) -> Result<()> {
    let lines: Vec<String> = summary
        .statuses
        .iter()
        .map(|status| match (status.address, status.state) {
            (Some(addr), OwnershipState::Pending) => {
                format!(
                    "{}: {} {}",
                    status.name,
                    ui::green(addr),
                    ui::yellow("(pending)")
                )
            }
            (Some(addr), OwnershipState::Accepted) => {
                format!(
                    "{}: {} {}",
                    status.name,
                    ui::green(addr),
                    ui::green("(accepted)")
                )
            }
            (Some(addr), OwnershipState::NotTransferred) => {
                format!(
                    "{}: {} {}",
                    status.name,
                    ui::green(addr),
                    ui::cyan("(no pending transfer)")
                )
            }
            (None, _) => {
                format!("{}: {}", status.name, ui::cyan("not configured"))
            }
        })
        .collect();

    ui::note(title, lines.join("\n"))?;
    Ok(())
}

/// Derive address from private key.
pub fn derive_address_from_key(key: &secrecy::SecretString) -> Result<Address> {
    use alloy_signer_local::PrivateKeySigner;
    use secrecy::ExposeSecret;

    let key_str = key.expose_secret();
    let key_hex = key_str.strip_prefix("0x").unwrap_or(key_str);

    let key_bytes: [u8; 32] = hex::decode(key_hex)
        .wrap_err("Invalid private key hex")?
        .try_into()
        .map_err(|_| eyre::eyre!("Private key must be 32 bytes"))?;

    let signer = PrivateKeySigner::from_bytes(&key_bytes.into()).wrap_err("Invalid private key")?;

    Ok(signer.address())
}

/// Create state manager for the ecosystem with context's logger.
///
/// # Errors
///
/// Returns error if the backend type requires async initialization.
pub fn create_state_manager_with_context(
    ecosystem_name: &str,
    context: &Context,
) -> Result<StateManager> {
    let ecosystem_path = context.config().state_dir.join(ecosystem_name);
    StateManager::with_backend_type_and_logger(
        context.config().state_backend,
        &ecosystem_path,
        Arc::clone(context.logger()),
    )
    .map_err(|e| eyre::eyre!("Failed to create state manager: {e}"))
}

/// Convert CLI S3Config to adi-state S3Config.
///
/// # Errors
///
/// Returns error if required fields are missing when S3 is enabled.
pub fn to_state_s3_config(cli_config: &crate::config::S3Config) -> Result<adi_state::S3Config> {
    use secrecy::ExposeSecret;

    let tenant_id = cli_config
        .tenant_id
        .clone()
        .ok_or_else(|| eyre::eyre!("S3 tenant_id required when s3.enabled=true"))?;

    let bucket = cli_config
        .bucket
        .clone()
        .ok_or_else(|| eyre::eyre!("S3 bucket required when s3.enabled=true"))?;

    let access_key_id = cli_config
        .access_key_id
        .as_ref()
        .map(|s| s.expose_secret().to_string())
        .or_else(|| std::env::var("AWS_ACCESS_KEY_ID").ok())
        .ok_or_else(|| {
            eyre::eyre!("S3 access_key_id required: set in config or AWS_ACCESS_KEY_ID env var")
        })?;

    let secret_access_key = cli_config
        .secret_access_key
        .as_ref()
        .map(|s| s.expose_secret().to_string())
        .or_else(|| std::env::var("AWS_SECRET_ACCESS_KEY").ok())
        .ok_or_else(|| {
            eyre::eyre!(
                "S3 secret_access_key required: set in config or AWS_SECRET_ACCESS_KEY env var"
            )
        })?;

    Ok(adi_state::S3Config {
        bucket,
        region: cli_config
            .region
            .clone()
            .unwrap_or_else(|| "us-east-1".to_string()),
        endpoint_url: cli_config.endpoint_url.as_ref().map(|u| u.to_string()),
        tenant_id,
        access_key_id,
        secret_access_key,
    })
}

/// Optional S3 sync control handle.
pub type OptionalS3Control = Option<adi_state::S3SyncControl>;

/// Create state manager with optional S3 sync and control handle.
///
/// If `s3.enabled=true` in config, creates S3SyncBackend with deferred sync mode.
/// Use the returned `S3SyncControl` to disable auto-sync for batch operations
/// and trigger manual sync when ready.
///
/// # Returns
///
/// Returns `(StateManager, Option<S3SyncControl>)`. The control handle is `Some`
/// only when S3 sync is enabled.
///
/// # Errors
///
/// Returns error if S3 is enabled but initialization fails.
pub async fn create_state_manager_with_s3(
    ecosystem_name: &str,
    context: &Context,
) -> Result<(StateManager, OptionalS3Control)> {
    let ecosystem_path = context.config().state_dir.join(ecosystem_name);

    if context.config().s3.enabled {
        use crate::s3_events::SpinnerS3EventHandler;

        let s3_config = to_state_s3_config(&context.config().s3)?;
        let event_handler = Arc::new(SpinnerS3EventHandler::new());

        let (manager, control) = StateManager::with_s3_sync_and_control(
            &ecosystem_path,
            ecosystem_name,
            s3_config,
            Arc::clone(context.logger()),
            event_handler,
        )
        .await
        .wrap_err("Failed to initialize S3 sync backend")?;

        return Ok((manager, Some(control)));
    }

    // Fallback to filesystem backend
    let manager = StateManager::with_backend_type_and_logger(
        context.config().state_backend,
        &ecosystem_path,
        Arc::clone(context.logger()),
    )
    .map_err(|e| eyre::eyre!("Failed to create state manager: {e}"))?;

    Ok((manager, None))
}

/// Collect existing chain names and their IDs from the ecosystem.
///
/// Used for validating chain name/ID uniqueness before creating a new chain.
///
/// # Returns
///
/// Vector of `(chain_name, chain_id)` tuples for all existing chains.
pub async fn collect_existing_chains(state_manager: &StateManager) -> Result<Vec<(String, u64)>> {
    let chain_names = state_manager.list_chains().await?;
    let mut chains = Vec::with_capacity(chain_names.len());

    for name in chain_names {
        let chain_ops = state_manager.chain(&name);
        if let Ok(metadata) = chain_ops.metadata().await {
            chains.push((name, metadata.chain_id));
        }
    }

    Ok(chains)
}

/// Resolve ecosystem name from optional arg or config.
pub fn resolve_ecosystem_name(arg_value: Option<&String>, config: &Config) -> Result<String> {
    arg_value
        .cloned()
        .or_else(|| Some(config.ecosystem.name.clone()))
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            eyre::eyre!("Ecosystem name required: use --ecosystem-name or set in config")
        })
}

/// Resolve chain name from optional arg or config.
///
/// Falls back to the first chain in `ecosystem.chains[]` if available.
pub fn resolve_chain_name(arg_value: Option<&String>, config: &Config) -> Result<String> {
    arg_value
        .cloned()
        .or_else(|| config.ecosystem.default_chain().map(|c| c.name.clone()))
        .filter(|s| !s.is_empty())
        .ok_or_else(|| eyre::eyre!("Chain name required: use --chain or set in config"))
}

/// Select a chain interactively from deployed state.
///
/// Behavior:
/// - If `--chain` provided via CLI: validate it exists and use it
/// - If 1 chain in state: auto-select with info message
/// - If multiple chains: show interactive selection
///
/// # Arguments
///
/// * `chain_arg` - Optional chain name from `--chain` CLI argument
/// * `state_manager` - StateManager to query chains from state
/// * `ecosystem_name` - Ecosystem name for error messages
///
/// # Errors
///
/// Returns error if no chains exist in state or user cancels selection.
pub async fn select_chain_from_state(
    chain_arg: Option<&String>,
    state_manager: &StateManager,
    ecosystem_name: &str,
) -> Result<String> {
    // If CLI arg provided, validate and use it
    if let Some(chain_name) = chain_arg {
        let chains = state_manager.list_chains().await?;
        if chains.contains(chain_name) {
            return Ok(chain_name.clone());
        }
        return Err(eyre::eyre!(
            "Chain '{}' not found in ecosystem '{}'. Available chains: {}",
            chain_name,
            ecosystem_name,
            chains.join(", ")
        ));
    }

    // List available chains from state
    let chains = state_manager.list_chains().await?;

    match chains.len() {
        0 => Err(eyre::eyre!(
            "No chains found in ecosystem '{}'. Run 'adi init' first.",
            ecosystem_name
        )),
        1 => {
            let chain_name = chains
                .into_iter()
                .next()
                .ok_or_else(|| eyre::eyre!("Failed to get chain name"))?;
            ui::info(format!("Auto-selected chain: {}", ui::green(&chain_name)))?;
            Ok(chain_name)
        }
        _ => {
            // Multiple chains - show interactive selection
            let items: Vec<(String, String, String)> = chains
                .into_iter()
                .map(|name| (name.clone(), name, String::new()))
                .collect();

            let selected: String = ui::select("Select a chain")
                .items(&items)
                .interact()
                .wrap_err("Chain selection cancelled")?;

            Ok(selected)
        }
    }
}

/// Select a chain from config defaults or prompt for new chain.
///
/// Behavior:
/// - If `--chain` provided via CLI: existing if in config, else New
/// - If 0 chains in config + allow_new: prompt for new chain name
/// - If 1 chain in config + !allow_new: auto-select
/// - If 1+ chains + allow_new: selection with "Create new" option
///
/// # Arguments
///
/// * `chain_arg` - Optional chain name from CLI argument
/// * `allow_new` - If true, shows "Create new chain..." option
/// * `config` - Config containing `ecosystem.chains[]` defaults
///
/// # Returns
///
/// * `ChainSelection::Existing(name)` - Selected existing chain from config
/// * `ChainSelection::New(name)` - User wants to create new chain
///
/// # Errors
///
/// Returns error if no chains in config and allow_new is false, or user cancels.
pub fn select_chain_from_config(
    chain_arg: Option<&String>,
    allow_new: bool,
    config: &Config,
) -> Result<ChainSelection> {
    let chain_names = config.ecosystem.chain_names();

    // If CLI arg provided, check if it exists in config
    if let Some(chain_name) = chain_arg {
        if chain_names.iter().any(|n| n == chain_name) {
            return Ok(ChainSelection::Existing(chain_name.clone()));
        }
        // CLI user specified a name not in config - treat as new
        return Ok(ChainSelection::New(chain_name.clone()));
    }

    match (chain_names.len(), allow_new) {
        // No chains and can't create new
        (0, false) => Err(eyre::eyre!(
            "No chains defined in config. Add chains to ecosystem.chains[] or provide --chain-name."
        )),

        // No chains but can create new - prompt for name
        (0, true) => {
            let name: String = ui::input("Enter chain name")
                .placeholder("my_chain")
                .interact()
                .wrap_err("Failed to read chain name")?;

            if name.is_empty() {
                return Err(eyre::eyre!("Chain name cannot be empty"));
            }

            Ok(ChainSelection::New(name))
        }

        // Single chain and can't create new - auto-select
        (1, false) => {
            let chain_name = chain_names
                .first()
                .ok_or_else(|| eyre::eyre!("Failed to get chain name"))?
                .to_string();

            ui::info(format!("Auto-selected chain: {}", ui::green(&chain_name)))?;
            Ok(ChainSelection::Existing(chain_name))
        }

        // Has chains - show selection (with or without "Create new" option)
        _ => {
            const CREATE_NEW_VALUE: &str = "__create_new__";

            let mut items: Vec<(String, String, String)> = chain_names
                .iter()
                .map(|name| {
                    (
                        name.to_string(),
                        name.to_string(),
                        "Use config defaults".to_string(),
                    )
                })
                .collect();

            if allow_new {
                items.push((
                    CREATE_NEW_VALUE.to_string(),
                    "[Create new chain...]".to_string(),
                    "Enter custom chain name".to_string(),
                ));
            }

            let selected: String = ui::select("Select a chain")
                .items(&items)
                .interact()
                .wrap_err("Chain selection cancelled")?;

            if selected == CREATE_NEW_VALUE {
                let name: String = ui::input("Enter chain name")
                    .placeholder("my_chain")
                    .interact()
                    .wrap_err("Failed to read chain name")?;

                if name.is_empty() {
                    return Err(eyre::eyre!("Chain name cannot be empty"));
                }

                return Ok(ChainSelection::New(name));
            }

            Ok(ChainSelection::Existing(selected))
        }
    }
}

/// Resolve RPC URL from optional arg or config.
///
/// Priority: CLI arg > ecosystem.rpc_url > funding.rpc_url (backward compat)
pub fn resolve_rpc_url(arg_value: Option<&Url>, config: &Config) -> Result<Url> {
    arg_value
        .cloned()
        .or_else(|| config.ecosystem.rpc_url.clone())
        .or_else(|| config.funding.rpc_url.clone()) // backward compatibility
        .ok_or_else(|| {
            eyre::eyre!("RPC URL required: use --rpc-url or set ecosystem.rpc_url in config")
        })
}

/// Resolve ecosystem new owner from config.
///
/// Priority: CLI arg > ecosystem.ownership.new_owner > ownership.new_owner (deprecated)
pub fn resolve_ecosystem_new_owner(arg_value: Option<Address>, config: &Config) -> Result<Address> {
    arg_value
        .or(config.ecosystem.ownership.new_owner)
        .or(config.ownership.new_owner) // backward compatibility
        .ok_or_else(|| {
            eyre::eyre!(
                "Ecosystem new owner required: use --new-owner or set ecosystem.ownership.new_owner in config"
            )
        })
}

/// Resolve chain new owner from config.
///
/// Priority: CLI arg > chains[name].ownership.new_owner > ownership.new_owner (deprecated)
pub fn resolve_chain_new_owner(
    arg_value: Option<Address>,
    chain_name: &str,
    config: &Config,
) -> Result<Address> {
    arg_value
        .or_else(|| {
            config
                .ecosystem
                .get_chain(chain_name)
                .and_then(|c| c.ownership.new_owner)
        })
        .or(config.ownership.new_owner) // backward compatibility
        .ok_or_else(|| {
            eyre::eyre!(
                "Chain new owner required: use --new-owner or set ecosystem.chains[{}].ownership.new_owner in config",
                chain_name
            )
        })
}

/// Resolve protocol version from optional arg or config.
pub fn resolve_protocol_version(arg_value: Option<&String>, config: &Config) -> Result<String> {
    arg_value
        .cloned()
        .or_else(|| config.protocol_version.clone())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            eyre::eyre!(
                "Protocol version required: use --protocol-version or set protocol_version in config"
            )
        })
}

/// Display calldata output for external submission.
pub fn display_calldata_output(title: &str, output: &CalldataOutput) -> Result<()> {
    if output.is_empty() {
        ui::note(title, "No pending ownership transfers")?;
        return Ok(());
    }

    let mut lines = Vec::new();
    for entry in &output.entries {
        lines.push(format!("{}", ui::cyan(&entry.name)));
        lines.push(format!("  To:       {}", ui::green(entry.to)));
        lines.push(format!("  Call:     {}", entry.description));
        lines.push(format!("  Calldata: {}", entry.calldata));
        lines.push(String::new());
    }

    // Remove trailing empty line
    if lines.last().is_some_and(|s| s.is_empty()) {
        lines.pop();
    }

    ui::note(title, lines.join("\n"))?;
    Ok(())
}
