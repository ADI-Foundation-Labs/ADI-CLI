//! v31 command-layer dispatch: resolve inputs, build the orchestrator, run its phases.

use std::sync::Arc;

use alloy_primitives::B256;

use adi_docker::transform_url_for_container;
use adi_toolkit::ProtocolVersion;
use adi_upgrade::onchain;
use adi_upgrade::v31::{OwnershipTargets, V31Orchestrator, V31OrchestratorParams};
use adi_upgrade::{ToolkitRunnerTrait, UpgradeConfig};
use alloy_provider::Provider;

use self::inputs::{now_nonce, resolve_inputs, salt};
use self::surface::surface;
use super::args::UpgradeArgs;
use super::runner_wrapper::ToolkitRunnerWrapper;
use crate::commands::helpers::{
    create_state_manager_with_s3, resolve_gas_multiplier, resolve_rpc_url,
};
use crate::context::Context;
use crate::error::{Result, WrapErr};
use crate::ui;

mod inputs;
mod surface;

/// Default L2 RPC when `--l2-rpc-url` is not given.
const DEFAULT_L2_RPC: &str = "http://127.0.0.1:3050";

/// Run the full v31 upgrade for the resolved chain.
pub(super) async fn run_v31(
    args: &UpgradeArgs,
    context: &Context,
    ecosystem_name: &str,
) -> Result<()> {
    let manual = args.safe || args.calldata;
    if manual && args.yes {
        return Err(eyre::eyre!(
            "--yes cannot be combined with --safe/--calldata: each phase forks the current L1 \
             and must be executed before the next is generated, so the per-bundle confirmation \
             is required in output-only mode"
        ));
    }
    if args.unlocked && manual {
        return Err(eyre::eyre!(
            "--unlocked cannot be combined with --safe/--calldata: unlocked broadcasts every \
             phase via node impersonation, output-only surfaces bundles for you to execute"
        ));
    }
    if args.sign_deployer && !manual {
        return Err(eyre::eyre!(
            "--sign-deployer only applies with --safe/--calldata: it broadcasts just the \
             deployer-signed phases (prepare + stage3) directly and surfaces the rest"
        ));
    }

    let version = ProtocolVersion::V0_31_0;
    let handler = adi_upgrade::versions::V0_31_0Handler;

    let rpc_url = resolve_rpc_url(args.rpc_url.as_ref(), context.config())?;
    let normalized: url::Url = adi_types::normalize_rpc_url(rpc_url.as_str())
        .parse()
        .wrap_err("Failed to parse normalized RPC URL")?;
    let l2_rpc: url::Url = match args.l2_rpc_url.clone() {
        Some(u) => u,
        None => DEFAULT_L2_RPC
            .parse()
            .wrap_err("parse default L2 RPC URL")?,
    };

    // protocol_ops runs inside the toolkit container; rewrite localhost only where
    // Docker needs it (macOS Docker Desktop), matching the rest of the CLI.
    let logger = context.logger();
    let l1_rpc_container = transform_url_for_container(rpc_url.as_str(), logger.as_ref());
    let l2_rpc_container = transform_url_for_container(l2_rpc.as_str(), logger.as_ref());

    ui::intro(format!(
        "Upgrading {} to {}",
        ui::green(ecosystem_name),
        ui::green(version)
    ))?;

    let (state_manager, _s3) = create_state_manager_with_s3(ecosystem_name, context).await?;
    let state_dir = context.config().state_dir.join(ecosystem_name);

    let localhost = adi_types::is_localhost_rpc(rpc_url.as_str());
    let gas_multiplier = (!localhost).then(|| resolve_gas_multiplier(None, context.config()));
    let config = UpgradeConfig::from_state(
        &state_manager,
        ecosystem_name,
        rpc_url.clone(),
        gas_multiplier,
        state_dir.clone(),
    )
    .await
    .wrap_err("Failed to build upgrade config")?;

    let chain_name = crate::commands::helpers::select_chain_from_state(
        args.chain.as_ref(),
        &state_manager,
        ecosystem_name,
    )
    .await?;
    let chain_meta = state_manager
        .chain(&chain_name)
        .metadata()
        .await
        .wrap_err("Failed to load chain metadata")?;

    // Chain-level phases (set-ts / upgrade / total-supply) are signed by the
    // chain's ChainAdmin owner, held in the chain-level wallets.yaml.
    let chain_wallets = state_manager
        .chain(&chain_name)
        .wallets()
        .await
        .wrap_err("Failed to load chain wallets")?;
    let chain_governor = chain_wallets
        .governor
        .as_ref()
        .ok_or_else(|| eyre::eyre!("chain governor wallet not found in state"))?;
    let chain_governor_address = chain_governor.address;
    let chain_governor_private_key =
        secrecy::SecretString::from(chain_governor.expose_private_key().to_string());

    let provider = onchain::create_provider(&normalized);
    let env_name = format!("{ecosystem_name}-v31");
    let (inputs, targets) =
        resolve_inputs(env_name, &config, &provider, &state_manager, &chain_meta).await?;

    let runner = adi_toolkit::ToolkitRunner::with_config_and_logger(
        context.toolkit_config(),
        Arc::clone(context.logger()),
    )
    .await
    .wrap_err("Failed to create toolkit runner")?;
    let wrapper = ToolkitRunnerWrapper(runner);

    let orchestrator = V31Orchestrator::new(V31OrchestratorParams {
        handler: &handler,
        config: &config,
        state_dir: &state_dir,
        runner: &wrapper,
        provider: &provider,
        protocol_version: version.to_semver(),
        env: &inputs.env_name,
        inputs: &inputs,
        l1_rpc_container: &l1_rpc_container,
        l2_rpc: &l2_rpc,
        l2_rpc_container: &l2_rpc_container,
        chain_id: chain_meta.chain_id,
        chain_governor_address,
        chain_governor_private_key,
        upgrade_timestamp: args.upgrade_timestamp.unwrap_or(1),
        manual,
        unlocked: args.unlocked,
        sign_deployer: args.sign_deployer,
    });

    // Per-run nonce so the ownership pre-fix governance op id is fresh each run.
    let prefix_salt = salt(
        config.bridgehub_address,
        config.bridgehub_address,
        "ownership-prefix",
        now_nonce(),
    );
    // The Safe JSON is stamped with the settlement-layer (L1) chain id:
    // those bundles target L1 ecosystem contracts, so the L2 chain id would fail the Safe
    // Transaction Builder network check.
    let l1_chain_id = provider
        .get_chain_id()
        .await
        .wrap_err("resolve settlement-layer chain id")?;
    run_phases(
        &orchestrator,
        args,
        l1_chain_id,
        &targets,
        prefix_salt,
        localhost,
    )
    .await
}

