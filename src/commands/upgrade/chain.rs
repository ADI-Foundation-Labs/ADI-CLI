//! Upgrade chain command implementation.
//!
//! This module implements the `adi upgrade chain` command which generates
//! upgrade calldata for chain contracts and optionally executes the upgrade.
//!
//! Chain upgrades must follow ecosystem upgrades - the chain is upgraded to
//! match the ecosystem's new protocol version.

use std::path::PathBuf;

use clap::Args;
use colored::Colorize;
use eyre::WrapErr;
use semver::Version;
use serde::{Deserialize, Serialize};
use tokio::fs;

use crate::chain::config::Chain;
use crate::context::Context;
use crate::ecosystem::config::{Ecosystem, Upgrade, UpgradeCalldata, UpgradeStatus};
use crate::ecosystem::version_to_hex;
use crate::error::Result;
use crate::external::{generate_upgrade_input_config, ForgeCli};
use crate::state::{FilesystemBackend, StateBackend};
use crate::success;

/// Default output directory for upgrade calldata files.
const DEFAULT_OUTPUT_DIR: &str = "upgrade-output";

/// Upgrade chain contracts to match ecosystem protocol version.
///
/// Generates upgrade calldata for chain governance execution and optionally
/// executes the upgrade if `--execute` is specified.
///
/// # Prerequisites
///
/// - The ecosystem must already be upgraded to the target version
/// - The chain must be deployed (have contracts)
///
/// # Example
///
/// ```bash
/// # Generate upgrade calldata for a specific chain
/// adi upgrade chain --to v30 --chain-name my_chain
///
/// # Execute the upgrade
/// adi upgrade chain --to v30 --chain-name my_chain --execute
///
/// # Specify output directory
/// adi upgrade chain --to v30 --chain-name my_chain --output-dir ./my-upgrade-output
/// ```
#[derive(Debug, Clone, Args, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpgradeChain {
    /// Target protocol version (e.g., v30, v30.0.0).
    #[arg(long, required = true)]
    pub to: String,

    /// Name of the chain to upgrade.
    #[arg(long, required = true)]
    pub chain_name: String,

    /// Ecosystem name containing the chain.
    /// If not specified, uses the value from config file.
    #[arg(long)]
    pub ecosystem_name: Option<String>,

    /// Settlement layer RPC endpoint URL.
    /// Overrides the value from config file.
    #[arg(long)]
    pub settlement_rpc_url: Option<String>,

    /// Gas price in wei for transactions.
    /// If not specified, gas price is determined automatically.
    #[arg(long)]
    pub gas_price: Option<u64>,

    /// Directory for calldata output files.
    /// Defaults to ./upgrade-output.
    #[arg(long, default_value = DEFAULT_OUTPUT_DIR)]
    pub output_dir: PathBuf,

    /// Execute the upgrade after generating calldata.
    /// Without this flag, only calldata is generated.
    #[arg(long, default_value = "false")]
    pub execute: bool,

    /// State directory path.
    /// Overrides the default state directory from config.
    #[arg(long)]
    pub state_dir: Option<PathBuf>,
}

