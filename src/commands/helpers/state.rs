//! State management helpers for creating state managers and querying state.

use adi_state::StateManager;
use alloy_primitives::Address;
use std::sync::Arc;

use crate::context::Context;
use crate::error::{Result, WrapErr};

/// Optional S3 sync control handle.
pub type OptionalS3Control = Option<adi_state::S3SyncControl>;

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
