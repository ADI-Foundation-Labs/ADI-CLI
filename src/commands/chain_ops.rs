//! Shared chain creation operations.
//!
//! This module contains chain creation logic shared between the `init` and `add` commands.

use adi_ecosystem::{build_chain_create_args, verify_chain_created, ChainConfig, ChainDefaults};
use adi_state::{export_ecosystem_state, import_chain_state, StateManager};
use adi_toolkit::{ProtocolVersion, ToolkitRunner, GENESIS_FILENAME};
use std::sync::Arc;
use tempfile::TempDir;

use super::helpers::OptionalS3Control;
use crate::context::Context;
use crate::error::{Result, WrapErr};
use crate::ui;

/// Create a chain within an existing ecosystem.
///
/// This function:
/// 1. Builds zkstack chain create arguments
/// 2. Creates temp directory and exports ecosystem state
/// 3. Copies genesis.json to temp directory
/// 4. Runs zkstack chain create via Docker toolkit
/// 5. Verifies chain files were created
/// 6. Imports chain state through StateManager
/// 7. Syncs to S3 if enabled
///
/// # Arguments
///
/// * `ecosystem_name` - Name of the ecosystem to add the chain to
/// * `chain_defaults` - Full chain configuration including operators/funding/ownership
/// * `state_manager` - StateManager for state operations
/// * `s3_control` - Optional S3 sync control
/// * `version` - Protocol version for toolkit image
/// * `context` - CLI context with config and logger
pub async fn create_chain(
    ecosystem_name: &str,
    chain_defaults: &ChainDefaults,
    state_manager: &StateManager,
    s3_control: &OptionalS3Control,
    version: &ProtocolVersion,
    context: &Context,
) -> Result<()> {
    let state_dir = &context.config().state_dir;

    // Convert ChainDefaults to ChainConfig for zkstack
    let chain_config = chain_defaults_to_config(chain_defaults);

    // Build zkstack chain create args
    let zkstack_args = build_chain_create_args(&chain_config);
    context
        .logger()
        .debug(&format!("zkstack args: {:?}", zkstack_args));

    // Create temp directory and export ecosystem state
    let temp_dir = TempDir::new().wrap_err("Failed to create temporary directory")?;
    let temp_path = temp_dir
        .path()
        .canonicalize()
        .wrap_err("Failed to resolve temp directory to absolute path")?;
    context
        .logger()
        .debug(&format!("Using temp directory: {}", temp_path.display()));

    // Export ecosystem state to temp dir (in ecosystem subdirectory)
    // zkstack expects ZkStack.yaml at /workspace root, so we mount the ecosystem dir
    let ecosystem_temp_path = temp_path.join(ecosystem_name);
    ui::info("Exporting ecosystem state to temp directory...")?;
    export_ecosystem_state(state_manager, &ecosystem_temp_path)
        .await
        .wrap_err("Failed to export ecosystem state")?;

    // Copy genesis.json to ecosystem temp directory
    let genesis_src = state_dir.join(GENESIS_FILENAME);
    if !genesis_src.exists() {
        return Err(eyre::eyre!(
            "genesis.json not found at {}. Run 'adi init' first.",
            genesis_src.display()
        ));
    }
    let genesis_dst = ecosystem_temp_path.join(GENESIS_FILENAME);
    std::fs::copy(&genesis_src, &genesis_dst).wrap_err("Failed to copy genesis.json")?;

    // Create toolkit runner and execute zkstack chain create
    ui::info("Connecting to Docker...")?;
    let runner = ToolkitRunner::with_config_and_logger(
        context.toolkit_config(),
        Arc::clone(context.logger()),
    )
    .await
    .wrap_err("Failed to create toolkit runner")?;

    ui::info("Running zkstack chain create...")?;
    let args_refs: Vec<&str> = zkstack_args.iter().map(String::as_str).collect();

    // Run zkstack with ecosystem dir as working directory (mounted as /workspace)
    let exit_code = runner
        .run_zkstack(
            &args_refs,
            &ecosystem_temp_path,
            state_dir,
            &version.to_semver(),
        )
        .await
        .wrap_err("Failed to run zkstack chain create")?;

    if exit_code != 0 {
        return Err(eyre::eyre!(
            "zkstack chain create failed with exit code {}",
            exit_code
        ));
    }

    // Verify chain was created
    ui::info("Verifying chain files...")?;
    verify_chain_created(
        &temp_path,
        ecosystem_name,
        &chain_config.name,
        context.logger().as_ref(),
    )
    .wrap_err("Chain verification failed")?;

    // Import chain state
    ui::info("Importing chain state...")?;
    import_chain_state(
        state_manager,
        &temp_path,
        ecosystem_name,
        &chain_config.name,
    )
    .await
    .wrap_err("Failed to import chain state")?;

    // Validate imported state
    ui::info("Validating imported state...")?;
    let chain_metadata = state_manager
        .chain(&chain_config.name)
        .metadata()
        .await
        .wrap_err("Failed to read chain metadata")?;

    context.logger().debug(&format!(
        "Chain '{}' validated: chain_id={}",
        chain_config.name, chain_metadata.chain_id
    ));

    // Sync to S3 if enabled
    if let Some(control) = s3_control {
        control.sync_now().await.wrap_err("Failed to sync to S3")?;
    }

    let chains = state_manager.list_chains().await?;
    ui::success(format!(
        "Chain added. Ecosystem now has {} chain(s)",
        chains.len()
    ))?;

    Ok(())
}

/// Convert ChainDefaults to ChainConfig for zkstack.
///
/// ChainDefaults contains all config (operators, funding, ownership).
/// ChainConfig contains only the fields needed for zkstack commands.
fn chain_defaults_to_config(defaults: &ChainDefaults) -> ChainConfig {
    ChainConfig {
        name: defaults.name.clone(),
        chain_id: defaults.chain_id,
        prover_mode: defaults.prover_mode,
        base_token_address: defaults
            .base_token_address
            .unwrap_or(adi_types::ETH_TOKEN_ADDRESS),
        base_token_price_nominator: defaults.base_token_price_nominator,
        base_token_price_denominator: defaults.base_token_price_denominator,
        evm_emulator: defaults.evm_emulator,
        blobs: defaults.blobs,
    }
}