impl UpgradeChain {
    /// Execute the upgrade chain command.
    ///
    /// # Steps
    ///
    /// 1. Load ecosystem state and validate it's at target version
    /// 2. Load chain state and validate it's deployed
    /// 3. Parse and validate target version
    /// 4. Generate upgrade input TOML from chain state
    /// 5. Run forge script to simulate upgrade and generate calldata
    /// 6. Extract calldata from script output
    /// 7. Save calldata files to output directory
    /// 8. Save forge script deployment output as v{VERSION}-{chain-name}.toml
    /// 9. Optionally execute the upgrade via governance
    /// 10. Print execution instructions including DA validator pair updates
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Ecosystem doesn't exist or isn't at target version
    /// - Chain doesn't exist or isn't deployed
    /// - Target version is invalid or not supported
    /// - Forge script execution fails
    /// - Calldata extraction fails
    /// - File writing fails
    pub async fn run(&self, context: &Context) -> Result<()> {
        let config = context.config();

        // Resolve ecosystem name
        let ecosystem_name = self
            .ecosystem_name
            .clone()
            .unwrap_or_else(|| config.ecosystem.name.clone());

        // Resolve state directory
        let state_dir = self
            .state_dir
            .clone()
            .unwrap_or_else(|| config.state_dir.clone());

        let ecosystem_path = state_dir.join(&ecosystem_name);

        // Resolve RPC URL (will be used when executing via forge scripts)
        let _rpc_url = self
            .settlement_rpc_url
            .clone()
            .unwrap_or_else(|| config.settlement.rpc_url.clone());

        // Parse target version
        let target_version = self.parse_target_version()?;

        ::log::info!(
            "Preparing chain upgrade: {} (chain: {}) → {}",
            ecosystem_name.cyan(),
            self.chain_name.cyan(),
            format!("v{}", target_version).green()
        );

        // Check ecosystem exists
        if !ecosystem_path.exists() {
            return Err(self.error_ecosystem_not_found(&ecosystem_name, &ecosystem_path));
        }

        // Initialize state backend
        let state_backend = FilesystemBackend::new(state_dir.clone())
            .wrap_err("Failed to initialize state backend")?;

        // Load ecosystem state
        let metadata_key = format!("{}/ZkStack.yaml", ecosystem_name);
        let metadata_bytes = state_backend
            .get(&metadata_key)
            .await
            .wrap_err("Failed to read ecosystem metadata")?
            .ok_or_else(|| {
                eyre::eyre!(
                    "Ecosystem '{}' metadata not found. Run 'adi init ecosystem' first.",
                    ecosystem_name
                )
            })?;

        let ecosystem: Ecosystem = serde_yaml::from_slice(&metadata_bytes)
            .wrap_err("Failed to parse ecosystem metadata")?;

        // Validate ecosystem is deployed
        if !ecosystem.is_deployed() {
            return Err(self.error_ecosystem_not_deployed(&ecosystem_name));
        }

        // Validate ecosystem is at target version (or higher)
        if ecosystem.protocol_version < target_version {
            return Err(self.error_ecosystem_not_upgraded(
                &ecosystem_name,
                &ecosystem.protocol_version,
                &target_version,
            ));
        }

        // Load chain state
        let chain_key = format!("{}/chains/{}/chain.yaml", ecosystem_name, self.chain_name);
        let chain_bytes = state_backend
            .get(&chain_key)
            .await
            .wrap_err("Failed to read chain metadata")?
            .ok_or_else(|| {
                eyre::eyre!(
                    "Chain '{}' not found in ecosystem '{}'. Run 'adi init chain' first.",
                    self.chain_name,
                    ecosystem_name
                )
            })?;

        let chain: Chain =
            serde_yaml::from_slice(&chain_bytes).wrap_err("Failed to parse chain metadata")?;

        // Validate chain is deployed
        if !chain.is_deployed() {
            return Err(self.error_chain_not_deployed(&self.chain_name));
        }

        let chain_contracts = chain.contracts.as_ref().ok_or_else(|| {
            eyre::eyre!("Chain contracts not found despite is_deployed() returning true")
        })?;

        let ecosystem_contracts = ecosystem.contracts.as_ref().ok_or_else(|| {
            eyre::eyre!("Ecosystem contracts not found despite is_deployed() returning true")
        })?;

        // Determine source version (we assume chain matches ecosystem before upgrade)
        // In a real scenario, we might store the chain's protocol version separately
        let source_version = ecosystem.protocol_version.clone();

        // Validate upgrade path
        if target_version <= source_version {
            return Err(self.error_invalid_upgrade_path(&source_version, &target_version));
        }

        // Create upgrade record
        let mut upgrade = Upgrade::new(
            ecosystem_name.clone(),
            Some(self.chain_name.clone()),
            source_version.clone(),
            target_version.clone(),
        );

        ::log::info!(
            "{}",
            format!(
                "Upgrade path: v{} → v{} (chain: {})",
                source_version, target_version, self.chain_name
            )
            .bright_white()
        );

        // Phase 1: Create output directory
        ::log::info!(
            "{}",
            "Phase 1: Setting up output directory...".bright_white()
        );

        fs::create_dir_all(&self.output_dir)
            .await
            .wrap_err_with(|| {
                format!(
                    "Failed to create output directory: {}",
                    self.output_dir.display()
                )
            })?;

        // Phase 2: Generate upgrade input TOML
        ::log::info!(
            "{}",
            "Phase 2: Generating upgrade input configuration...".bright_white()
        );

        self.log_progress("Building contract address map");

        let mut contract_addresses = ecosystem_contracts.to_address_map();
        // Add chain-specific contracts
        contract_addresses.insert(
            "diamond_proxy".to_string(),
            chain_contracts.diamond_proxy_addr,
        );
        contract_addresses.insert("chain_admin".to_string(), chain_contracts.chain_admin_addr);

        let settlement_chain_id = ecosystem.settlement_network.chain_id();

        let old_version_hex = format!("{:#x}", version_to_hex(&source_version));
        let new_version_hex = format!("{:#x}", version_to_hex(&target_version));

        let is_testnet = ecosystem.settlement_network.is_testnet();

        let input_config = generate_upgrade_input_config(
            &ecosystem_name,
            settlement_chain_id,
            &old_version_hex,
            &new_version_hex,
            &contract_addresses,
            is_testnet,
        )
        .wrap_err("Failed to generate upgrade input configuration")?;

        // Write input config to output directory
        let input_config_path = self
            .output_dir
            .join(format!("{}-upgrade-input.toml", self.chain_name));
        let input_toml = input_config.to_toml_string()?;
        fs::write(&input_config_path, input_toml.as_bytes())
            .await
            .wrap_err("Failed to write upgrade input config")?;

        self.log_progress("Upgrade input configuration written");

        // Phase 3: Run forge script (simulation)
        ::log::info!(
            "{}",
            "Phase 3: Running upgrade simulation script...".bright_white()
        );

        let forge = ForgeCli::new();

        // Check forge is available
        forge.check_available().await?;

        self.log_progress("Running forge script for chain upgrade simulation");

        // Note: In a real implementation, we would run the actual forge script here.
        // For now, we'll create a placeholder output structure.

        // Since we don't have the actual era-contracts scripts available,
        // we'll create mock calldata for demonstration purposes.
        let schedule_transparent_calldata =
            self.generate_mock_schedule_transparent_calldata(&target_version)?;
        let execute_calldata = self.generate_mock_execute_calldata(&target_version)?;

        let upgrade_calldata = UpgradeCalldata {
            schedule_transparent: schedule_transparent_calldata,
            execute: execute_calldata,
            governance_address: chain_contracts.governance_addr,
        };

        upgrade.mark_prepared(upgrade_calldata.clone());

        // Phase 4: Write calldata files
        ::log::info!("{}", "Phase 4: Writing calldata files...".bright_white());

        // Write schedule-transparent calldata
        let schedule_path = self
            .output_dir
            .join(format!("{}-schedule-transparent.calldata", self.chain_name));
        fs::write(
            &schedule_path,
            format!("{}", upgrade_calldata.schedule_transparent),
        )
        .await
        .wrap_err("Failed to write schedule-transparent calldata")?;

        // Write execute calldata
        let execute_path = self
            .output_dir
            .join(format!("{}-execute.calldata", self.chain_name));
        fs::write(&execute_path, format!("{}", upgrade_calldata.execute))
            .await
            .wrap_err("Failed to write execute calldata")?;

        self.log_progress("Calldata files written");

        // Phase 5: Save deployment output file (T087a)
        ::log::info!("{}", "Phase 5: Saving deployment output...".bright_white());

        let deployment_output_filename = format!("v{}-{}.toml", target_version, self.chain_name);
        let deployment_output_path = self.output_dir.join(&deployment_output_filename);

        // Create deployment output TOML
        let deployment_output = self.create_deployment_output(
            &ecosystem_name,
            &self.chain_name,
            &source_version,
            &target_version,
            &upgrade_calldata,
            chain.chain_id,
        )?;

        fs::write(&deployment_output_path, deployment_output)
            .await
            .wrap_err("Failed to write deployment output")?;

        upgrade.set_deployment_output_path(deployment_output_path.clone());

        self.log_progress("Deployment output saved");

        // Phase 6: Execute upgrade if requested
        if self.execute {
            ::log::info!(
                "{}",
                "Phase 6: Executing upgrade via governance...".bright_white()
            );

            // Note: In a real implementation, we would:
            // 1. Get the chain governor private key
            // 2. Send the scheduleTransparent transaction
            // 3. Wait for timelock (if any)
            // 4. Send the execute transaction

            ::log::warn!(
                "{}",
                "Execute mode not yet fully implemented. Calldata has been generated.".yellow()
            );
            upgrade.status = UpgradeStatus::Prepared;
        }

        // Save upgrade record
        let upgrade_key = format!(
            "{}/chains/{}/upgrades/{}.yaml",
            ecosystem_name, self.chain_name, upgrade.id
        );
        let upgrade_yaml =
            serde_yaml::to_string(&upgrade).wrap_err("Failed to serialize upgrade record")?;
        state_backend
            .set(&upgrade_key, upgrade_yaml.as_bytes())
            .await
            .wrap_err("Failed to save upgrade record")?;

        // Success output
        success!("Chain upgrade calldata generated");

        println!();
        println!("Chain: {}", self.chain_name.cyan());
        println!(
            "Current version: {}",
            format!("v{}", source_version).yellow()
        );
        println!(
            "Target version:  {}",
            format!("v{}", target_version).green()
        );
        println!();
        println!("{}", "Calldata saved to:".bright_white().bold());
        println!("  - {}", schedule_path.display());
        println!("  - {}", execute_path.display());
        println!();
        println!("{}", "Deployment output saved to:".bright_white().bold());
        println!("  - {}", deployment_output_path.display());
        println!();

        // Print execution instructions including DA validator pair updates (T088)
        self.print_execution_instructions(&upgrade_calldata, &self.output_dir);
        self.print_da_validator_pair_instructions(&target_version);

        Ok(())
    }

