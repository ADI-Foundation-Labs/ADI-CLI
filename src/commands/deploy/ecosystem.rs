//! Deploy ecosystem command implementation.
//!
//! This module implements the `adi deploy ecosystem` command which deploys
//! ecosystem smart contracts to the settlement layer.

use std::path::{Path, PathBuf};

use clap::Args;
use colored::Colorize;
use eyre::WrapErr;
use secrecy::ExposeSecret;
use serde::{Deserialize, Serialize};

use crate::context::Context;
use crate::ecosystem::config::Ecosystem;
use crate::ecosystem::contracts::EcosystemContracts;
use crate::error::Result;
use crate::external::{EcosystemInitConfig, ZkstackCli};
use crate::funding::{check_ecosystem_funding, fund_ecosystem_wallets, print_funding_status};
use crate::state::{FilesystemBackend, StateBackend};
use crate::success;

/// Deploy ecosystem contracts to the settlement layer.
///
/// Deploys all ecosystem infrastructure contracts including:
/// - Bridgehub (central hub for chain registration)
/// - Governance contracts
/// - Verifier contracts
/// - DA infrastructure
/// - Token bridges
///
/// # Example
///
/// ```bash
/// # Deploy with auto-funding (requires funder wallet in config)
/// adi deploy ecosystem
///
/// # Deploy specific ecosystem
/// adi deploy ecosystem --name my_ecosystem
///
/// # Deploy with custom gas price
/// adi deploy ecosystem --gas-price 10000000000
///
/// # Dry run (simulate without broadcasting)
/// adi deploy ecosystem --dry-run
/// ```
#[derive(Debug, Clone, Args, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeployEcosystem {
    /// Ecosystem name to deploy.
    /// If not specified, uses the value from config file.
    #[arg(long)]
    pub name: Option<String>,

    /// Settlement layer RPC endpoint URL.
    /// Overrides the value from config file.
    #[arg(long)]
    pub settlement_rpc_url: Option<String>,

    /// Gas price in wei for transactions.
    /// If not specified, gas price is determined automatically.
    #[arg(long)]
    pub gas_price: Option<u64>,

    /// Simulate deployment without broadcasting transactions.
    #[arg(long, default_value = "false")]
    pub dry_run: bool,

    /// Automatically fund wallets from funder wallet if they have insufficient balance.
    /// Requires funder wallet to be configured in config file.
    #[arg(long, default_value = "true")]
    pub auto_fund: bool,

    /// State directory path.
    /// Overrides the default state directory from config.
    #[arg(long)]
    pub state_dir: Option<PathBuf>,
}

