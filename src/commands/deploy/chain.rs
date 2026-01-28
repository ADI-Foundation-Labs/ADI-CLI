//! Deploy chain command implementation.
//!
//! This module implements the `adi deploy chain` command which deploys
//! chain contracts to the settlement layer and registers the chain with Bridgehub.

use std::path::{Path, PathBuf};

use clap::Args;
use colored::Colorize;
use eyre::WrapErr;
use secrecy::ExposeSecret;
use serde::{Deserialize, Serialize};

use crate::chain::config::Chain;
use crate::chain::contracts::ChainContracts;
use crate::context::Context;
use crate::ecosystem::config::Ecosystem;
use crate::error::Result;
use crate::external::{ChainInitConfig, ZkstackCli};
use crate::funding::{check_chain_funding, fund_chain_wallets, print_funding_status};
use crate::state::{FilesystemBackend, StateBackend};
use crate::success;

/// Deploy chain contracts to the settlement layer.
///
/// Deploys chain infrastructure contracts and registers the chain with Bridgehub:
/// - Diamond Proxy (main L2 contract)
/// - Chain Admin contract
/// - Chain Governance
/// - Settlement layer bridges
/// - L2 bridges
///
/// # Example
///
/// ```bash
/// # Deploy a chain
/// adi deploy chain --name my_chain
///
/// # Deploy with custom gas price
/// adi deploy chain --name my_chain --gas-price 10000000000
///
/// # Dry run (simulate without broadcasting)
/// adi deploy chain --name my_chain --dry-run
/// ```
#[derive(Debug, Clone, Args, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeployChain {
    /// Chain name to deploy.
    #[arg(long)]
    pub name: String,

    /// Parent ecosystem name.
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

impl DeployChain {
    /// Execute the deploy chain command.
    ///
    /// # Steps
    ///
    /// 1. Load ecosystem and chain state from storage
    /// 2. Check if ecosystem is deployed (prerequisite)
    /// 3. Check if chain is already deployed
    /// 4. Check wallet balances (auto-fund if needed and enabled)
    /// 5. Deploy chain contracts via zkstack
    /// 6. Register chain with Bridgehub
    /// 7. Parse deployed contract addresses
    /// 8. Save contract addresses to state
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Ecosystem doesn't exist or hasn't been deployed
    /// - Chain doesn't exist or hasn't been initialized
    /// - Chain is already deployed
    /// - Wallet balances are insufficient (and auto-fund is disabled/failed)
    /// - Contract deployment fails
    /// - State persistence fails
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

        // Resolve RPC URL
        let rpc_url = self
            .settlement_rpc_url
            .clone()
            .unwrap_or_else(|| config.settlement.rpc_url.clone());

        ::log::info!(
            "Deploying chain '{}' (ecosystem '{}') to {}",
            self.name.cyan(),
            ecosystem_name.bright_white(),
            rpc_url.bright_blue()
        );

        // Check ecosystem exists
        if !ecosystem_path.exists() {
            return Err(self.error_ecosystem_not_found(&ecosystem_name, &ecosystem_path));
        }

        // Load state backend
        let state_backend = FilesystemBackend::new(state_dir.clone())
            .wrap_err("Failed to initialize state backend")?;

        // Load ecosystem state
        let ecosystem = self.load_ecosystem(&state_backend, &ecosystem_name).await?;

        // Verify ecosystem is deployed
        if !ecosystem.is_deployed() {
            return Err(self.error_ecosystem_not_deployed(&ecosystem_name));
        }

        // Load chain state
        let chain_path = ecosystem_path.join("chains").join(&self.name);
        if !chain_path.exists() {
            return Err(self.error_chain_not_found(&self.name, &ecosystem_name, &chain_path));
        }

        let chain = self
            .load_chain(&state_backend, &ecosystem_name, &self.name)
            .await?;

        // Check if already deployed
        if chain.is_deployed() {
            return Err(self.error_already_deployed(&self.name));
        }