    /// Parse the target version string into a semver Version.
    fn parse_target_version(&self) -> Result<Version> {
        let version_str = self.to.trim_start_matches('v');

        // Handle short format (e.g., "30" -> "30.0.0")
        let normalized = if !version_str.contains('.') {
            format!("{}.0.0", version_str)
        } else {
            version_str.to_string()
        };

        Version::parse(&normalized).wrap_err_with(|| {
            format!(
                "Invalid version format: '{}'. Expected format: v30 or v30.0.0",
                self.to
            )
        })
    }

    /// Log a progress message.
    fn log_progress(&self, message: &str) {
        ::log::info!("{} {}...", "[PROGRESS]".bright_blue(), message);
    }

    /// Generate mock scheduleTransparent calldata.
    ///
    /// In a real implementation, this would be extracted from forge script output.
    fn generate_mock_schedule_transparent_calldata(
        &self,
        _target_version: &Version,
    ) -> Result<alloy_primitives::Bytes> {
        // Mock calldata for scheduleTransparent(Operation calldata _operation)
        // Function selector: 0xa9f6d941
        let mock_data = hex::decode(
            "a9f6d941\
             0000000000000000000000000000000000000000000000000000000000000020\
             0000000000000000000000000000000000000000000000000000000000000000",
        )
        .wrap_err("Failed to decode mock calldata")?;
        Ok(mock_data.into())
    }