impl DeployEcosystem {
    /// Execute the deploy ecosystem command.
    ///
    /// # Steps
    ///
    /// 1. Load ecosystem state from storage
    /// 2. Check if ecosystem is already deployed
    /// 3. Check wallet balances (auto-fund if needed and enabled)
    /// 4. Deploy ecosystem contracts via zkstack
    /// 5. Parse deployed contract addresses
    /// 6. Save contract addresses to state
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Ecosystem doesn't exist or hasn't been initialized
    /// - Ecosystem is already deployed
    /// - Wallet balances are insufficient (and auto-fund is disabled/failed)
    /// - Contract deployment fails
    /// - State persistence fails
    pub async fn run(&self, context: &Context) -> Result<()> {
        let config = context.config();

        // Resolve ecosystem name
        let ecosystem_name = self
            .name
            .clone()
            .unwrap_or_else(|| config.ecosystem.name.clone());

        // Resolve state directory
        let state_dir = self
            .state_dir
            .clone()
            .unwrap_or_else(|| config.state_dir.clone());

        let ecosystem_path = state_dir.join(&ecosystem_name);

        // Resolve RPC URL
        let rpc_url = self
            .settlement_rpc_url
            .clone()
            .unwrap_or_else(|| config.settlement.rpc_url.clone());

        ::log::info!(
            "Deploying ecosystem '{}' to {}",
            ecosystem_name.cyan(),
            rpc_url.bright_blue()
        );

        // Check ecosystem exists
        if !ecosystem_path.exists() {
            return Err(self.error_ecosystem_not_found(&ecosystem_name, &ecosystem_path));
        }

        // Load ecosystem state
        let state_backend = FilesystemBackend::new(state_dir.clone())
            .wrap_err("Failed to initialize state backend")?;

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

        // Check if already deployed
        if ecosystem.is_deployed() {
            return Err(self.error_already_deployed(&ecosystem_name));
        }

        // Phase 1: Check wallet balances
        ::log::info!("{}", "Phase 1: Checking wallet balances...".bright_white());

        let funding_status = check_ecosystem_funding(
            &ecosystem.wallets,
            &rpc_url,
            config.funder.as_ref().and_then(|f| f.cgt_address),
        )
        .await
        .wrap_err("Failed to check wallet funding status")?;

        print_funding_status(&funding_status, None);

        if !funding_status.all_funded {
            if self.auto_fund {
                // Try to auto-fund
                if let Some(ref funder) = config.funder {
                    ::log::info!(
                        "{}",
                        "Auto-funding wallets from funder wallet...".bright_white()
                    );

                    fund_ecosystem_wallets(&ecosystem.wallets, funder, &rpc_url)
                        .await
                        .wrap_err("Failed to auto-fund wallets")?;
                } else {
                    return Err(self.error_insufficient_funds(&funding_status));
                }
            } else {
                return Err(self.error_insufficient_funds(&funding_status));
            }
        } else {
            ::log::info!("{} All wallets sufficiently funded", "✓".green());
        }

        // Dry run check
        if self.dry_run {
            ::log::info!("{}", "Dry run mode - skipping actual deployment".yellow());
            println!();
            println!("{}", "Dry run completed successfully.".green().bold());
            println!("To deploy for real, run without --dry-run");
            return Ok(());
        }

        // Phase 2: Deploy contracts
        ::log::info!(
            "{}",
            "Phase 2: Deploying ecosystem contracts...".bright_white()
        );

        let deployer_pk = ecosystem
            .wallets
            .deployer
            .private_key
            .as_ref()
            .ok_or_else(|| {
                eyre::eyre!(
                    "Deployer wallet private key not found.\n\n\
                     Resolution: Re-initialize the ecosystem with wallet generation."
                )
            })?;

        let governor_pk = ecosystem
            .wallets
            .governor
            .private_key
            .as_ref()
            .ok_or_else(|| {
                eyre::eyre!(
                    "Governor wallet private key not found.\n\n\
                     Resolution: Re-initialize the ecosystem with wallet generation."
                )
            })?;

        // Show progress for deployment phases
        self.log_progress("Initializing deployment");

        let zkstack = ZkstackCli::new();

        let init_config = EcosystemInitConfig {
            ecosystem_path: ecosystem_path.clone(),
            l1_rpc_url: rpc_url.clone(),
            deployer_private_key: deployer_pk.expose_secret().clone(),
            governor_private_key: governor_pk.expose_secret().clone(),
            skip_balance_check: true, // We already checked
            no_verification: true,    // Skip contract verification for now
            gas_price: self.gas_price.or(config.settlement.gas_price),
        };

        self.log_progress("Deploying Bridgehub and core infrastructure");

        let output = zkstack
            .ecosystem_init(&init_config)
            .await
            .wrap_err("Failed to deploy ecosystem contracts")?;

        if !output.success() {
            return Err(self.error_deployment_failed(&output.stderr));
        }

        // Phase 3: Parse deployed contracts
        ::log::info!(
            "{}",
            "Phase 3: Parsing deployed contract addresses...".bright_white()
        );

        self.log_progress("Reading deployment output");

        let contracts_path = ecosystem.contracts_path();

        // Wait for contracts file to be written
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;

        let contracts = if contracts_path.exists() {
            EcosystemContracts::from_yaml_file(&contracts_path)
                .wrap_err("Failed to parse deployed contract addresses")?
        } else {
            return Err(eyre::eyre!(
                "Contracts file not found after deployment: {}\n\n\
                 This may indicate deployment did not complete successfully.\n\
                 Check the zkstack output above for errors.",
                contracts_path.display()
            ));
        };

        // Validate contracts
        contracts
            .validate()
            .wrap_err("Deployed contracts validation failed")?;

        // Phase 4: Persist state
        ::log::info!("{}", "Phase 4: Saving deployment state...".bright_white());

        self.log_progress("Updating ecosystem state");

        // Update ecosystem with contracts
        let mut updated_ecosystem = ecosystem.clone();
        updated_ecosystem.contracts = Some(contracts.clone());
        updated_ecosystem.updated_at = chrono::Utc::now();

        // Save updated metadata
        let metadata_yaml = serde_yaml::to_string(&updated_ecosystem)
            .wrap_err("Failed to serialize ecosystem metadata")?;
        state_backend
            .set(&metadata_key, metadata_yaml.as_bytes())
            .await
            .wrap_err("Failed to save ecosystem metadata")?;

        // Save contracts separately for easy access
        let contracts_yaml =
            serde_yaml::to_string(&contracts).wrap_err("Failed to serialize contracts")?;
        let contracts_key = format!("{}/configs/contracts.yaml", ecosystem_name);
        state_backend
            .set(&contracts_key, contracts_yaml.as_bytes())
            .await
            .wrap_err("Failed to save contracts")?;

        // Success output
        success!("Ecosystem contracts deployed");

        println!();
        println!("{}", "Contract addresses saved to:".bright_white().bold());
        println!("  - configs/contracts.yaml");
        println!();
        println!("{}", "Key addresses:".bright_white().bold());
        println!(
            "  - Bridgehub: {}",
            format!("{}", contracts.bridgehub_proxy_addr).bright_yellow()
        );
        println!(
            "  - Governance: {}",
            format!("{}", contracts.governance_addr).bright_yellow()
        );
        println!(
            "  - Chain Admin: {}",
            format!("{}", contracts.chain_admin_addr).bright_yellow()
        );
        println!(
            "  - Verifier: {}",
            format!("{}", contracts.verifier_addr).bright_yellow()
        );
        println!();
        println!("{}", "Next steps:".bright_white().bold());
        println!(
            "  1. Run: {} to initialize a chain",
            "adi init chain".cyan()
        );
        println!(
            "  2. Run: {} to deploy chain contracts",
            "adi deploy chain".cyan()
        );

        Ok(())
    }

