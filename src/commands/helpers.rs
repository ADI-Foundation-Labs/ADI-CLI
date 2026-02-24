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
pub fn create_state_manager_with_context(ecosystem_name: &str, context: &Context) -> StateManager {
    let ecosystem_path = context.config().state_dir.join(ecosystem_name);
    StateManager::with_backend_type_and_logger(
        context.config().state_backend.clone(),
        &ecosystem_path,
        Arc::clone(context.logger()),
    )
}

/// Convert CLI S3Config to adi-state S3Config.
///
/// # Errors
///
/// Returns error if required fields are missing when S3 is enabled.
#[cfg(feature = "s3")]
pub fn to_state_s3_config(cli_config: &crate::config::S3Config) -> Result<adi_state::S3Config> {
    use secrecy::ExposeSecret;

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
        access_key_id,
        secret_access_key,
    })
}

/// Create state manager with optional S3 sync based on config.
///
/// If `s3.enabled=true` in config, creates S3SyncBackend that automatically
/// syncs state to S3 after every write operation.
/// Otherwise, creates regular FilesystemBackend.
///
/// # Errors
///
/// Returns error if S3 is enabled but initialization fails.
pub async fn create_state_manager_with_s3(
    ecosystem_name: &str,
    context: &Context,
) -> Result<StateManager> {
    let ecosystem_path = context.config().state_dir.join(ecosystem_name);

    #[cfg(feature = "s3")]
    if context.config().s3.enabled {
        let s3_config = to_state_s3_config(&context.config().s3)?;
        return StateManager::with_s3_sync(
            &ecosystem_path,
            ecosystem_name,
            s3_config,
            Arc::clone(context.logger()),
        )
        .await
        .wrap_err("Failed to initialize S3 sync backend");
    }

    // Fallback to filesystem backend
    Ok(StateManager::with_backend_type_and_logger(
        context.config().state_backend.clone(),
        &ecosystem_path,
        Arc::clone(context.logger()),
    ))
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
