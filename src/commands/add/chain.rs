//! Chain addition command implementation.

use adi_ecosystem::{validate_chain_id, validate_chain_id_unique, ChainDefaults};
use adi_funding::{normalize_rpc_url, FundingProvider};
use adi_toolkit::ProtocolVersion;

use super::AddArgs;
use crate::commands::chain_ops;
use crate::commands::helpers::{
    collect_existing_chains, create_state_manager_with_s3, resolve_ecosystem_name,
    resolve_protocol_version, resolve_rpc_url, select_chain_from_config, ChainSelection,
};
use crate::config_writer;
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
/// 6. Creates chain using shared chain_ops module
/// 7. Offers to save chain config to ~/.adi.yml
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

    // 3. Create state manager and validate ecosystem exists (needed for uniqueness checks)
    let state_dir = &context.config().state_dir;
    #[allow(unused_variables)]
    let (state_manager, s3_control) = create_state_manager_with_s3(&ecosystem_name, context)
        .await
        .wrap_err("Failed to create state manager")?;

    // Disable auto-sync for batch operations
    if let Some(ref control) = s3_control {
        control.disable_auto_sync();
    }

    let ecosystem_state_dir = state_dir.join(&ecosystem_name);
    crate::commands::state_paths::validate_and_fix_state_paths(
        &state_manager,
        &ecosystem_state_dir,
    )
    .await?;

    if !state_manager.exists().await? {
        return Err(eyre::eyre!(
            "Ecosystem '{}' does not exist. Run 'adi init' first.",
            ecosystem_name
        ));
    }

    // 4. Select chain from config (--chain takes priority, then --chain-name, else interactive)
    let chain_arg = args.chain.as_ref().or(args.chain_name.as_ref());
    let chain_selection = select_chain_from_config(chain_arg, true, context.config())?;

    // 5. Merge CLI args with config defaults (CLI > Config)
    let config_defaults = &context.config().ecosystem;
    let selected_chain = match &chain_selection {
        ChainSelection::Existing(name) => config_defaults.get_chain(name),
        ChainSelection::New(_) => None,
    };
    let chain_defaults = build_chain_defaults(args, selected_chain, &chain_selection);

    context
        .logger()
        .debug(&format!("Chain config: {:?}", chain_defaults));

    // 6. Validate chain ID uniqueness within ecosystem
    let existing_chains = collect_existing_chains(&state_manager).await?;
    // Exclude the chain being overwritten (same name) from uniqueness check
    let chains_for_id_check: Vec<_> = existing_chains
        .iter()
        .filter(|(name, _)| name != &chain_defaults.name)
        .cloned()
        .collect();
    if let Err(msg) = validate_chain_id_unique(chain_defaults.chain_id, &chains_for_id_check) {
        return Err(eyre::eyre!("{}", msg));
    }

    // 7. Validate chain ID doesn't conflict with settlement layer
    ui::info("Validating chain ID against settlement layer...")?;
    let rpc_url = resolve_rpc_url(args.rpc_url.as_ref(), context.config())?;
    let normalized_rpc = normalize_rpc_url(rpc_url.as_str());
    let provider = FundingProvider::new(&normalized_rpc)
        .wrap_err("Failed to connect to settlement layer RPC")?;
    let settlement_chain_id = provider
        .get_chain_id()
        .await
        .wrap_err("Failed to get settlement layer chain ID")?;

    if let Err(msg) = validate_chain_id(chain_defaults.chain_id, settlement_chain_id) {
        return Err(eyre::eyre!("{}", msg));
    }
    ui::success(format!(
        "Chain ID {} validated (settlement layer: {})",
        ui::green(chain_defaults.chain_id),
        ui::green(settlement_chain_id)
    ))?;

    // 8. Check if chain already exists
    let chain_state_ops = state_manager.chain(&chain_defaults.name);
    if chain_state_ops.exists().await? {
        if args.force {
            ui::warning(format!(
                "Chain '{}' already exists. Force flag set, will overwrite.",
                ui::yellow(&chain_defaults.name)
            ))?;
            // Delete existing chain state
            chain_state_ops
                .delete()
                .await
                .wrap_err("Failed to delete existing chain")?;
            ui::success("Existing chain deleted")?;
        } else {
            ui::warning(format!(
                "Chain '{}' already exists.",
                ui::yellow(&chain_defaults.name)
            ))?;

            // Require typing chain name to confirm deletion
            let prompt = format!(
                "Type '{}' to confirm deletion and overwrite",
                ui::green(&chain_defaults.name)
            );
            let user_input: String = ui::input(prompt)
                .interact()
                .wrap_err("Failed to read user input")?;

            if user_input != chain_defaults.name {
                return Err(eyre::eyre!(
                    "Confirmation failed: expected '{}', got '{}'",
                    chain_defaults.name,
                    user_input
                ));
            }

            // Delete existing chain state
            chain_state_ops
                .delete()
                .await
                .wrap_err("Failed to delete existing chain")?;
            ui::success("Existing chain deleted")?;
        }
    }

    // 9. Display config summary
    let base_token_display = chain_defaults
        .base_token_address
        .map(|a| format!("{}", a))
        .unwrap_or_else(|| "ETH".to_string());

    ui::note(
        format!("Protocol version: {}", ui::green(&version)),
        format!(
            "Ecosystem: {}\nChain: {} (ID: {})\nProver mode: {}\nBase token: {}\nEVM emulator: {}",
            ui::green(&ecosystem_name),
            ui::green(&chain_defaults.name),
            ui::green(chain_defaults.chain_id),
            ui::green(&chain_defaults.prover_mode),
            ui::green(&base_token_display),
            ui::green(chain_defaults.evm_emulator)
        ),
    )?;

    // 10. Get confirmation unless --yes flag
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

    // 11. Offer to save chain config (before Docker operations)
    config_writer::prompt_and_save_chain_config(
        &chain_defaults,
        context.config_path(),
        args.force,
    )?;

    // 12. Create chain using shared chain_ops module
    chain_ops::create_chain(
        &ecosystem_name,
        &chain_defaults,
        &state_manager,
        &s3_control,
        &version,
        context,
    )
    .await?;

    let ecosystem_path = state_dir.join(&ecosystem_name);
    ui::info(format!("Location: {}", ui::green(ecosystem_path.display())))?;
    ui::outro(format!(
        "Chain '{}' added successfully!",
        chain_defaults.name
    ))?;

    Ok(())
}