    /// Log a progress message.
    fn log_progress(&self, message: &str) {
        ::log::info!("{} {}...", "[PROGRESS]".bright_blue(), message);
    }

    /// Create error for ecosystem not found.
    fn error_ecosystem_not_found(&self, name: &str, path: &Path) -> eyre::Error {
        eyre::eyre!(
            "Ecosystem '{}' not found\n\n\
             Expected at: {}\n\n\
             Resolution:\n  \
             1. Initialize the ecosystem first with: adi init ecosystem --name {}\n  \
             2. Or specify a different ecosystem name with --name",
            name,
            path.display(),
            name
        )
    }

    /// Create error for already deployed.
    fn error_already_deployed(&self, name: &str) -> eyre::Error {
        eyre::eyre!(
            "Ecosystem '{}' is already deployed\n\n\
             Resolution:\n  \
             1. To upgrade, use: adi upgrade ecosystem\n  \
             2. To redeploy, remove existing state and re-initialize",
            name
        )
    }

    /// Create error for insufficient funds.
    fn error_insufficient_funds(&self, status: &crate::funding::FundingCheckResult) -> eyre::Error {
        let underfunded = status.underfunded_wallets();
        let mut details = String::new();

        for wallet in &underfunded {
            let deficit = wallet.eth_deficit();
            let deficit_eth = deficit.to_string().parse::<f64>().unwrap_or(0.0) / 1e18;
            details.push_str(&format!(
                "\n  - {}: needs {:.4} more ETH",
                wallet.requirement.name, deficit_eth
            ));
        }

        eyre::eyre!(
            "Insufficient balance in ecosystem wallets\n\n\
             Details:{}\n\n\
             Resolution:\n  \
             1. Fund the wallets manually with the required amounts\n  \
             2. Or configure a funder wallet in ~/.adi_cli/.adi.yml:\n     \
             funder:\n       \
             private_key: \"0x...\"\n  \
             3. Re-run: adi deploy ecosystem",
            details
        )
    }

    /// Create error for deployment failure.
    fn error_deployment_failed(&self, stderr: &str) -> eyre::Error {
        // Try to provide actionable guidance based on error content
        let guidance = if stderr.contains("insufficient funds") {
            "Ensure deployer wallet has sufficient ETH for gas"
        } else if stderr.contains("nonce") {
            "Transaction nonce issue - try waiting and retrying"
        } else if stderr.contains("gas") {
            "Gas estimation failed - try specifying --gas-price"
        } else if stderr.contains("revert") {
            "Contract deployment reverted - check contract compatibility"
        } else if stderr.contains("connection") || stderr.contains("timeout") {
            "Network connection issue - verify RPC URL is accessible"
        } else {
            "Check the error output above for details"
        };

        eyre::eyre!(
            "Ecosystem contract deployment failed\n\n\
             Error: {}\n\n\
             Guidance: {}\n\n\
             Resolution:\n  \
             1. Fix the issue described above\n  \
             2. Re-run: adi deploy ecosystem",
            stderr.trim(),
            guidance
        )
    }
}