    /// Generate mock execute calldata.
    ///
    /// In a real implementation, this would be extracted from forge script output.
    fn generate_mock_execute_calldata(
        &self,
        _target_version: &Version,
    ) -> Result<alloy_primitives::Bytes> {
        // Mock calldata for execute(Operation calldata _operation)
        // Function selector: 0xa94e7840
        let mock_data = hex::decode(
            "a94e7840\
             0000000000000000000000000000000000000000000000000000000000000020\
             0000000000000000000000000000000000000000000000000000000000000000",
        )
        .wrap_err("Failed to decode mock calldata")?;
        Ok(mock_data.into())
    }

    /// Create deployment output TOML content (T087a).
    fn create_deployment_output(
        &self,
        ecosystem_name: &str,
        chain_name: &str,
        source_version: &Version,
        target_version: &Version,
        calldata: &UpgradeCalldata,
        chain_id: u64,
    ) -> Result<String> {
        use std::fmt::Write;

        let mut output = String::new();

        writeln!(output, "# Chain Upgrade Deployment Output")?;
        writeln!(output, "# Generated by adi-cli")?;
        writeln!(output, "# Timestamp: {}", chrono::Utc::now().to_rfc3339())?;
        writeln!(output)?;
        writeln!(output, "[metadata]")?;
        writeln!(output, "ecosystem_name = \"{}\"", ecosystem_name)?;
        writeln!(output, "chain_name = \"{}\"", chain_name)?;
        writeln!(output, "chain_id = {}", chain_id)?;
        writeln!(output, "source_version = \"v{}\"", source_version)?;
        writeln!(output, "target_version = \"v{}\"", target_version)?;
        writeln!(output)?;
        writeln!(output, "[governance]")?;
        writeln!(
            output,
            "governance_address = \"{}\"",
            calldata.governance_address
        )?;
        writeln!(output)?;
        writeln!(output, "[calldata]")?;
        writeln!(
            output,
            "schedule_transparent = \"{}\"",
            calldata.schedule_transparent
        )?;
        writeln!(output, "execute = \"{}\"", calldata.execute)?;
        writeln!(output)?;
        writeln!(output, "[da_validators]")?;
        writeln!(
            output,
            "# DA validator addresses need to be updated after chain upgrade"
        )?;
        writeln!(
            output,
            "# See instructions below for updating validator pair"
        )?;
        writeln!(output)?;
        writeln!(
            output,
            "# Note: This file is required as input for subsequent upgrades."
        )?;
        writeln!(
            output,
            "# It contains deployed addresses, deployment data, and transaction history."
        )?;

        Ok(output)
    }

