//! Ecosystem initialization command implementation.

use adi_ecosystem::{
    build_ecosystem_create_args, normalize_name, validate_chain_id, verify_ecosystem_created,
    ChainDefaults, EcosystemConfig, EcosystemDefaults,
};
use adi_funding::{normalize_rpc_url, FundingProvider};
use adi_state::import_ecosystem_state;
use adi_toolkit::{ProtocolVersion, ToolkitRunner};
use std::sync::Arc;
use tempfile::TempDir;

use super::InitArgs;
use crate::commands::chain_ops;
use crate::commands::chain_prompts::{prompt_chain_defaults, PartialChainDefaults};
use crate::commands::helpers::{
    collect_existing_chains, create_state_manager_with_s3, resolve_protocol_version,
    resolve_rpc_url, select_chain_from_config, ChainSelection,
};
use crate::config_writer;
use crate::context::Context;
use crate::error::{Result, WrapErr};
use crate::ui;

/// Execute the ecosystem initialization command.
///
/// This command:
/// 1. Validates the protocol version
/// 2. Merges CLI args with config defaults
/// 3. Checks if ecosystem already exists (prompts for confirmation to reinitialize)
/// 4. Creates a temporary directory for zkstack output
/// 5. Runs zkstack ecosystem create pointing to temp dir
/// 6. Verifies ecosystem was created in temp dir
/// 7. Imports state from temp dir through StateManager to configured backend
/// 8. Validates imported state
/// 9. TempDir is automatically cleaned up on drop
pub async fn run(args: &InitArgs, context: &Context) -> Result<()> {
    ui::intro("ADI Init")?;
    context.logger().debug("Starting ecosystem initialization");

    // 1. Parse and validate protocol version
    let protocol_version_str =
        resolve_protocol_version(args.protocol_version.as_ref(), context.config())?;
    let version =
        ProtocolVersion::parse(&protocol_version_str).wrap_err("Invalid protocol version")?;

    // 2. Select chain from config (--chain takes priority, then --chain-name, else interactive)
    let chain_arg = args.chain.as_ref().or(args.chain_name.as_ref());
    let chain_selection = select_chain_from_config(chain_arg, true, context.config())?;

    // 3. Merge CLI args with config defaults (CLI > Config)
    let config_defaults = &context.config().ecosystem;
    let selected_chain = match &chain_selection {
        ChainSelection::Existing(name) => config_defaults.get_chain(name),
        ChainSelection::New(_) => None,
    };
    let config = build_ecosystem_config(args, config_defaults, selected_chain, &chain_selection);

    // 4. Validate chain ID doesn't conflict with settlement layer
    ui::info("Validating chain ID against settlement layer...")?;
    let rpc_url = resolve_rpc_url(args.rpc_url.as_ref(), context.config())?;
    let normalized_rpc = normalize_rpc_url(rpc_url.as_str());
    let provider = FundingProvider::new(&normalized_rpc)
        .wrap_err("Failed to connect to settlement layer RPC")?;
    let settlement_chain_id = provider
        .get_chain_id()
        .await
        .wrap_err("Failed to get settlement layer chain ID")?;

    if let Err(msg) = validate_chain_id(config.chain_id, settlement_chain_id) {
        return Err(eyre::eyre!("{}", msg));
    }
    ui::success(format!(
        "Chain ID {} validated (settlement layer: {})",
        ui::green(config.chain_id),
        ui::green(settlement_chain_id)
    ))?;

    ui::note(
        format!("Protocol version: {}", ui::green(&version)),
        format!(
            "Ecosystem: {}\nL1 network: {}\nChain: {} (ID: {})\nProver mode: {}",
            ui::green(&config.name),
            ui::green(&config.l1_network),
            ui::green(&config.chain_name),
            ui::green(config.chain_id),
            ui::green(&config.prover_mode)
        ),
    )?;
    context
        .logger()
        .debug(&format!("Full ecosystem config: {:?}", config));

    // 3. Check if ecosystem state already exists
    let state_dir = &context.config().state_dir;
    let ecosystem_path = state_dir.join(&config.name);
    #[allow(unused_variables)]
    let (state_manager, s3_control) = create_state_manager_with_s3(&config.name, context)
        .await
        .wrap_err("Failed to create state manager")?;

    // Disable auto-sync for batch import operations
    if let Some(ref control) = s3_control {
        control.disable_auto_sync();
    }

    if state_manager.exists().await? {
        // Ecosystem exists - present options to user
        ui::warning(format!(
            "Ecosystem '{}' already exists at {}",
            ui::yellow(&config.name),
            ui::yellow(ecosystem_path.display())
        ))?;

        // Define action options
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        enum ExistingEcosystemAction {
            AddChain,
            Reinitialize,
            Cancel,
        }
        // With -y flag, auto-select reinitialize
        let action = if args.yes {
            ui::info("Auto-selecting reinitialize (--yes flag)")?;
            ExistingEcosystemAction::Reinitialize
        } else {
            let items = vec![
                (
                    ExistingEcosystemAction::AddChain,
                    "Add a new chain to this ecosystem",
                    "Keep existing ecosystem, add another chain".to_string(),
                ),
                (
                    ExistingEcosystemAction::Reinitialize,
                    "Delete and reinitialize ecosystem",
                    "Remove all existing data and start fresh".to_string(),
                ),
                (
                    ExistingEcosystemAction::Cancel,
                    "Cancel",
                    "Exit without making changes".to_string(),
                ),
            ];

            ui::select("What would you like to do?")
                .items(&items)
                .interact()
                .wrap_err("Selection cancelled")?
        };

        match action {
            ExistingEcosystemAction::AddChain => {
                // Collect existing chains BEFORE prompting (for inline validation)
                let existing_chains = collect_existing_chains(&state_manager).await?;

                // Build partial config from CLI args for prompts
                // Include chain name from selection to avoid asking twice
                let mut partial = build_partial_chain_defaults(args);
                partial.name = Some(chain_selection.name().to_string());

                // Prompt for complete chain config (validates name/ID uniqueness inline)
                let chain_defaults = prompt_chain_defaults(&partial, &existing_chains)?;

                // Validate chain ID doesn't conflict with settlement layer
                if let Err(msg) = validate_chain_id(chain_defaults.chain_id, settlement_chain_id) {
                    return Err(eyre::eyre!("{}", msg));
                }

                // Display config summary
                ui::note(
                    format!("Adding chain: {}", ui::green(&chain_defaults.name)),
                    format!(
                        "Chain ID: {}\nProver mode: {}\nEVM emulator: {}",
                        ui::green(chain_defaults.chain_id),
                        ui::green(&chain_defaults.prover_mode),
                        ui::green(chain_defaults.evm_emulator)
                    ),
                )?;

                // Offer to save to config (before Docker operations)
                config_writer::prompt_and_save_chain_config(
                    &chain_defaults,
                    context.config_path(),
                    args.yes,
                )?;

                // Run chain creation
                chain_ops::create_chain(
                    &config.name,
                    &chain_defaults,
                    &state_manager,
                    &s3_control,
                    &version,
                    context,
                )
                .await?;

                ui::info(format!("Location: {}", ui::green(ecosystem_path.display())))?;
                ui::outro(format!(
                    "Chain '{}' added to ecosystem '{}' successfully!",
                    chain_defaults.name, config.name
                ))?;

                return Ok(());
            }
            ExistingEcosystemAction::Reinitialize => {
                // Show files that will be deleted
                let files = state_manager.list_state_files().await;
                let file_list: String = files
                    .iter()
                    .map(|f| format!("  - {}", f))
                    .collect::<Vec<_>>()
                    .join("\n");
                ui::note("Files to be deleted", file_list)?;

                if args.force || args.yes {
                    ui::info("Skipping confirmation (--force or --yes flag)")?;
                } else {
                    // Require typing ecosystem name to confirm deletion
                    let prompt = format!(
                        "Type '{}' to confirm deletion and reinitialize",
                        ui::green(&config.name)
                    );
                    let user_input: String = ui::input(prompt)
                        .interact()
                        .wrap_err("Failed to read user input")?;

                    if user_input != config.name {
                        return Err(eyre::eyre!(
                            "Confirmation failed: expected '{}', got '{}'",
                            config.name,
                            user_input
                        ));
                    }
                }

                ui::info("Deleting existing ecosystem state...")?;
                state_manager
                    .delete_all()
                    .await
                    .wrap_err("Failed to delete existing ecosystem state")?;
                ui::success("Existing state deleted")?;
                // Continue with normal init flow below
            }
            ExistingEcosystemAction::Cancel => {
                ui::outro_cancel("Cancelled")?;
                return Ok(());
            }
        }
    }

    // 4. Build zkstack command arguments (domain logic - no Docker knowledge)
    let zkstack_args = build_ecosystem_create_args(&config);
    context
        .logger()
        .debug(&format!("zkstack args: {:?}", zkstack_args));

    // 5. Create temp directory for zkstack output
    let temp_dir = TempDir::new().wrap_err("Failed to create temporary directory")?;
    let temp_path = temp_dir
        .path()
        .canonicalize()
        .wrap_err("Failed to resolve temp directory to absolute path")?;
    context
        .logger()
        .debug(&format!("Using temp directory: {}", temp_path.display()));

    // 6. Create state directory
    std::fs::create_dir_all(state_dir).wrap_err("Failed to create state directory")?;

    // 7. Create toolkit runner and execute pointing to temp dir
    ui::info("Connecting to Docker...")?;
    let runner = ToolkitRunner::with_config_and_logger(
        context.toolkit_config(),
        Arc::clone(context.logger()),
    )
    .await
    .wrap_err("Failed to create toolkit runner")?;

    ui::info("Running zkstack ecosystem create...")?;
    let args_refs: Vec<&str> = zkstack_args.iter().map(String::as_str).collect();

    let exit_code = runner
        .run_zkstack(&args_refs, &temp_path, state_dir, &version.to_semver())
        .await
        .wrap_err("Failed to run zkstack ecosystem create")?;

    if exit_code != 0 {
        return Err(eyre::eyre!(
            "zkstack ecosystem create failed with exit code {}",
            exit_code
        ));
    }

    // 8. Verify ecosystem was created in temp dir
    ui::info("Verifying ecosystem files...")?;
    verify_ecosystem_created(&temp_path, &config, context.logger().as_ref())
        .wrap_err("Ecosystem verification failed")?;

    // 9. Import state from temp dir through StateManager
    // zkstack normalizes ecosystem names (- → _), so we need to use the same normalization
    let ecosystem_name = normalize_name(&config.name);
    ui::info(format!(
        "State directory: {}",
        ui::green(state_dir.display())
    ))?;
    ui::info("Importing ecosystem state through backend...")?;
    import_ecosystem_state(
        &state_manager,
        &temp_path,
        &ecosystem_name,
        &config.chain_name,
    )
    .await
    .wrap_err("Failed to import ecosystem state")?;

    // 10. Validate imported state
    ui::info("Validating imported state...")?;
    let metadata = state_manager
        .ecosystem()
        .metadata()
        .await
        .wrap_err("Failed to read ecosystem metadata")?;

    context
        .logger()
        .debug(&format!("Ecosystem metadata: name={}", metadata.name));

    let chain_metadata = state_manager
        .chain(&config.chain_name)
        .metadata()
        .await
        .wrap_err("Failed to read chain metadata")?;

    context.logger().debug(&format!(
        "Chain '{}' validated: chain_id={}",
        config.chain_name, chain_metadata.chain_id
    ));

    let chains = state_manager
        .list_chains()
        .await
        .wrap_err("Failed to list chains")?;

    ui::success(format!("State validated: {} chain(s) found", chains.len()))?;

    // Offer to save chain config to config file
    let chain_defaults = ecosystem_config_to_chain_defaults(&config);
    config_writer::prompt_and_save_chain_config(&chain_defaults, context.config_path(), args.yes)?;

    // Sync to S3 once at the end (if enabled)
    if let Some(control) = s3_control {
        control
            .sync_now()
            .await
            .wrap_err("Failed to sync state to S3")?;
    }

    ui::info(format!("Location: {}", ui::green(ecosystem_path.display())))?;
    ui::outro(format!(
        "Ecosystem '{}' initialized successfully!",
        config.name
    ))?;

    // TempDir is automatically cleaned up when dropped
    Ok(())
}

