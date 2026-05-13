//! Post-deployment step: deploy `FeeAdjusterConfig` on L1 with `ChainAdmin` as owner.
//!
//! Runs `forge script` inside the toolkit container against the prebuilt
//! `/deps/fee-adjuster-contracts` sources. The deployed proxy address is parsed
//! from the JSON artefact written by the script and persisted into
//! `chains/{chain}/configs/contracts.yaml` under `l1.fee_adjuster_config`.
//!
//! Skipped (with an info log) when the chain already has a `fee_adjuster_config`
//! address recorded.
//!
//! Idempotency is on the caller — re-running `adi deploy --with-fee-adjuster`
//! against an already-deployed chain is a no-op.

use adi_funding::is_localhost_rpc;
use adi_state::StateManager;
use adi_toolkit::{ForgeScriptParams, ProtocolVersion, ToolkitRunner};
use alloy_primitives::Address;
use serde::Deserialize;
use std::path::Path;
use url::Url;

use crate::context::Context;
use crate::error::{Result, WrapErr};
use crate::ui;

/// Path inside the toolkit container where the deploy artefact is written.
const ARTEFACT_REL: &str = ".adi/fee-adjuster.json";

/// Working directory inside the toolkit image where the contract sources live.
const FEE_ADJUSTER_WORKDIR: &str = "/deps/fee-adjuster-contracts";

#[derive(Debug, Deserialize)]
struct DeployArtefact {
    address: Address,
    #[allow(dead_code)]
    owner: Address,
}

/// Parameters for [`deploy_fee_adjuster`].
pub struct DeployFeeAdjusterParams<'a> {
    /// CLI context (config, logger).
    pub context: &'a Context,
    /// State manager bound to the target ecosystem.
    pub state_manager: &'a StateManager,
    /// Name of the chain whose ChainAdmin will own the deployed contract.
    pub chain_name: &'a str,
    /// Ecosystem name (used to resolve the state directory mount + read wallets).
    pub ecosystem_name: &'a str,
    /// Settlement-layer RPC URL.
    pub rpc_url: &'a Url,
    /// Toolkit protocol version for image selection.
    pub protocol_version: &'a ProtocolVersion,
    /// Optional gas price in wei (None ⇒ let forge estimate, used on localhost).
    pub gas_price_wei: Option<u128>,
}

/// Deploy `FeeAdjusterConfig` on L1 and persist the address in chain state.
///
/// No-op if the chain already has a `fee_adjuster_config` address set.
pub async fn deploy_fee_adjuster(params: DeployFeeAdjusterParams<'_>) -> Result<Address> {
    ui::section("Deploying FeeAdjusterConfig")?;

    let DeployFeeAdjusterParams {
        context,
        state_manager,
        chain_name,
        ecosystem_name,
        rpc_url,
        protocol_version,
        gas_price_wei,
    } = params;

    let chain = state_manager.chain(chain_name);
    let mut contracts = chain
        .contracts()
        .await
        .wrap_err("Failed to read chain contracts for fee-adjuster deployment")?;

    if let Some(existing) = contracts.fee_adjuster_config() {
        ui::info(format!(
            "FeeAdjusterConfig already deployed at {} — skipping",
            ui::green(existing)
        ))?;
        return Ok(existing);
    }

    let chain_admin = contracts.chain_admin_addr().ok_or_else(|| {
        eyre::eyre!(
            "Chain admin address not found in chain '{}' contracts — \
             ensure ecosystem deployment completed first",
            chain_name
        )
    })?;

    // Use the ecosystem-level deployer wallet — only that one is funded by the
    // funding plan; the per-chain deployer wallet stays empty.
    let ecosystem_wallets = state_manager
        .ecosystem()
        .wallets()
        .await
        .wrap_err("Failed to read ecosystem wallets for fee-adjuster deployment")?;
    let deployer_key = ecosystem_wallets
        .deployer
        .as_ref()
        .ok_or_else(|| {
            eyre::eyre!("Ecosystem deployer wallet required for fee-adjuster deployment")
        })?
        .private_key
        .clone();

    let ecosystem_path = context.config().state_dir.join(ecosystem_name);
    let runner = ToolkitRunner::with_config_and_logger(
        context.toolkit_config(),
        std::sync::Arc::clone(context.logger()),
    )
    .await
    .wrap_err("Failed to create toolkit runner for fee-adjuster deployment")?;

    let chain_admin_arg = format!("{chain_admin:#x}");
    let sig_args = [chain_admin_arg.as_str()];

    let effective_gas_price = if is_localhost_rpc(rpc_url.as_str()) {
        None
    } else {
        gas_price_wei
    };

    let semver_version = protocol_version.to_semver();
    let params = ForgeScriptParams {
        working_dir: FEE_ADJUSTER_WORKDIR,
        script_path: "script/Deploy.s.sol",
        signature: "run(address)",
        sig_args: &sig_args,
        rpc_url: rpc_url.as_str(),
        gas_price_wei: effective_gas_price,
        state_dir: &ecosystem_path,
        protocol_version: &semver_version,
        log_label: "Deploying FeeAdjusterConfig...",
        log_command: "fee-adjuster-deploy",
    };

    let exit_code = runner
        .run_forge_script(&params, &deployer_key)
        .await
        .wrap_err("Failed to run forge script for fee-adjuster deployment")?;
    if exit_code != 0 {
        return Err(eyre::eyre!(
            "forge script for FeeAdjusterConfig exited with code {exit_code}"
        ));
    }

    let deployed = read_deploy_artefact(&ecosystem_path)
        .wrap_err("Failed to parse fee-adjuster deploy artefact")?;

    let l1 = contracts.l1.get_or_insert_with(Default::default);
    l1.fee_adjuster_config = Some(deployed.address);
    chain
        .update_contracts(&contracts)
        .await
        .wrap_err("Failed to persist fee_adjuster_config address to chain state")?;

    ui::success(format!(
        "FeeAdjusterConfig deployed at {} (owner = ChainAdmin {})",
        ui::green(deployed.address),
        ui::green(chain_admin),
    ))?;

    Ok(deployed.address)
}

fn read_deploy_artefact(ecosystem_path: &Path) -> Result<DeployArtefact> {
    let path = ecosystem_path.join(ARTEFACT_REL);
    let raw = std::fs::read_to_string(&path).wrap_err_with(|| {
        format!(
            "Fee-adjuster deploy artefact not found at {} — \
             check forge script logs",
            path.display()
        )
    })?;
    serde_json::from_str::<DeployArtefact>(&raw)
        .wrap_err_with(|| format!("Invalid JSON in {}", path.display()))
}