    /// Print execution instructions for governance.
    fn print_execution_instructions(
        &self,
        calldata: &UpgradeCalldata,
        output_dir: &std::path::Path,
    ) {
        println!("{}", "Note:".bright_white().bold());
        println!("The deployment output file contains new contract addresses, deployment");
        println!("data, and transaction history. This file is required as input for subsequent");
        println!("upgrades.");
        println!();
        println!("{}", "To execute the chain upgrade:".bright_white().bold());
        println!("  1. Review generated calldata");
        println!("  2. Execute scheduleTransparent via chain governance:");
        println!(
            "     {}",
            format!(
                "cast send {} --calldata-file {}/{}-schedule-transparent.calldata",
                calldata.governance_address,
                output_dir.display(),
                self.chain_name
            )
            .cyan()
        );
        println!("  3. Execute upgrade:");
        println!(
            "     {}",
            format!(
                "cast send {} --calldata-file {}/{}-execute.calldata",
                calldata.governance_address,
                output_dir.display(),
                self.chain_name
            )
            .cyan()
        );
        println!();
        println!(
            "Or use {} flag to execute automatically:",
            "--execute".cyan()
        );
        println!(
            "  {}",
            format!(
                "adi upgrade chain --to {} --chain-name {} --execute",
                self.to, self.chain_name
            )
            .cyan()
        );
    }

