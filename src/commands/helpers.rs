//! Shared helper functions for CLI commands.
//!
//! This module contains common utilities used across multiple commands,
//! reducing code duplication.

use adi_ecosystem::{OwnershipResult, OwnershipState, OwnershipStatusSummary, OwnershipSummary};
use adi_state::StateManager;
use alloy_primitives::Address;
use std::sync::Arc;
use url::Url;

use crate::config::Config;
use crate::context::Context;
use crate::error::{Result, WrapErr};
use crate::ui;

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
pub fn display_summary(summary: &OwnershipSummary) -> Result<()> {
    ui::success(format!("  Successful: {}", summary.successful_count()))?;
    ui::info(format!("  Skipped: {}", summary.skipped_count()))?;
    ui::warning(format!("  Failed: {}", summary.failed_count()))?;

    for result in &summary.results {
        match categorize_result(result) {
            ResultCategory::SuccessWithTx(tx) => {
                ui::success(format!("    {}: {}", result.name, tx))?;
            }
            ResultCategory::SuccessNoTx => {
                ui::success(format!("    {}", result.name))?;
            }
            ResultCategory::Skipped(reason) => {
                ui::info(format!("    {}: {}", result.name, reason))?;
            }
            ResultCategory::Failed(error) => {
                ui::warning(format!("    {}: {}", result.name, error))?;
            }
        }
    }
    Ok(())
}

/// Display ownership status for contracts.
pub fn display_ownership_status(summary: &OwnershipStatusSummary) -> Result<()> {
    for status in &summary.statuses {
        match (status.address, status.state) {
            (Some(addr), OwnershipState::Pending) => {
                ui::warning(format!("  {}: {} (pending)", status.name, addr))?;
            }
            (Some(addr), OwnershipState::Accepted) => {
                ui::success(format!("  {}: {} (accepted)", status.name, addr))?;
            }
            (Some(addr), OwnershipState::NotTransferred) => {
                ui::error(format!(
                    "  {}: {} (ownership not transferred!)",
                    status.name, addr
                ))?;
            }
            (None, _) => {
                ui::info(format!("  {}: not configured", status.name))?;
            }
        }
    }
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
pub fn create_state_manager_with_context(ecosystem_name: &str, context: &Context) -> StateManager {
    let ecosystem_path = context.config().state_dir.join(ecosystem_name);
    StateManager::with_backend_type_and_logger(
        context.config().state_backend.clone(),
        &ecosystem_path,
        Arc::clone(context.logger()),
    )
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