/// Build ecosystem config by merging CLI args with config defaults.
/// CLI args take priority over config file values.
///
/// # Arguments
/// * `args` - CLI arguments
/// * `defaults` - Ecosystem defaults from config
/// * `selected_chain` - Selected chain defaults (if existing chain from config)
/// * `chain_selection` - Chain selection result (for getting chain name)
fn build_ecosystem_config(
    args: &InitArgs,
    defaults: &EcosystemDefaults,
    selected_chain: Option<&adi_ecosystem::ChainDefaults>,
    chain_selection: &ChainSelection,
) -> EcosystemConfig {
    // Use selected chain defaults (from config) or None for new chains
    let chain_defaults = selected_chain;

    EcosystemConfig {
        name: args
            .ecosystem_name
            .clone()
            .unwrap_or_else(|| defaults.name.clone()),
        l1_network: args.l1_network.unwrap_or(defaults.l1_network),
        chain_name: args
            .chain_name
            .clone()
            .or_else(|| chain_defaults.map(|c| c.name.clone()))
            .unwrap_or_else(|| chain_selection.name().to_string()),
        chain_id: args
            .chain_id
            .or_else(|| chain_defaults.map(|c| c.chain_id))
            .unwrap_or(222),
        prover_mode: args
            .prover_mode
            .or_else(|| chain_defaults.map(|c| c.prover_mode))
            .unwrap_or_default(),
        base_token_address: args
            .base_token_address
            .or_else(|| chain_defaults.and_then(|c| c.base_token_address))
            .unwrap_or(adi_types::ETH_TOKEN_ADDRESS),
        base_token_price_nominator: args
            .base_token_price_nominator
            .or_else(|| chain_defaults.map(|c| c.base_token_price_nominator))
            .unwrap_or(1),
        base_token_price_denominator: args
            .base_token_price_denominator
            .or_else(|| chain_defaults.map(|c| c.base_token_price_denominator))
            .unwrap_or(1),
        evm_emulator: args
            .evm_emulator
            .or_else(|| chain_defaults.map(|c| c.evm_emulator))
            .unwrap_or(false),
        rpc_url: args.rpc_url.clone().or_else(|| defaults.rpc_url.clone()),
    }
}