        // Phase 1: Check wallet balances
        ::log::info!("{}", "Phase 1: Checking wallet balances...".bright_white());

        let funding_status = check_chain_funding(
            &chain.wallets,
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

                    fund_chain_wallets(&chain.wallets, funder, &rpc_url)
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

        // Phase 2: Deploy contracts and register with Bridgehub
        ::log::info!("{}", "Phase 2: Deploying chain contracts...".bright_white());

        let deployer_pk = chain.wallets.deployer.private_key.as_ref().ok_or_else(|| {
            eyre::eyre!(
                "Chain deployer wallet private key not found.\n\n\
                 Resolution: Re-initialize the chain with wallet generation."
            )
        })?;

        let governor_pk = chain.wallets.governor.private_key.as_ref().ok_or_else(|| {
            eyre::eyre!(
                "Chain governor wallet private key not found.\n\n\
                 Resolution: Re-initialize the chain with wallet generation."
            )
        })?;

        // Show progress for deployment phases
        self.log_progress("Initializing chain deployment");

        let zkstack = ZkstackCli::new();

        let init_config = ChainInitConfig {
            ecosystem_path: ecosystem_path.clone(),
            chain_name: self.name.clone(),
            l1_rpc_url: rpc_url.clone(),
            deployer_private_key: deployer_pk.expose_secret().clone(),
            governor_private_key: governor_pk.expose_secret().clone(),
            skip_balance_check: true, // We already checked
            no_verification: true,    // Skip contract verification for now
            gas_price: self.gas_price.or(config.settlement.gas_price),
        };

        self.log_progress("Deploying Diamond Proxy and chain contracts");

        let output = zkstack
            .chain_init(&init_config)
            .await
            .wrap_err("Failed to deploy chain contracts")?;

        if !output.success() {
            return Err(self.error_deployment_failed(&output.stderr));
        }

        self.log_progress("Registering chain with Bridgehub");

        // The chain init command registers with Bridgehub automatically

        // Phase 3: Parse deployed contracts
        ::log::info!(
            "{}",
            "Phase 3: Parsing deployed contract addresses...".bright_white()
        );

        self.log_progress("Reading deployment output");

        let contracts_path = chain.contracts_path(&ecosystem_path);

        // Wait for contracts file to be written
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;

        let contracts = if contracts_path.exists() {
            ChainContracts::from_yaml_file(&contracts_path)
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

        self.log_progress("Updating chain state");

        // Update chain with contracts
        let mut updated_chain = chain.clone();
        updated_chain.contracts = Some(contracts.clone());
        updated_chain.state = crate::chain::config::ChainState::Deployed;
        updated_chain.updated_at = chrono::Utc::now();

        // Save updated chain metadata
        let chain_key = format!("{}/chains/{}/ZkStack.yaml", ecosystem_name, self.name);
        let chain_yaml =
            serde_yaml::to_string(&updated_chain).wrap_err("Failed to serialize chain metadata")?;
        state_backend
            .set(&chain_key, chain_yaml.as_bytes())
            .await
            .wrap_err("Failed to save chain metadata")?;

        // Save contracts separately for easy access
        let contracts_yaml =
            serde_yaml::to_string(&contracts).wrap_err("Failed to serialize contracts")?;
        let contracts_key = format!(
            "{}/chains/{}/configs/contracts.yaml",
            ecosystem_name, self.name
        );
        state_backend
            .set(&contracts_key, contracts_yaml.as_bytes())
            .await
            .wrap_err("Failed to save contracts")?;

        // Success output
        success!("Chain '{}' deployed and registered", self.name);

        println!();
        println!("{}", "Contract addresses saved to:".bright_white().bold());
        println!("  - chains/{}/configs/contracts.yaml", self.name);
        println!();
        println!("{}", "Key addresses:".bright_white().bold());
        println!(
            "  - Diamond Proxy: {}",
            format!("{}", contracts.diamond_proxy_addr).bright_yellow()
        );
        println!(
            "  - Chain Admin: {}",
            format!("{}", contracts.chain_admin_addr).bright_yellow()
        );
        println!(
            "  - Governance: {}",
            format!("{}", contracts.governance_addr).bright_yellow()
        );
        println!();
        println!("{}", "Next steps:".bright_white().bold());
        println!("  1. Start the chain server (external operation)");
        println!(
            "  2. Run: {} to upgrade chain contracts",
            "adi upgrade chain".cyan()
        );

        Ok(())
    }

    /// Load ecosystem from state backend.
    async fn load_ecosystem(
        &self,
        state_backend: &FilesystemBackend,
        ecosystem_name: &str,
    ) -> Result<Ecosystem> {
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

        serde_yaml::from_slice(&metadata_bytes).wrap_err("Failed to parse ecosystem metadata")
    }

    /// Load chain from state backend.
    async fn load_chain(
        &self,
        state_backend: &FilesystemBackend,
        ecosystem_name: &str,
        chain_name: &str,
    ) -> Result<Chain> {
        let metadata_key = format!("{}/chains/{}/ZkStack.yaml", ecosystem_name, chain_name);
        let metadata_bytes = state_backend
            .get(&metadata_key)
            .await
            .wrap_err("Failed to read chain metadata")?
            .ok_or_else(|| {
                eyre::eyre!(
                    "Chain '{}' metadata not found. Run 'adi init chain' first.",
                    chain_name
                )
            })?;

        serde_yaml::from_slice(&metadata_bytes).wrap_err("Failed to parse chain metadata")
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
             2. Or specify a different ecosystem name with --ecosystem-name",
            name,
            path.display(),
            name
        )
    }

