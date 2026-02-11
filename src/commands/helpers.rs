//! Shared helper functions for CLI commands.
//!
//! This module contains common utilities used across multiple commands,
//! reducing code duplication.

use adi_ecosystem::{OwnershipResult, OwnershipState, OwnershipStatusSummary, OwnershipSummary};
use adi_state::StateManager;
use alloy_primitives::Address;
use colored::Colorize;
use url::Url;

use crate::config::Config;
use crate::error::{Result, WrapErr};

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

/// Display the ownership summary.
pub fn display_summary(summary: &OwnershipSummary) {
    log::info!(
        "  Successful: {}",
        summary.successful_count().to_string().green()
    );
    log::info!("  Skipped: {}", summary.skipped_count().to_string().cyan());
    log::info!("  Failed: {}", summary.failed_count().to_string().yellow());

    for result in &summary.results {
        match categorize_result(result) {
            ResultCategory::SuccessWithTx(tx) => {
                log::info!("    {} {}: {}", "✓".green(), result.name, tx.green());
            }
            ResultCategory::SuccessNoTx => {
                log::info!("    {} {}", "✓".green(), result.name);
            }
            ResultCategory::Skipped(reason) => {
                log::info!("    {} {}: {}", "⊘".cyan(), result.name, reason.cyan());
            }
            ResultCategory::Failed(error) => {
                log::info!("    {} {}: {}", "✗".yellow(), result.name, error.yellow());
            }
        }
    }
}

/// Display ownership status for contracts.
pub fn display_ownership_status(summary: &OwnershipStatusSummary) {
    for status in &summary.statuses {
        match (status.address, status.state) {
            (Some(addr), OwnershipState::Pending) => {
                log::info!(
                    "  {} {}: {} {}",
                    "⏳".yellow(),
                    status.name,
                    addr.to_string().green(),
                    "(pending)".yellow()
                );
            }
            (Some(addr), OwnershipState::Accepted) => {
                log::info!(
                    "  {} {}: {} {}",
                    "✓".green(),
                    status.name,
                    addr.to_string().green(),
                    "(accepted)".green()
                );
            }
            (Some(addr), OwnershipState::NotTransferred) => {
                log::info!(
                    "  {} {}: {} {}",
                    "⚠".red(),
                    status.name,
                    addr.to_string().green(),
                    "(ownership not transferred!)".red()
                );
            }
            (None, _) => {
                log::info!(
                    "  {} {}: {}",
                    "⊘".cyan(),
                    status.name,
                    "not configured".cyan()
                );
            }
        }
    }
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

/// Create state manager for the ecosystem.
pub fn create_state_manager(ecosystem_name: &str, config: &Config) -> StateManager {
    let ecosystem_path = config.state_dir.join(ecosystem_name);
    StateManager::with_backend_type(config.state_backend.clone(), &ecosystem_path)
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
pub fn resolve_chain_name(arg_value: Option<&String>, config: &Config) -> Result<String> {
    arg_value
        .cloned()
        .or_else(|| Some(config.ecosystem.chain_name.clone()))
        .filter(|s| !s.is_empty())
        .ok_or_else(|| eyre::eyre!("Chain name required: use --chain or set in config"))
}

/// Resolve RPC URL from optional arg or config.
pub fn resolve_rpc_url(arg_value: Option<&Url>, config: &Config) -> Result<Url> {
    arg_value
        .cloned()
        .or_else(|| config.funding.rpc_url.clone())
        .ok_or_else(|| eyre::eyre!("RPC URL required: use --rpc-url or set in config"))
}

/// Resolve new owner address from optional arg or config.
pub fn resolve_new_owner(arg_value: Option<Address>, config: &Config) -> Result<Address> {
    arg_value
        .or(config.ownership.new_owner)
        .ok_or_else(|| eyre::eyre!("New owner required: use --new-owner or set in config"))
}
