//! Ecosystem initialization command implementation.

use adi_ecosystem::{build_ecosystem_create_args, verify_ecosystem_created, EcosystemConfig};
use adi_toolkit::{ProtocolVersion, ToolkitRunner, GENESIS_FILENAME};

use super::EcosystemArgs;
use crate::context::Context;
use crate::error::{Result, WrapErr};

/// Execute the ecosystem initialization command.
///
/// This command:
/// 1. Validates the protocol version
/// 2. Merges CLI args with config defaults
/// 3. Builds zkstack command arguments (domain logic - no Docker knowledge)
/// 4. Resolves state directory to absolute path
/// 5. Checks genesis.json exists in state directory
/// 6. Creates toolkit runner and executes the command
/// 7. Verifies ecosystem was created (domain logic - no Docker knowledge)
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

    // 3. Build zkstack command arguments (domain logic - no Docker knowledge)
    let zkstack_args = build_ecosystem_create_args(&config);
    log::debug!("zkstack args: {:?}", zkstack_args);

    // 4. Create state directory if needed and resolve to absolute path
    let state_dir = &context.config().state_dir;
    std::fs::create_dir_all(state_dir).wrap_err("Failed to create state directory")?;

    // Docker requires absolute paths for bind mounts
    let state_dir = state_dir
        .canonicalize()
        .wrap_err("Failed to resolve state directory to absolute path")?;
    log::info!("State directory: {}", state_dir.display());

    // 5. Check genesis.json exists in state directory (required for zkSync OS)
    let genesis_path = state_dir.join(GENESIS_FILENAME);
    if !genesis_path.exists() {
        return Err(eyre::eyre!(
            "genesis.json not found in state directory.\n\
             Please place the genesis.json file at: {}\n\
             You can download it from: https://raw.githubusercontent.com/matter-labs/zksync-os-server/ec996154d7cb0f3bd2857ff015d061781a9fbbe6/genesis/genesis.json",
            genesis_path.display()
        ));
    }
    log::info!("Genesis file: {}", genesis_path.display());

    // 6. Create toolkit runner and execute
    log::info!("Connecting to Docker...");
    let runner = ToolkitRunner::new()
        .await
        .wrap_err("Failed to create toolkit runner")?;

    log::info!("Running zkstack ecosystem create...");
    let args_refs: Vec<&str> = zkstack_args.iter().map(String::as_str).collect();

    let exit_code = runner
        .run_zkstack(&args_refs, &state_dir, &version.to_semver())
        .await
        .wrap_err("Failed to run zkstack ecosystem create")?;

    if exit_code != 0 {
        return Err(eyre::eyre!(
            "zkstack ecosystem create failed with exit code {}",
            exit_code
        ));
    }

    // 7. Verify ecosystem was created (domain logic - no Docker knowledge)
    log::info!("Verifying ecosystem files...");
    verify_ecosystem_created(&state_dir, &config).wrap_err("Ecosystem verification failed")?;

    log::info!("Ecosystem '{}' initialized successfully!", config.name);
    log::info!("Location: {}/{}", state_dir.display(), config.name);

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
