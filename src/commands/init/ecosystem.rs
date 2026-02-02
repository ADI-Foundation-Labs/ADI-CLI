//! Ecosystem initialization command implementation.

use adi_ecosystem::{build_ecosystem_create_args, verify_ecosystem_created, EcosystemConfig};
use adi_state::{import_ecosystem_state, StateManager};
use adi_toolkit::{ProtocolVersion, ToolkitRunner, GENESIS_FILENAME};
use dialoguer::Input;
use tempfile::TempDir;

use super::EcosystemArgs;
use crate::context::Context;
use crate::error::{Result, WrapErr};

/// Execute the ecosystem initialization command.
///
/// This command:
/// 1. Validates the protocol version
/// 2. Merges CLI args with config defaults
/// 3. Checks if ecosystem already exists (prompts for confirmation to reinitialize)
/// 4. Creates a temporary directory for zkstack output
/// 5. Copies genesis.json to temp directory
/// 6. Runs zkstack ecosystem create pointing to temp dir
/// 7. Verifies ecosystem was created in temp dir
/// 8. Imports state from temp dir through StateManager to configured backend
/// 9. Validates imported state
/// 10. TempDir is automatically cleaned up on drop
pub async fn run(args: &EcosystemArgs, context: &Context) -> Result<()> {
    log::debug!("Starting ecosystem initialization");

    // 1. Parse and validate protocol version
    let version =
        ProtocolVersion::parse(&args.protocol_version).wrap_err("Invalid protocol version")?;
    log::info!("Protocol version: {}", version);

    // 2. Merge CLI args with config defaults (CLI > Config)
    let config_defaults = &context.config().ecosystem;
    let config = build_ecosystem_config(args, config_defaults);

    log::info!("Ecosystem: {}", config.name);
    log::info!("  L1 network: {}", config.l1_network);
    log::info!("  Chain: {} (ID: {})", config.chain_name, config.chain_id);
    log::info!("  Prover mode: {}", config.prover_mode);
    log::debug!("Full ecosystem config: {:?}", config);

    // 3. Check if ecosystem state already exists
    let state_dir = &context.config().state_dir;
    let ecosystem_path = state_dir.join(&config.name);
    let state_manager =
        StateManager::with_backend_type(context.config().state_backend.clone(), &ecosystem_path);

    if state_manager.exists().await? {
        // Show files that will be deleted
        let files = state_manager.list_state_files();
        log::warn!(
            "Ecosystem '{}' already exists at {}",
            config.name,
            ecosystem_path.display()
        );
        log::warn!("The following files will be deleted:");
        for file in &files {
            log::warn!("  - {}", file);
        }

        // Require typing ecosystem name to confirm deletion
        let prompt = format!(
            "Type '{}' to confirm deletion and reinitialize",
            config.name
        );
        let input: String = Input::new()
            .with_prompt(prompt)
            .interact_text()
            .wrap_err("Failed to read user input")?;

        if input != config.name {
            return Err(eyre::eyre!(
                "Confirmation failed: expected '{}', got '{}'",
                config.name,
                input
            ));
        }

        log::info!("Deleting existing ecosystem state...");
        state_manager
            .delete_all()
            .await
            .wrap_err("Failed to delete existing ecosystem state")?;
        log::info!("Existing state deleted");
    }

    // 4. Build zkstack command arguments (domain logic - no Docker knowledge)
    let zkstack_args = build_ecosystem_create_args(&config);
    log::debug!("zkstack args: {:?}", zkstack_args);

    // 5. Create temp directory for zkstack output
    let temp_dir = TempDir::new().wrap_err("Failed to create temporary directory")?;
    let temp_path = temp_dir
        .path()
        .canonicalize()
        .wrap_err("Failed to resolve temp directory to absolute path")?;
    log::debug!("Using temp directory: {}", temp_path.display());

    // 6. Check genesis.json exists in state directory and copy to temp
    std::fs::create_dir_all(state_dir).wrap_err("Failed to create state directory")?;

    let genesis_src = state_dir.join(GENESIS_FILENAME);
    if !genesis_src.exists() {
        return Err(eyre::eyre!(
            "genesis.json not found in state directory.\n\
             Please place the genesis.json file at: {}\n\
             You can download it from: https://raw.githubusercontent.com/matter-labs/zksync-os-server/48650acecd1182c56c0f6d86f3c471f8d72159c6/genesis/genesis.json",
            genesis_src.display()
        ));
    }

    let genesis_dst = temp_path.join(GENESIS_FILENAME);
    std::fs::copy(&genesis_src, &genesis_dst)
        .wrap_err("Failed to copy genesis.json to temp dir")?;
    log::info!("Genesis file copied to temp directory");

    // 7. Create toolkit runner and execute pointing to temp dir
    log::info!("Connecting to Docker...");
    let runner = ToolkitRunner::new()
        .await
        .wrap_err("Failed to create toolkit runner")?;

    log::info!("Running zkstack ecosystem create...");
    let args_refs: Vec<&str> = zkstack_args.iter().map(String::as_str).collect();

    let exit_code = runner
        .run_zkstack(&args_refs, &temp_path, &version.to_semver())
        .await
        .wrap_err("Failed to run zkstack ecosystem create")?;

    if exit_code != 0 {
        return Err(eyre::eyre!(
            "zkstack ecosystem create failed with exit code {}",
            exit_code
        ));
    }

    // 8. Verify ecosystem was created in temp dir
    log::info!("Verifying ecosystem files...");
    verify_ecosystem_created(&temp_path, &config).wrap_err("Ecosystem verification failed")?;

    // 9. Import state from temp dir through StateManager
    log::info!("State directory: {}", state_dir.display());
    log::info!("Importing ecosystem state through backend...");
    import_ecosystem_state(&state_manager, &temp_path, &config.name, &config.chain_name)
        .await
        .wrap_err("Failed to import ecosystem state")?;

    // 10. Validate imported state
    log::info!("Validating imported state...");
    let metadata = state_manager
        .ecosystem()
        .metadata()
        .await
        .wrap_err("Failed to read ecosystem metadata")?;

    log::debug!("Ecosystem metadata: name={}", metadata.name);

    let chain_metadata = state_manager
        .chain(&config.chain_name)
        .metadata()
        .await
        .wrap_err("Failed to read chain metadata")?;

    log::debug!(
        "Chain '{}' validated: chain_id={}",
        config.chain_name,
        chain_metadata.chain_id
    );

    let chains = state_manager
        .list_chains()
        .await
        .wrap_err("Failed to list chains")?;

    log::info!("State validated: {} chain(s) found", chains.len());
    log::info!("Ecosystem '{}' initialized successfully!", config.name);
    log::info!("Location: {}", ecosystem_path.display());

    // TempDir is automatically cleaned up when dropped
    Ok(())
}

/// Build ecosystem config by merging CLI args with config defaults.
/// CLI args take priority over config file values.
fn build_ecosystem_config(args: &EcosystemArgs, defaults: &EcosystemConfig) -> EcosystemConfig {
    EcosystemConfig {
        name: args
            .ecosystem_name
            .clone()
            .unwrap_or_else(|| defaults.name.clone()),
        l1_network: args
            .l1_network
            .clone()
            .unwrap_or_else(|| defaults.l1_network.clone()),
        chain_name: args
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
            .clone()
            .unwrap_or_else(|| defaults.base_token_address.clone()),
        base_token_price_nominator: args
            .base_token_price_nominator
            .unwrap_or(defaults.base_token_price_nominator),
        base_token_price_denominator: args
            .base_token_price_denominator
            .unwrap_or(defaults.base_token_price_denominator),
        evm_emulator: args.evm_emulator.unwrap_or(defaults.evm_emulator),
    }
}
