//! Ecosystem deployment command — main entry point.
//!
//! This command:
//! 1. Funds ecosystem and chain wallets
//! 2. Deploys ecosystem contracts via zkstack
//! 3. Configures validator roles for operators
//! 4. Handles ownership acceptance and transfer (config-driven)

pub use super::args::DeployArgs;

use adi_ecosystem::validate_chain_id;
use adi_funding::{is_localhost_rpc, normalize_rpc_url, FundingProvider};
use adi_state::StateManager;

use crate::commands::helpers::{
    create_state_manager_with_s3, resolve_rpc_url, select_chain_from_state,
};
use crate::context::Context;
use crate::error::{Result, WrapErr};
use crate::ui;

use adi_ecosystem::PubdataMode;

use super::da_config::resolve_pubdata_mode;
use super::funding::{run_anvil_funding, run_production_funding};

/// Execute the ecosystem deploy command.
pub async fn run(args: DeployArgs, context: &Context) -> Result<()> {
    ui::intro("ADI Deploy")?;
    context.logger().debug("Starting ecosystem deployment");

    let ecosystem_name = resolve_ecosystem_name(&args, context)?;

    let state_manager = create_state_manager(&ecosystem_name, context).await?;
    validate_ecosystem_exists(&state_manager, &ecosystem_name).await?;

    let chain_name =
        select_chain_from_state(args.chain_name.as_ref(), &state_manager, &ecosystem_name).await?;

    if args.skip_funding {
        ui::info("Skipping wallet funding (--skip-funding)")?;
        ui::outro("Ecosystem deployment complete (funding skipped)")?;
        return Ok(());
    }

    let rpc_url = resolve_rpc_url(args.rpc_url.as_ref(), context.config())?;

    // Validate chain ID doesn't conflict with settlement layer
    ui::info("Validating chain ID against settlement layer...")?;
    let chain_metadata = state_manager
        .chain(&chain_name)
        .metadata()
        .await
        .wrap_err("Failed to load chain metadata")?;

    let normalized_rpc = normalize_rpc_url(rpc_url.as_str());
    let validation_provider = FundingProvider::new(&normalized_rpc)
        .wrap_err("Failed to connect to settlement layer for validation")?;
    let settlement_chain_id = validation_provider
        .get_chain_id()
        .await
        .wrap_err("Failed to get settlement layer chain ID")?;

    if let Err(msg) = validate_chain_id(chain_metadata.chain_id, settlement_chain_id) {
        return Err(eyre::eyre!("{}", msg));
    }
    ui::success(format!(
        "Chain ID {} validated (settlement layer: {})",
        ui::green(chain_metadata.chain_id),
        ui::green(settlement_chain_id)
    ))?;

    let pubdata_mode = resolve_pubdata_mode(&args, context, &chain_name);
    let chain_type = match pubdata_mode {
        PubdataMode::Blobs => "Blobs (rollup)",
        PubdataMode::Calldata => "Calldata (rollup)",
        PubdataMode::CustomDa => "Custom DA (external)",
    };
    ui::note(
        "Deployment target",
        format!(
            "Ecosystem: {}\nChain: {} ({})\nSettlement layer RPC: {}",
            ui::green(&ecosystem_name),
            ui::green(&chain_name),
            ui::green(chain_type),
            ui::green(&rpc_url)
        ),
    )?;

    let is_anvil = is_localhost_rpc(rpc_url.as_str()) && args.funder_key.is_none();
    if is_anvil {
        return run_anvil_funding(
            &args,
            context,
            &state_manager,
            &ecosystem_name,
            &chain_name,
            &rpc_url,
        )
        .await;
    }

    run_production_funding(
        &args,
        context,
        &state_manager,
        &ecosystem_name,
        &chain_name,
        &rpc_url,
    )
    .await
}

fn resolve_ecosystem_name(args: &DeployArgs, context: &Context) -> Result<String> {
    args.ecosystem_name
        .clone()
        .or_else(|| Some(context.config().ecosystem.name.clone()))
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            eyre::eyre!("Ecosystem name required: use --ecosystem-name or set in config")
        })
}

async fn create_state_manager(ecosystem_name: &str, context: &Context) -> Result<StateManager> {
    let (state_manager, _control) = create_state_manager_with_s3(ecosystem_name, context).await?;
    let state_dir = context.config().state_dir.join(ecosystem_name);
    crate::commands::state_paths::validate_and_fix_state_paths(&state_manager, &state_dir).await?;
    Ok(state_manager)
}

async fn validate_ecosystem_exists(
    state_manager: &StateManager,
    ecosystem_name: &str,
) -> Result<()> {
    if !state_manager.exists().await? {
        return Err(eyre::eyre!(
            "Ecosystem '{}' not found. Run 'adi init' first.",
            ecosystem_name
        ));
    }
    Ok(())
}