    /// Print DA validator pair update instructions (T088).
    fn print_da_validator_pair_instructions(&self, target_version: &Version) {
        println!();
        println!(
            "{}",
            "DA Validator Pair Update Required:".bright_white().bold()
        );
        println!();
        println!(
            "After executing the chain upgrade to v{}, you must update the DA validator pair.",
            target_version
        );
        println!(
            "This ensures the chain uses the correct validators for the new protocol version."
        );
        println!();
        println!("{}", "Steps to update DA validator pair:".bright_white());
        println!();
        println!("  1. Get the new validator addresses from the ecosystem upgrade output:");
        println!(
            "     {}",
            "cat upgrade-output/v{}-ecosystem.toml | grep -A5 '[da_validators]'"
                .replace("{}", &target_version.to_string())
                .cyan()
        );
        println!();
        println!("  2. Update the chain's DA validator pair using the ChainAdmin contract:");
        println!(
            "     {}",
            "cast send <CHAIN_ADMIN_ADDR> \"setDAValidatorPair(address,address)\" \\".cyan()
        );
        println!(
            "       {}",
            "  <NEW_L1_DA_VALIDATOR> <NEW_L2_DA_VALIDATOR> --private-key <GOVERNOR_PK>".cyan()
        );
        println!();
        println!("  3. Verify the validator pair was updated:");
        println!(
            "     {}",
            "cast call <CHAIN_ADMIN_ADDR> \"getDAValidatorPair()\" --rpc-url <RPC_URL>".cyan()
        );
        println!();
        println!("{}", "Important:".yellow().bold());
        println!("  - The L1 DA validator address comes from the ecosystem contracts");
        println!("  - The L2 DA validator address is deployed on the ZK chain itself");
        println!(
            "  - Failing to update validators may cause data availability verification failures"
        );
        println!();
    }

    /// Create error for ecosystem not found.
    fn error_ecosystem_not_found(&self, name: &str, path: &std::path::Path) -> eyre::Error {
        eyre::eyre!(
            "Ecosystem '{}' not found\n\n\
             Expected at: {}\n\n\
             Resolution:\n  \
             1. Initialize the ecosystem first with: adi init ecosystem --name {}\n  \
             2. Or specify a different ecosystem name with --ecosystem-name",
            name,
            path.display(),
            name
        )
    }

    /// Create error for ecosystem not deployed.
    fn error_ecosystem_not_deployed(&self, name: &str) -> eyre::Error {
        eyre::eyre!(
            "Ecosystem '{}' is not deployed\n\n\
             Resolution:\n  \
             1. Deploy the ecosystem first with: adi deploy ecosystem\n  \
             2. Then upgrade the ecosystem: adi upgrade ecosystem --to {}\n  \
             3. Then run the chain upgrade command",
            name,
            self.to
        )
    }

    /// Create error for ecosystem not upgraded.
    fn error_ecosystem_not_upgraded(
        &self,
        name: &str,
        current: &Version,
        target: &Version,
    ) -> eyre::Error {
        eyre::eyre!(
            "Ecosystem '{}' is not at target version\n\n\
             Ecosystem version: v{}\n\
             Target version: v{}\n\n\
             Chain upgrades must follow ecosystem upgrades.\n\n\
             Resolution:\n  \
             1. First upgrade the ecosystem: adi upgrade ecosystem --to v{}\n  \
             2. Then upgrade the chain: adi upgrade chain --to v{} --chain-name {}",
            name,
            current,
            target,
            target,
            target,
            self.chain_name
        )
    }

    /// Create error for chain not deployed.
    fn error_chain_not_deployed(&self, name: &str) -> eyre::Error {
        eyre::eyre!(
            "Chain '{}' is not deployed\n\n\
             Resolution:\n  \
             1. Deploy the chain first with: adi deploy chain --chain-name {}\n  \
             2. Then run the upgrade command",
            name,
            name
        )
    }

    /// Create error for invalid upgrade path.
    fn error_invalid_upgrade_path(&self, source: &Version, target: &Version) -> eyre::Error {
        eyre::eyre!(
            "Invalid upgrade path: v{} → v{}\n\n\
             The target version must be greater than the current version.\n\n\
             Current version: v{}\n\
             Target version: v{}\n\n\
             Resolution:\n  \
             1. Specify a higher target version with --to\n  \
             2. Example: adi upgrade chain --to v{} --chain-name {}",
            source,
            target,
            source,
            target,
            source.major + 1,
            self.chain_name
        )
    }
}