    /// Create error for ecosystem not deployed.
    fn error_ecosystem_not_deployed(&self, name: &str) -> eyre::Error {
        eyre::eyre!(
            "Ecosystem '{}' has not been deployed\n\n\
             Chain deployment requires ecosystem contracts to be deployed first.\n\n\
             Resolution:\n  \
             1. Deploy the ecosystem with: adi deploy ecosystem --name {}\n  \
             2. Then re-run: adi deploy chain --name {}",
            name,
            name,
            self.name
        )
    }

    /// Create error for chain not found.
    fn error_chain_not_found(
        &self,
        chain_name: &str,
        ecosystem_name: &str,
        path: &Path,
    ) -> eyre::Error {
        eyre::eyre!(
            "Chain '{}' not found in ecosystem '{}'\n\n\
             Expected at: {}\n\n\
             Resolution:\n  \
             1. Initialize the chain first with: adi init chain --name {} --ecosystem-name {}\n  \
             2. Or specify a different chain name with --name",
            chain_name,
            ecosystem_name,
            path.display(),
            chain_name,
            ecosystem_name
        )
    }

    /// Create error for already deployed.
    fn error_already_deployed(&self, name: &str) -> eyre::Error {
        eyre::eyre!(
            "Chain '{}' is already deployed\n\n\
             Resolution:\n  \
             1. To upgrade, use: adi upgrade chain --name {}\n  \
             2. To redeploy, remove existing state and re-initialize",
            name,
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
            "Insufficient balance in chain wallets\n\n\
             Details:{}\n\n\
             Resolution:\n  \
             1. Fund the wallets manually with the required amounts\n  \
             2. Or configure a funder wallet in ~/.adi_cli/.adi.yml:\n     \
             funder:\n       \
             private_key: \"0x...\"\n  \
             3. Re-run: adi deploy chain --name {}",
            details,
            self.name
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
        } else if stderr.contains("bridgehub") {
            "Bridgehub registration failed - ensure ecosystem contracts are deployed"
        } else {
            "Check the error output above for details"
        };

        eyre::eyre!(
            "Chain contract deployment failed\n\n\
             Error: {}\n\n\
             Guidance: {}\n\n\
             Resolution:\n  \
             1. Fix the issue described above\n  \
             2. Re-run: adi deploy chain --name {}",
            stderr.trim(),
            guidance,
            self.name
        )
    }
}
