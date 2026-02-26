//! Chain addition command implementation.

use adi_ecosystem::{build_chain_create_args, verify_chain_created, ChainConfig, EcosystemConfig};
use adi_state::{export_ecosystem_state, import_chain_state};
use adi_toolkit::{ProtocolVersion, ToolkitRunner, GENESIS_FILENAME};
use std::sync::Arc;
use tempfile::TempDir;

use super::AddArgs;
use crate::commands::helpers::{
    create_state_manager_with_s3, resolve_ecosystem_name, resolve_protocol_version,
};
use crate::context::Context;
use crate::error::{Result, WrapErr};
use crate::ui;

/// Execute the chain addition command.
///
/// This command:
/// 1. Validates the protocol version
/// 2. Merges CLI args with config defaults
/// 3. Validates ecosystem exists
/// 4. Checks if chain already exists, handles conflict
/// 5. Shows config summary, gets confirmation (unless --yes)
/// 6. Creates a temporary directory for zkstack output
/// 7. Exports ecosystem state to temp dir (needed by zkstack)
/// 8. Copies genesis.json to temp directory
/// 9. Runs zkstack chain create pointing to temp dir
/// 10. Verifies chain files created
/// 11. Imports chain state through StateManager
/// 12. Validates imported state
pub async fn run(args: &AddArgs, context: &Context) -> Result<()> {
    ui::intro("ADI Add Chain")?;
    context.logger().debug("Starting chain addition");

    // 1. Parse and validate protocol version
    let protocol_version_str =
        resolve_protocol_version(args.protocol_version.as_ref(), context.config())?;
    let version =
        ProtocolVersion::parse(&protocol_version_str).wrap_err("Invalid protocol version")?;

    // 2. Resolve ecosystem name from args or config
    let ecosystem_name = resolve_ecosystem_name(args.ecosystem_name.as_ref(), context.config())?;

    // 3. Merge CLI args with config defaults (CLI > Config)
    let config_defaults = &context.config().ecosystem;
    let chain_config = build_chain_config(args, config_defaults);

    context
        .logger()
        .debug(&format!("Chain config: {:?}", chain_config));

    // 4. Create state manager and validate ecosystem exists
    let state_dir = &context.config().state_dir;
    #[allow(unused_variables)]
    let (state_manager, s3_control) = create_state_manager_with_s3(&ecosystem_name, context)
        .await
        .wrap_err("Failed to create state manager")?;

    // Disable auto-sync for batch operations
    #[cfg(feature = "s3")]
    if let Some(ref control) = s3_control {
        control.disable_auto_sync();
    }

    if !state_manager.exists().await? {
        return Err(eyre::eyre!(
            "Ecosystem '{}' does not exist. Run 'adi init' first.",
            ecosystem_name
        ));
    }

    // 5. Check if chain already exists
    let chain_ops = state_manager.chain(&chain_config.name);
    if chain_ops.exists().await? {
        if args.force {
            ui::warning(format!(
                "Chain '{}' already exists. Force flag set, will overwrite.",
                ui::yellow(&chain_config.name)
            ))?;
            // Delete existing chain state
            chain_ops
                .delete()
                .await
                .wrap_err("Failed to delete existing chain")?;
            ui::success("Existing chain deleted")?;
        } else {
            ui::warning(format!(
                "Chain '{}' already exists.",
                ui::yellow(&chain_config.name)
            ))?;

            // Require typing chain name to confirm deletion
            let prompt = format!(
                "Type '{}' to confirm deletion and overwrite",
                ui::green(&chain_config.name)
            );
            let user_input: String = ui::input(prompt)
                .interact()
                .wrap_err("Failed to read user input")?;

            if user_input != chain_config.name {
                return Err(eyre::eyre!(
                    "Confirmation failed: expected '{}', got '{}'",
                    chain_config.name,
                    user_input
                ));
            }

            // Delete existing chain state
            chain_ops
                .delete()
                .await
                .wrap_err("Failed to delete existing chain")?;
            ui::success("Existing chain deleted")?;
        }
    }

    // 6. Display config summary
    ui::note(
        format!("Protocol version: {}", ui::green(&version)),
        format!(
            "Ecosystem: {}\nChain: {} (ID: {})\nProver mode: {}\nBase token: {}\nEVM emulator: {}",
            ui::green(&ecosystem_name),
            ui::green(&chain_config.name),
            ui::green(chain_config.chain_id),
            ui::green(&chain_config.prover_mode),
            ui::green(&chain_config.base_token_address),
            ui::green(chain_config.evm_emulator)
        ),
    )?;

    // 7. Get confirmation unless --yes flag
    if !args.yes {
        let confirm = ui::confirm("Proceed with chain creation?")
            .initial_value(true)
            .interact()
            .wrap_err("Failed to read confirmation")?;

        if !confirm {
            ui::outro_cancel("Cancelled")?;
            return Ok(());
        }
    }

    // 8. Build zkstack chain create args
    let zkstack_args = build_chain_create_args(&chain_config);
    context
        .logger()
        .debug(&format!("zkstack args: {:?}", zkstack_args));

    // 9. Create temp directory and export ecosystem state
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
    let ecosystem_temp_path = temp_path.join(&ecosystem_name);
    ui::info("Exporting ecosystem state to temp directory...")?;
    export_ecosystem_state(&state_manager, &ecosystem_temp_path)
        .await
        .wrap_err("Failed to export ecosystem state")?;

    // 10. Copy genesis.json to ecosystem temp directory
    let genesis_src = state_dir.join(GENESIS_FILENAME);
    if !genesis_src.exists() {
        return Err(eyre::eyre!(
            "genesis.json not found at {}. Run 'adi init' first.",
            genesis_src.display()
        ));
    }
    let genesis_dst = ecosystem_temp_path.join(GENESIS_FILENAME);
    std::fs::copy(&genesis_src, &genesis_dst).wrap_err("Failed to copy genesis.json")?;

    // 11. Create toolkit runner and execute zkstack chain create
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

    // 12. Verify chain was created
    ui::info("Verifying chain files...")?;
    verify_chain_created(
        &temp_path,
        &ecosystem_name,
        &chain_config.name,
        context.logger().as_ref(),
    )
    .wrap_err("Chain verification failed")?;

    // 13. Import chain state
    ui::info("Importing chain state...")?;
    import_chain_state(
        &state_manager,
        &temp_path,
        &ecosystem_name,
        &chain_config.name,
    )
    .await
    .wrap_err("Failed to import chain state")?;

    // 14. Validate imported state
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
    #[cfg(feature = "s3")]
    if let Some(control) = s3_control {
        control.sync_now().await.wrap_err("Failed to sync to S3")?;
    }

    let chains = state_manager.list_chains().await?;
    ui::success(format!(
        "Chain added. Ecosystem now has {} chain(s)",
        chains.len()
    ))?;

    let ecosystem_path = state_dir.join(&ecosystem_name);
    ui::info(format!("Location: {}", ecosystem_path.display()))?;
    ui::outro(format!("Chain '{}' added successfully!", chain_config.name))?;

    Ok(())
}

/// Build chain config by merging CLI args with config defaults.
/// CLI args take priority over config file values.
fn build_chain_config(args: &AddArgs, defaults: &EcosystemConfig) -> ChainConfig {
    ChainConfig {
        name: args
            .chain_name
            .clone()
            .unwrap_or_else(|| defaults.chain_name.clone()),
        chain_id: args.chain_id.unwrap_or(defaults.chain_id),
        prover_mode: args
            .prover_mode
            .clone()
            .unwrap_or_else(|| defaults.prover_mode.clone()),
        base_token_address: args
            .base_token_address
            .unwrap_or(defaults.base_token_address),
        base_token_price_nominator: args
            .base_token_price_nominator
            .unwrap_or(defaults.base_token_price_nominator),
        base_token_price_denominator: args
            .base_token_price_denominator
            .unwrap_or(defaults.base_token_price_denominator),
        evm_emulator: args.evm_emulator.unwrap_or(defaults.evm_emulator),
    }
}
