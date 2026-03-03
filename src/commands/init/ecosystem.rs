//! Ecosystem initialization command implementation.

use adi_ecosystem::{
    build_ecosystem_create_args, normalize_name, validate_chain_id, verify_ecosystem_created,
    EcosystemConfig,
};
use adi_funding::FundingProvider;
use adi_state::{import_ecosystem_state, StateManager};
use adi_toolkit::{ProtocolVersion, ToolkitRunner, GENESIS_FILENAME};
use adi_types::{Wallet, Wallets};
use alloy_signer_local::PrivateKeySigner;
use secrecy::{ExposeSecret, SecretString};
use std::sync::Arc;
use tempfile::TempDir;

use super::InitArgs;
use crate::commands::helpers::{
    create_state_manager_with_s3, resolve_protocol_version, resolve_rpc_url,
};
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
/// 5. Copies genesis.json to temp directory
/// 6. Runs zkstack ecosystem create pointing to temp dir
/// 7. Verifies ecosystem was created in temp dir
/// 8. Imports state from temp dir through StateManager to configured backend
/// 9. Validates imported state
/// 10. TempDir is automatically cleaned up on drop
pub async fn run(args: &InitArgs, context: &Context) -> Result<()> {
    ui::intro("ADI Init")?;
    context.logger().debug("Starting ecosystem initialization");

    // 1. Parse and validate protocol version
    let protocol_version_str =
        resolve_protocol_version(args.protocol_version.as_ref(), context.config())?;
    let version =
        ProtocolVersion::parse(&protocol_version_str).wrap_err("Invalid protocol version")?;
    // 2. Merge CLI args with config defaults (CLI > Config)
    let config_defaults = &context.config().ecosystem;
    let config = build_ecosystem_config(args, config_defaults);

    // 3. Validate chain ID doesn't conflict with settlement layer
    ui::info("Validating chain ID against settlement layer...")?;
    let rpc_url = resolve_rpc_url(args.rpc_url.as_ref(), context.config())?;
    let provider = FundingProvider::new(rpc_url.as_str())
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
        config.chain_id, settlement_chain_id
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
        // Show files that will be deleted
        let files = state_manager.list_state_files();
        let file_list: String = files
            .iter()
            .map(|f| format!("  - {}", f))
            .collect::<Vec<_>>()
            .join("\n");
        ui::warning(format!(
            "Ecosystem '{}' already exists at {}",
            ui::yellow(&config.name),
            ui::yellow(ecosystem_path.display())
        ))?;
        ui::note("Files to be deleted", file_list)?;

        if args.force {
            ui::info("Force flag set, skipping confirmation")?;
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

    // 6. Check genesis.json exists in state directory, download if missing
    std::fs::create_dir_all(state_dir).wrap_err("Failed to create state directory")?;

    let genesis_src = state_dir.join(GENESIS_FILENAME);
    if !genesis_src.exists() {
        let genesis_url = version.genesis_url();
        ui::info(format!("Downloading genesis.json from {genesis_url}..."))?;

        download_genesis(&genesis_url, &genesis_src)
            .await
            .wrap_err("Failed to download genesis.json")?;

        ui::success("Genesis file downloaded")?;
    } else {
        context
            .logger()
            .debug("genesis.json already exists, skipping download");
    }

    let genesis_dst = temp_path.join(GENESIS_FILENAME);
    std::fs::copy(&genesis_src, &genesis_dst)
        .wrap_err("Failed to copy genesis.json to temp dir")?;
    ui::success("Genesis file copied to temp directory")?;

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
    // zkstack normalizes names (- → _), so we need to use the same normalization
    let ecosystem_name = normalize_name(&config.name);
    let chain_name = normalize_name(&config.chain_name);

    ui::info(format!("State directory: {}", state_dir.display()))?;
    ui::info("Importing ecosystem state through backend...")?;
    import_ecosystem_state(&state_manager, &temp_path, &ecosystem_name, &chain_name)
        .await
        .wrap_err("Failed to import ecosystem state")?;

    // 9.5. Override operator keys if configured
    apply_operator_key_overrides(&state_manager, &chain_name, args, context).await?;

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

    // Sync to S3 once at the end (if enabled)
    if let Some(control) = s3_control {
        control
            .sync_now()
            .await
            .wrap_err("Failed to sync state to S3")?;
    }

    ui::info(format!("Location: {}", ecosystem_path.display()))?;
    ui::outro(format!(
        "Ecosystem '{}' initialized successfully!",
        config.name
    ))?;

    // TempDir is automatically cleaned up when dropped
    Ok(())
}

/// Build ecosystem config by merging CLI args with config defaults.
/// CLI args take priority over config file values.
fn build_ecosystem_config(args: &InitArgs, defaults: &EcosystemConfig) -> EcosystemConfig {
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
            .unwrap_or(defaults.base_token_address),
        base_token_price_nominator: args
            .base_token_price_nominator
            .unwrap_or(defaults.base_token_price_nominator),
        base_token_price_denominator: args
            .base_token_price_denominator
            .unwrap_or(defaults.base_token_price_denominator),
        evm_emulator: args.evm_emulator.unwrap_or(defaults.evm_emulator),
        l3: defaults.l3,
        rpc_url: args.rpc_url.clone().or_else(|| defaults.rpc_url.clone()),
    }
}

