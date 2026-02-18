//! Contract verification orchestration.
//!
//! Handles the execution of forge verify-contract commands via Docker toolkit.

use adi_ecosystem::verification::{
    VerificationOutcome, VerificationResult, VerificationSummary, VerificationTarget,
};
use adi_toolkit::{ProtocolVersion, ToolkitRunner};
use std::sync::Arc;
use std::time::Duration;
use url::Url;

use crate::context::Context;
use crate::ui;

/// Delay between verification submissions to avoid rate limiting.
const VERIFICATION_DELAY_MS: u64 = 200;

/// Verify multiple contracts via the Docker toolkit.
pub async fn verify_contracts(
    targets: &[VerificationTarget],
    explorer_url: &Url,
    api_key: &str,
    chain_id: u64,
    protocol_version: &str,
    context: &Context,
) -> VerificationSummary {
    let mut results = Vec::new();

    // Parse protocol version
    let version = match parse_protocol_version(protocol_version) {
        Ok(v) => v,
        Err(e) => {
            context.logger().error(&format!(
                "Invalid protocol version '{}': {}",
                protocol_version, e
            ));
            // Return failure for all contracts
            for target in targets {
                results.push(VerificationResult::failed(
                    target.contract_type.display_name(),
                    target.address,
                    format!("Invalid protocol version: {}", e),
                ));
            }
            return VerificationSummary::new(results);
        }
    };

    // Build toolkit config
    let toolkit_config = context.toolkit_config();

    // Create toolkit runner
    let runner =
        match ToolkitRunner::with_config_and_logger(toolkit_config, Arc::clone(context.logger()))
            .await
        {
            Ok(r) => r,
            Err(e) => {
                context
                    .logger()
                    .error(&format!("Failed to create toolkit runner: {}", e));
                for target in targets {
                    results.push(VerificationResult::failed(
                        target.contract_type.display_name(),
                        target.address,
                        format!("Failed to create toolkit runner: {}", e),
                    ));
                }
                return VerificationSummary::new(results);
            }
        };

    // Verify each contract sequentially
    for target in targets {
        let spinner = cliclack::spinner();
        spinner.start(format!(
            "Verifying {} ({})...",
            target.contract_type.display_name(),
            &target.address.to_string()[..10]
        ));

        let result = verify_single_contract(
            &runner,
            target,
            explorer_url,
            api_key,
            chain_id,
            version,
            context,
        )
        .await;

        match &result.outcome {
            VerificationOutcome::Submitted { guid } => {
                spinner.stop(format!(
                    "{} → {} (GUID: {})",
                    target.contract_type.display_name(),
                    ui::green("Submitted"),
                    guid
                ));
            }
            VerificationOutcome::Confirmed => {
                spinner.stop(format!(
                    "{} → {}",
                    target.contract_type.display_name(),
                    ui::green("Confirmed")
                ));
            }
            VerificationOutcome::AlreadyVerified => {
                spinner.stop(format!(
                    "{} → {}",
                    target.contract_type.display_name(),
                    ui::cyan("Already Verified")
                ));
            }
            VerificationOutcome::Failed { reason } => {
                spinner.stop(format!(
                    "{} → {}",
                    target.contract_type.display_name(),
                    ui::red(&format!("Failed: {}", reason))
                ));
            }
            VerificationOutcome::Skipped { reason } => {
                spinner.stop(format!(
                    "{} → {}",
                    target.contract_type.display_name(),
                    ui::yellow(&format!("Skipped: {}", reason))
                ));
            }
        }

        results.push(result);

        // Rate limit delay
        tokio::time::sleep(Duration::from_millis(VERIFICATION_DELAY_MS)).await;
    }

    VerificationSummary::new(results)
}

/// Verify a single contract.
async fn verify_single_contract(
    runner: &ToolkitRunner,
    target: &VerificationTarget,
    explorer_url: &Url,
    api_key: &str,
    chain_id: u64,
    protocol_version: ProtocolVersion,
    context: &Context,
) -> VerificationResult {
    context.logger().debug(&format!(
        "Verifying {} at {} (source: {})",
        target.contract_type.display_name(),
        target.address,
        target.forge_contract_path()
    ));

    // Execute forge verify-contract
    let exit_code = match runner
        .run_forge_verify(
            &format!("{:?}", target.address),
            &target.forge_contract_path(),
            chain_id,
            explorer_url.as_str(),
            api_key,
            None, // Constructor args - could be added later
            &protocol_version.to_semver(),
        )
        .await
    {
        Ok(code) => code,
        Err(e) => {
            return VerificationResult::failed(
                target.contract_type.display_name(),
                target.address,
                format!("Forge command failed: {}", e),
            );
        }
    };

    if exit_code == 0 {
        // Success - forge returns 0 when verification is submitted or confirmed
        VerificationResult::submitted(
            target.contract_type.display_name(),
            target.address,
            "pending".to_string(), // Forge doesn't always return GUID
        )
    } else {
        VerificationResult::failed(
            target.contract_type.display_name(),
            target.address,
            format!("Forge exited with code {}", exit_code),
        )
    }
}

/// Parse protocol version string to ProtocolVersion.
fn parse_protocol_version(version_str: &str) -> Result<ProtocolVersion, String> {
    ProtocolVersion::parse(version_str).map_err(|e| e.to_string())
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_protocol_version() {
        let v = parse_protocol_version("v30.0.2").unwrap();
        let semver = v.to_semver();
        assert_eq!(semver.major, 30);
        assert_eq!(semver.minor, 0);
        assert_eq!(semver.patch, 2);

        let v = parse_protocol_version("30.0.2").unwrap();
        let semver = v.to_semver();
        assert_eq!(semver.major, 30);

        assert!(parse_protocol_version("invalid").is_err());
    }
}
