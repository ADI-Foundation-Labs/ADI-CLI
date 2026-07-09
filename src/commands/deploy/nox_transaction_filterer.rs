//! Post-deployment step: deploy `NoxTransactionFilterer` on L1 (Validium mode only).
//!
//! Runs `forge script` inside the toolkit container against the prebuilt
//! `/deps/nox-transaction-filterer` sources. The deployed proxy address is
//! parsed from the JSON artefact written by the script and persisted into
//! `chains/{chain}/configs/contracts.yaml` under `l1.nox_transaction_filterer_addr`.
//!
//! Skipped (with an info log) when the chain already has a
//! `nox_transaction_filterer_addr` recorded.
//!
//! Idempotency is on the caller — re-running deploy in Validium mode against
//! an already-deployed chain is a no-op for this step.

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
const ARTEFACT_REL: &str = ".adi/nox-transaction-filterer.json";

/// Working directory inside the toolkit image where the contract sources live.
const NOX_TRANSACTION_FILTERER_WORKDIR: &str = "/deps/nox-transaction-filterer";

#[derive(Debug, Deserialize)]
struct DeployArtefact {
    address: Address,
}

/// Parameters for [`deploy_nox_transaction_filterer`].
pub struct DeployNoxTransactionFiltererParams<'a> {
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