/// Download genesis.json from the given URL to the destination path.
async fn download_genesis(url: &str, dest: &std::path::Path) -> Result<()> {
    let response = reqwest::get(url)
        .await
        .wrap_err("Failed to fetch genesis.json")?;

    if !response.status().is_success() {
        return Err(eyre::eyre!(
            "Failed to download genesis.json: HTTP {}",
            response.status()
        ));
    }

    let content = response
        .bytes()
        .await
        .wrap_err("Failed to read response body")?;

    std::fs::write(dest, &content).wrap_err("Failed to write genesis.json to disk")?;

    Ok(())
}

/// Apply predefined operator key overrides after ecosystem import.
///
/// If operator keys are configured via CLI args, ENV, or config file,
/// this function updates the generated wallets with the predefined keys.
/// Priority: CLI args > Config (ENV already merged by config crate).
async fn apply_operator_key_overrides(
    state_manager: &StateManager,
    chain_name: &str,
    args: &InitArgs,
    context: &Context,
) -> Result<()> {
    let config_keys = &context.config().operator_keys;

    // Build Wallets with only operator keys set
    // Priority: CLI args > Config (ENV already merged by config crate)
    let partial = Wallets {
        operator: key_to_wallet(args.operator_key.as_ref().or(config_keys.operator.as_ref()))?,
        blob_operator: key_to_wallet(
            args.blob_operator_key
                .as_ref()
                .or(config_keys.blob_operator.as_ref()),
        )?,
        prove_operator: key_to_wallet(
            args.prove_operator_key
                .as_ref()
                .or(config_keys.prove_operator.as_ref()),
        )?,
        execute_operator: key_to_wallet(
            args.execute_operator_key
                .as_ref()
                .or(config_keys.execute_operator.as_ref()),
        )?,
        ..Default::default()
    };

    // Check if any overrides exist
    let has_overrides = partial.operator.is_some()
        || partial.blob_operator.is_some()
        || partial.prove_operator.is_some()
        || partial.execute_operator.is_some();

    if has_overrides {
        ui::info("Applying predefined operator keys...")?;

        // Update ecosystem-level wallets (uses existing merge_wallets)
        state_manager
            .ecosystem()
            .update_wallets(&partial)
            .await
            .wrap_err("Failed to update ecosystem operator keys")?;

        // Update chain-level wallets
        state_manager
            .chain(chain_name)
            .update_wallets(&partial)
            .await
            .wrap_err("Failed to update chain operator keys")?;

        ui::success("Operator keys updated")?;
    }

    Ok(())
}

/// Convert private key to Wallet (derive address from key).
fn key_to_wallet(key: Option<&SecretString>) -> Result<Option<Wallet>> {
    key.map(wallet_from_private_key).transpose()
}

/// Create a Wallet from a private key, deriving the address.
fn wallet_from_private_key(key: &SecretString) -> Result<Wallet> {
    let signer: PrivateKeySigner = key
        .expose_secret()
        .parse()
        .wrap_err("Invalid private key")?;

    Ok(Wallet::new(signer.address(), key.clone()))
}
