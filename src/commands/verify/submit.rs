//! Verification submission logic.

use adi_ecosystem::verification::{
    encode_chain_admin_constructor_args, encode_era_verifier_constructor_args,
    encode_proxy_constructor_args, encode_verifier_constructor_args, VerificationOutcome,
    VerificationResult, VerificationSummary, VerificationTarget,
};
use adi_toolkit::{ForgeVerifyParams, ProtocolVersion, ToolkitRunner};
use std::sync::Arc;

use crate::commands::helpers::resolve_protocol_version;
use crate::error::{Result, WrapErr};
use crate::ui;

use super::check::CheckResult;
use super::config::VerifyConfig;
use super::VerifyArgs;

/// Submit verifications for unverified contracts.
pub(super) async fn submit_verifications(
    config: &VerifyConfig<'_>,
    args: &VerifyArgs,
    status_results: &[(String, CheckResult)],
) -> Result<()> {
    let unverified_targets: Vec<_> = config
        .targets
        .iter()
        .filter(|t| {
            let name = t.contract_type.display_name();
            status_results
                .iter()
                .any(|(n, r)| n == name && !matches!(r, CheckResult::Verified))
        })
        .cloned()
        .collect();

    if unverified_targets.is_empty() {
        ui::outro("No contracts need verification.")?;
        return Ok(());
    }

    if args.dry_run {
        display_verification_plan(&unverified_targets)?;
        ui::outro("Dry-run mode: verification plan displayed")?;
        return Ok(());
    }

    let protocol_version_str =
        resolve_protocol_version(args.protocol_version.as_ref(), config.context.config())?;
    let protocol_version = ProtocolVersion::parse(&protocol_version_str)
        .map_err(|e| eyre::eyre!("Invalid protocol version: {}", e))?;

    ui::section("Submitting Verifications")?;
    ui::info(format!(
        "Submitting {} contracts for verification...",
        unverified_targets.len()
    ))?;

    let runner = ToolkitRunner::with_config_and_logger(
        config.context.toolkit_config(),
        Arc::clone(config.context.logger()),
    )
    .await
    .wrap_err("Failed to create toolkit runner")?;

    let log_dir = config.context.config().state_dir.clone();
    tokio::fs::create_dir_all(log_dir.join("logs"))
        .await
        .wrap_err("Failed to create log directory")?;

    let mut results = Vec::new();
    let progress = cliclack::progress_bar(unverified_targets.len() as u64);
    progress.start("Submitting verifications...");

    for target in &unverified_targets {
        let name = target.contract_type.display_name();
        let address = target.address;

        progress.start(format!("Verifying {}...", name));

        let constructor_args = compute_constructor_args(target);

        let address_str = format!("{:?}", address);
        let contract_path = target.forge_contract_path();
        let semver = protocol_version.to_semver();
        let exit_code = runner
            .run_forge_verify(&ForgeVerifyParams {
                address: &address_str,
                contract_path: &contract_path,
                chain_id: config.explorer_client.config().chain_id,
                verifier_url: config.explorer_client.config().api_url.as_str(),
                verifier: config
                    .explorer_client
                    .config()
                    .explorer_type
                    .forge_verifier_name(),
                api_key: config.explorer_client.config().api_key.as_deref(),
                constructor_args: constructor_args.as_deref(),
                protocol_version: &semver,
                log_dir: &log_dir,
                root_path: target.root_path,
            })
            .await;

        let result = match exit_code {
            Ok(0) => VerificationResult::submitted(name, address, "submitted".to_string()),
            Ok(code) => VerificationResult::failed(name, address, format!("Exit code: {}", code)),
            Err(e) => VerificationResult::failed(name, address, e.to_string()),
        };

        let is_failure = matches!(result.outcome, VerificationOutcome::Failed { .. });
        results.push(result);
        progress.inc(1);

        if is_failure && !args.continue_on_error {
            progress.stop("Stopped due to failure");
            config.context.logger().warning(
                "Stopping verification due to failure (use --continue-on-error to continue)",
            );
            break;
        }

        config.explorer_client.rate_limit_delay().await;
    }

    progress.stop("Verification submission complete");

    display_summary(&VerificationSummary::new(results), &log_dir)
}

/// Compute constructor args based on contract type.
fn compute_constructor_args(target: &VerificationTarget) -> Option<String> {
    target
        .proxy_info
        .as_ref()
        .map(|info| {
            encode_proxy_constructor_args(info.impl_addr, info.proxy_admin_addr, &info.init_data)
        })
        .or_else(|| {
            target.verifier_info.as_ref().map(|info| {
                if let Some(owner) = info.owner_addr {
                    encode_verifier_constructor_args(info.fflonk_addr, info.plonk_addr, owner)
                } else {
                    encode_era_verifier_constructor_args(info.fflonk_addr, info.plonk_addr)
                }
            })
        })
        .or_else(|| {
            target.chain_admin_info.as_ref().map(|info| {
                encode_chain_admin_constructor_args(info.owner_addr, info.token_multiplier_setter)
            })
        })
}

/// Display verification summary and final message.
fn display_summary(summary: &VerificationSummary, log_dir: &std::path::Path) -> Result<()> {
    ui::note(
        "Verification Summary",
        format!(
            "Submitted: {}  Already verified: {}  Skipped: {}  Failed: {}",
            ui::green(summary.submitted_count()),
            ui::cyan(summary.already_verified_count()),
            ui::yellow(summary.skipped_count()),
            ui::red(summary.failed_count())
        ),
    )?;

    if summary.failed_count() > 0 {
        ui::outro_cancel(format!(
            "{} contracts failed verification. Check logs in {}",
            summary.failed_count(),
            log_dir.display()
        ))?;
    } else {
        ui::outro("Verification submission complete!")?;
    }

    Ok(())
}

/// Display verification plan (dry-run mode).
fn display_verification_plan(targets: &[VerificationTarget]) -> Result<()> {
    let lines: Vec<String> = targets
        .iter()
        .map(|t| format!("  {} -> {:?}", t.contract_type.display_name(), t.address))
        .collect();
    ui::note("Contracts to verify", lines.join("\n"))?;
    Ok(())
}