/// Deploy `NoxTransactionFilterer` on L1 and persist the proxy address in chain state.
///
/// No-op if the chain already has a `nox_transaction_filterer_addr` set.
pub async fn deploy_nox_transaction_filterer(
    params: DeployNoxTransactionFiltererParams<'_>,
) -> Result<Address> {
    ui::section("Deploying NoxTransactionFilterer")?;

    let DeployNoxTransactionFiltererParams {
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
        .wrap_err("Failed to read chain contracts for nox-transaction-filterer deployment")?;

    if let Some(existing) = contracts.nox_transaction_filterer_addr() {
        ui::info(format!(
            "NoxTransactionFilterer already deployed at {} — skipping",
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

    let chain_metadata = chain
        .metadata()
        .await
        .wrap_err("Failed to read chain metadata for nox-transaction-filterer deployment")?;

    let ecosystem_contracts =
        state_manager.ecosystem().contracts().await.wrap_err(
            "Failed to read ecosystem contracts for nox-transaction-filterer deployment",
        )?;
    let l1_asset_router = ecosystem_contracts
        .bridges
        .as_ref()
        .and_then(|b| b.shared.as_ref())
        .and_then(|s| s.l1_address)
        .ok_or_else(|| {
            eyre::eyre!(
                "L1 asset router (shared bridge) address not found in ecosystem contracts — \
                 ensure ecosystem deployment completed first"
            )
        })?;

    // Use the ecosystem-level deployer wallet — only that one is funded by the
    // funding plan; the per-chain deployer wallet stays empty.
    let ecosystem_wallets = state_manager
        .ecosystem()
        .wallets()
        .await
        .wrap_err("Failed to read ecosystem wallets for nox-transaction-filterer deployment")?;
    let deployer_key = ecosystem_wallets
        .deployer
        .as_ref()
        .ok_or_else(|| {
            eyre::eyre!(
                "Ecosystem deployer wallet required for nox-transaction-filterer deployment"
            )
        })?
        .private_key
        .clone();

    let ecosystem_path = context.config().state_dir.join(ecosystem_name);
    let runner = ToolkitRunner::with_config_and_logger(
        context.toolkit_config(),
        std::sync::Arc::clone(context.logger()),
    )
    .await
    .wrap_err("Failed to create toolkit runner for nox-transaction-filterer deployment")?;

    let l1_asset_router_arg = format!("{l1_asset_router:#x}");
    let l2_chain_id_arg = chain_metadata.chain_id.to_string();
    let chain_admin_arg = format!("{chain_admin:#x}");
    let sig_args = [
        l1_asset_router_arg.as_str(),
        l2_chain_id_arg.as_str(),
        chain_admin_arg.as_str(),
    ];

    let is_localhost = is_localhost_rpc(rpc_url.as_str());
    let effective_gas_price = gas_price_wei.filter(|_| !is_localhost);

    let semver_version = protocol_version.to_semver();
    let forge_params = ForgeScriptParams {
        working_dir: NOX_TRANSACTION_FILTERER_WORKDIR,
        script_path: "script/Deploy.s.sol",
        signature: "run(address,uint256,address)",
        sig_args: &sig_args,
        rpc_url: rpc_url.as_str(),
        gas_price_wei: effective_gas_price,
        state_dir: &ecosystem_path,
        protocol_version: &semver_version,
        log_label: "Deploying NoxTransactionFilterer...",
        log_command: "nox-transaction-filterer-deploy",
    };

    let exit_code = runner
        .run_forge_script(&forge_params, &deployer_key)
        .await
        .wrap_err("Failed to run forge script for nox-transaction-filterer deployment")?;
    if exit_code != 0 {
        return Err(eyre::eyre!(
            "forge script for NoxTransactionFilterer exited with code {exit_code}"
        ));
    }

    let deployed = read_deploy_artefact(&ecosystem_path)
        .wrap_err("Failed to parse nox-transaction-filterer deploy artefact")?;

    let l1 = contracts.l1.get_or_insert_with(Default::default);
    l1.nox_transaction_filterer_addr = Some(deployed.address);
    chain
        .update_contracts(&contracts)
        .await
        .wrap_err("Failed to persist nox_transaction_filterer_addr to chain state")?;

    ui::success(format!(
        "NoxTransactionFilterer deployed at {} (owner = ChainAdmin {})",
        ui::green(deployed.address),
        ui::green(chain_admin),
    ))?;

    Ok(deployed.address)
}

fn read_deploy_artefact(ecosystem_path: &Path) -> Result<DeployArtefact> {
    let path = ecosystem_path.join(ARTEFACT_REL);
    let raw = std::fs::read_to_string(&path).wrap_err_with(|| {
        format!(
            "Nox-transaction-filterer deploy artefact not found at {} — \
             check forge script logs",
            path.display()
        )
    })?;
    serde_json::from_str::<DeployArtefact>(&raw)
        .wrap_err_with(|| format!("Invalid JSON in {}", path.display()))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use alloy_primitives::address;
    use tempfile::TempDir;

    fn write_artefact(ecosystem_path: &Path, contents: &str) {
        let adi_dir = ecosystem_path.join(".adi");
        std::fs::create_dir_all(&adi_dir).expect("create .adi dir");
        std::fs::write(adi_dir.join("nox-transaction-filterer.json"), contents)
            .expect("write artefact");
    }

    #[test]
    fn read_deploy_artefact_parses_proxy_address() {
        let dir = TempDir::new().expect("tempdir");
        write_artefact(
            dir.path(),
            r#"{"address":"0x0000000000000000000000000000000000000042","implementation":"0x0000000000000000000000000000000000000099"}"#,
        );

        let artefact = read_deploy_artefact(dir.path()).expect("artefact should parse");

        assert_eq!(
            artefact.address,
            address!("0000000000000000000000000000000000000042")
        );
    }

    #[test]
    fn read_deploy_artefact_ignores_unknown_fields() {
        let dir = TempDir::new().expect("tempdir");
        write_artefact(
            dir.path(),
            r#"{"address":"0x0000000000000000000000000000000000000042","implementation":"0x0000000000000000000000000000000000000099","extra":"ignored"}"#,
        );

        assert!(read_deploy_artefact(dir.path()).is_ok());
    }

    #[test]
    fn read_deploy_artefact_errors_when_file_missing() {
        let dir = TempDir::new().expect("tempdir");

        let result = read_deploy_artefact(dir.path());

        assert!(result.is_err());
    }

    #[test]
    fn read_deploy_artefact_errors_on_invalid_json() {
        let dir = TempDir::new().expect("tempdir");
        write_artefact(dir.path(), "not json");

        let result = read_deploy_artefact(dir.path());

        assert!(result.is_err());
    }

    #[test]
    fn read_deploy_artefact_errors_when_address_missing() {
        let dir = TempDir::new().expect("tempdir");
        write_artefact(
            dir.path(),
            r#"{"implementation":"0x0000000000000000000000000000000000000099"}"#,
        );

        let result = read_deploy_artefact(dir.path());

        assert!(result.is_err());
    }
}