/// Build partial chain defaults from CLI arguments.
///
/// This extracts chain-related arguments from InitArgs to create
/// a PartialChainDefaults for the interactive prompts.
fn build_partial_chain_defaults(args: &InitArgs) -> PartialChainDefaults {
    PartialChainDefaults {
        name: args.chain_name.clone(),
        chain_id: args.chain_id,
        prover_mode: args.prover_mode,
        base_token_address: args.base_token_address,
        base_token_price_nominator: args.base_token_price_nominator,
        base_token_price_denominator: args.base_token_price_denominator,
        evm_emulator: args.evm_emulator,
        operator: args.operator,
        prove_operator: args.prove_operator,
        execute_operator: args.execute_operator,
        // These are not in InitArgs, so leave as None
        operator_eth: None,
        prove_operator_eth: None,
        execute_operator_eth: None,
        new_owner: None,
    }
}

/// Convert ecosystem config to chain defaults for config file storage.
///
/// This extracts the chain-specific fields from EcosystemConfig to create
/// a ChainDefaults struct for saving to the config file.
fn ecosystem_config_to_chain_defaults(config: &EcosystemConfig) -> ChainDefaults {
    ChainDefaults {
        name: config.chain_name.clone(),
        chain_id: config.chain_id,
        prover_mode: config.prover_mode,
        base_token_address: if config.base_token_address == adi_types::ETH_TOKEN_ADDRESS {
            None
        } else {
            Some(config.base_token_address)
        },
        base_token_price_nominator: config.base_token_price_nominator,
        base_token_price_denominator: config.base_token_price_denominator,
        evm_emulator: config.evm_emulator,
        blobs: false, // Default to calldata mode (L3)
        operators: Default::default(),
        funding: Default::default(),
        ownership: Default::default(),
    }
}