/// Sequence the upgrade phases with the runbook's ordering gates.
async fn run_phases<R, P>(
    orch: &V31Orchestrator<'_, R, P>,
    args: &UpgradeArgs,
    l1_chain_id: u64,
    targets: &OwnershipTargets,
    prefix_salt: B256,
    localhost: bool,
) -> Result<()>
where
    R: ToolkitRunnerTrait,
    P: Provider + Clone,
{
    let manual = args.safe || args.calldata;
    ui::section("v31 upgrade")?;
    if manual {
        ui::info("Output-only mode: nothing is broadcast; execute each operation yourself")?;
    }
    orch.prepare_env()?;

    // CHECK 1: the chain must be on the expected pre-upgrade version (v30.1).
    let current = orch.diamond_protocol_version().await?;
    if current != orch.source_version() {
        return Err(eyre::eyre!(
            "chain is not on the expected pre-upgrade version: diamond reports {current}, expected {}",
            orch.source_version()
        ));
    }

    // CAH/VT/NTV/NUL must be governance-owned or governance stage 1 reverts.
    ui::info("Ownership pre-fix (CAH/VT/NTV/NUL -> governance)...")?;
    surface(
        &orch.ownership_prefix(targets, prefix_salt).await?,
        args,
        l1_chain_id,
        localhost,
    )?;

    ui::info("Preparing ecosystem (protocol_ops upgrade-prepare-all)...")?;
    surface(&orch.prepare().await?, args, l1_chain_id, localhost)?;

    ui::info("Executing governance stages 0+1+2...")?;
    surface(&orch.governance().await?, args, l1_chain_id, localhost)?;

    // CHECK 2: the ecosystem CTM must be on v31 before any per-chain phase.
    let ctm_version = orch.ctm_protocol_version().await?;
    if ctm_version != orch.target_version() {
        return Err(eyre::eyre!(
            "ecosystem CTM did not reach v31: reports {ctm_version}, expected {}",
            orch.target_version()
        ));
    }
    ui::success("Ecosystem CTM is on v31")?;

    // Discover legacy bridged tokens so stage3 registers them (not a
    // silent ETH-only fallback). ADI has none, so this yields an empty list.
    ui::info("Discovering legacy bridged tokens (stage3 input)...")?;
    orch.discover_bridged_tokens().await?;
    match orch.bridged_tokens_count() {
        Ok(0) => ui::info("No legacy bridged tokens found; stage3 registers ETH only")?,
        Ok(n) => {
            ui::info(format!(
                "{n} legacy bridged token(s) discovered; stage3 will register them"
            ))?;
        }
        Err(e) => ui::warning(format!("Could not read discovered token list: {e}"))?,
    }

    ui::info("Registering legacy tokens (stage3)...")?;
    surface(&orch.stage3().await?, args, l1_chain_id, localhost)?;

    ui::info("Setting upgrade timestamp...")?;
    surface(
        &orch.set_upgrade_timestamp().await?,
        args,
        l1_chain_id,
        localhost,
    )?;

    // The server must point at the new BytecodesSupplier before it produces the
    // v31 upgrade batch. The CLI does not own the server/EN configs, so print the value
    // and, when interactive, block until the operator has set it. A fork rehearsal has
    // no server to configure, and --yes is non-interactive, so both skip the pause.
    match orch.new_bytecodes_supplier() {
        Err(e) => ui::warning(format!("Could not read new bytecodes supplier: {e}"))?,
        Ok(supplier) => {
            ui::info(format!(
                "New bytecodes supplier: {supplier} — set genesis_bytecode_supplier_address to it on the server and every EN (no restart needed)"
            ))?;
            if !args.yes && !args.fork {
                let ok = ui::confirm(format!(
                    "Set genesis_bytecode_supplier_address = {supplier} in the server + EN config, then confirm to continue"
                ))
                .interact()
                .wrap_err("bytecodes-supplier confirmation")?;
                eyre::ensure!(ok, "aborted: new bytecodes supplier not set on the server");
            }
        }
    }

    // Every pre-v31 (V6) batch must be executed on L1 BEFORE the diamond cut
    // swaps the verifier V6->V7, or un-executed V6 batches stall permanently. Fatal.
    // An L1-only fork rehearsal has no fork L2 draining batches, so the gate would
    // hang on the real chain's finality; --fork skips it there only.
    if args.fork {
        ui::warning(
            "Skipping readiness gate (--fork); pre-v31 batch drain NOT verified, never use on a real upgrade",
        )?;
    } else {
        // Capture the L2 head just before draining so every block <= it is a pre-upgrade
        // (V6) block. On a busy chain, pause deposits so no new v30.1 block lands between
        // this capture and the injected upgrade tx.
        let pre_upgrade_head = orch.l2_head().await?;
        ui::info("Waiting for all pre-v31 batches to execute (readiness gate)...")?;
        let finalized = orch
            .wait_pre_upgrade_drain(pre_upgrade_head)
            .await
            .wrap_err(
            "pre-finalize readiness gate: pre-v31 batches not fully executed, refusing to finalize",
        )?;
        ui::info(format!(
            "Pre-v31 batches executed up to L2 block {finalized}"
        ))?;
    }

    ui::info("Finalizing chain diamond cut...")?;
    surface(
        &orch.chain_upgrade(None).await?,
        args,
        l1_chain_id,
        localhost,
    )?;

    if !orch.verify().await? {
        return Err(eyre::eyre!(
            "chain did not reach v31 on L1 after the diamond cut"
        ));
    }
    ui::success("Chain reached v31 protocol version on L1")?;

    // Pre-v31 total supply (one-time, irreversible). The calc reads
    // L2BaseToken.Withdrawal at 0x800a, which only a v0.20.x L2 server exposes; a
    // live pre-upgrade L2 (v0.13) has no 0x800a, so --fork skips this step.
    if args.fork {
        ui::warning(
            "Skipping pre-v31 total supply (--fork): needs the L2 server on v0.20.x; a pre-upgrade L2 does not expose L2BaseToken at 0x800a",
        )?;
    } else {
        ui::info("Recording pre-v31 total supply...")?;
        let finalize_tx = if manual {
            ui::input(
                "Paste the chain-upgrade (finalize) tx hash, needed to compute the pre-v31 total supply",
            )
            .interact()
            .wrap_err("finalize tx prompt")?
        } else {
            orch.finalize_tx()?
        };
        surface(
            &orch.set_total_supply(&finalize_tx).await?,
            args,
            l1_chain_id,
            localhost,
        )?;

        // Read-back. Only in broadcast mode; in manual mode the setter has not
        // been executed yet, so the L2 read would revert.
        if !manual {
            // Let the server settle the setter before reading it back; best-effort, a
            // drain failure must not fail an already-completed upgrade.
            let _ = orch.wait_for_server_drain().await;
            orch.l2_base_token_total_supply().await.map_or_else(
                |e| ui::warning(format!("totalSupply() read-back skipped: {e}")),
                |supply| ui::info(format!("L2 base-token totalSupply() = {supply}")),
            )?;
        }
    }

    ui::outro(format!(
        "Upgrade to {} completed",
        ui::green(ProtocolVersion::V0_31_0)
    ))?;
    Ok(())
}