/// Build chain defaults by merging CLI args with config defaults.
/// CLI args take priority over config file values.
///
/// # Arguments
/// * `args` - CLI arguments
/// * `selected_chain` - Selected chain defaults (if existing chain from config)
/// * `chain_selection` - Chain selection result (for getting chain name)
fn build_chain_defaults(
    args: &AddArgs,
    selected_chain: Option<&ChainDefaults>,
    chain_selection: &ChainSelection,
) -> ChainDefaults {
    // Use selected chain defaults (from config) or None for new chains
    let defaults = selected_chain;

    ChainDefaults {
        name: args
            .chain_name
            .clone()
            .or_else(|| defaults.map(|c| c.name.clone()))
            .unwrap_or_else(|| chain_selection.name().to_string()),
        chain_id: args
            .chain_id
            .or_else(|| defaults.map(|c| c.chain_id))
            .unwrap_or(222),
        prover_mode: args
            .prover_mode
            .or_else(|| defaults.map(|c| c.prover_mode))
            .unwrap_or_default(),
        base_token_address: args
            .base_token_address
            .or_else(|| defaults.and_then(|c| c.base_token_address)),
        base_token_price_nominator: args
            .base_token_price_nominator
            .or_else(|| defaults.map(|c| c.base_token_price_nominator))
            .unwrap_or(1),
        base_token_price_denominator: args
            .base_token_price_denominator
            .or_else(|| defaults.map(|c| c.base_token_price_denominator))
            .unwrap_or(1),
        evm_emulator: args
            .evm_emulator
            .or_else(|| defaults.map(|c| c.evm_emulator))
            .unwrap_or(false),
        // Use blobs setting from config or default to false (calldata/L3 mode)
        blobs: defaults.map(|c| c.blobs).unwrap_or(false),
        // Copy operators, funding, ownership from selected chain if exists
        operators: defaults.map(|c| c.operators.clone()).unwrap_or_default(),
        funding: defaults.map(|c| c.funding.clone()).unwrap_or_default(),
        ownership: defaults.map(|c| c.ownership.clone()).unwrap_or_default(),
    }
}
