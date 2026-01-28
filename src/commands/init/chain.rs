//! Initialize chain command implementation.
//!
//! This module implements the `adi init chain` command which creates
//! a new chain configuration within an existing ecosystem.

use std::path::PathBuf;

use chrono::Utc;
use clap::Args;
use colored::Colorize;
use eyre::WrapErr;
use serde::{Deserialize, Serialize};

use crate::chain::config::{BaseToken, Chain, ChainState, ProverMode};
use crate::chain::wallets::ChainWallets;
use crate::context::Context;
use crate::ecosystem::config::Ecosystem;
use crate::error::Result;
use crate::state::{FilesystemBackend, StateBackend};
use crate::success;

/// CLI argument for prover mode selection.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum ProverModeArg {
    /// No proofs (for development/testing).
    #[default]
    NoProofs,
    /// GPU-based proving.
    Gpu,
}

impl From<ProverModeArg> for ProverMode {
    fn from(arg: ProverModeArg) -> Self {
        match arg {
            ProverModeArg::NoProofs => ProverMode::NoProofs,
            ProverModeArg::Gpu => ProverMode::Gpu,
        }
    }
}

/// Initialize a new chain configuration within an ecosystem.
///
/// Creates the chain directory structure and generates wallet keypairs
/// for deployer, governor, and operator roles.
///
/// # Example
///
/// ```bash
/// # Initialize chain with required options
/// adi init chain --name my_chain --chain-id 270
///
/// # Initialize chain with custom base token
/// adi init chain --name my_chain --chain-id 270 --base-token-address 0x...
///
/// # Initialize chain with GPU prover
/// adi init chain --name my_chain --chain-id 270 --prover-mode gpu
/// ```
#[derive(Debug, Clone, Args, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InitChain {
    /// Parent ecosystem name.
    /// If not specified, uses the value from config file.
    #[arg(long)]
    pub ecosystem_name: Option<String>,

    /// Chain name (alphanumeric with underscores, max 64 chars).
    #[arg(long)]
    pub name: String,

    /// Chain ID (must not conflict with settlement layer networks).
    #[arg(long)]
    pub chain_id: u64,

    /// Custom base token contract address (for Custom Gas Token chains).
    /// If not specified, ETH is used as the base token.
    #[arg(long)]
    pub base_token_address: Option<String>,

    /// Prover mode for the chain.
    #[arg(long, value_enum, default_value = "no-proofs")]
    pub prover_mode: ProverModeArg,

    /// State directory path for storing chain data.
    /// Overrides the default state directory from config.
    #[arg(long)]
    pub state_dir: Option<PathBuf>,
}

impl InitChain {
    /// Execute the init chain command.
    ///
    /// # Steps
    ///
    /// 1. Validate inputs and resolve defaults from config
    /// 2. Load and validate parent ecosystem exists
    /// 3. Check that chain doesn't already exist
    /// 4. Generate wallet keypairs (deployer, governor, operators)
    /// 5. Create chain directory structure
    /// 6. Save chain wallets and genesis configuration
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Parent ecosystem doesn't exist
    /// - Chain already exists within ecosystem
    /// - Wallet generation fails
    /// - Directory creation fails
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

        ::log::info!(
            "Initializing chain '{}' (ID: {}) in ecosystem '{}'",
            self.name.cyan(),
            self.chain_id.to_string().cyan(),
            ecosystem_name.cyan()
        );

        // Check if ecosystem exists
        if !ecosystem_path.exists() {
            return Err(eyre::eyre!(
                "Ecosystem '{}' does not exist at {}\n\n\
                Resolution:\n  \
                1. Initialize the ecosystem first with: adi init ecosystem --name {}\n  \
                2. Or specify the correct ecosystem name with --ecosystem-name",
                ecosystem_name,
                ecosystem_path.display(),
                ecosystem_name
            ));
        }

        // Load ecosystem metadata to validate it
        let state_backend = FilesystemBackend::new(state_dir.clone())
            .wrap_err("Failed to initialize state backend")?;

        let metadata_key = format!("{}/ZkStack.yaml", ecosystem_name);
        let ecosystem_data = state_backend
            .get(&metadata_key)
            .await
            .wrap_err("Failed to read ecosystem metadata")?
            .ok_or_else(|| {
                eyre::eyre!(
                    "Ecosystem '{}' metadata not found. The ecosystem may be corrupted.\n\n\
                    Resolution:\n  \
                    1. Re-initialize the ecosystem with: adi init ecosystem --name {}",
                    ecosystem_name,
                    ecosystem_name
                )
            })?;

        let ecosystem: Ecosystem = serde_yaml::from_slice(&ecosystem_data)
            .wrap_err("Failed to parse ecosystem metadata")?;

        // Check if chain already exists
        let chain_path = ecosystem_path.join("chains").join(&self.name);
        if chain_path.exists() {
            return Err(eyre::eyre!(
                "Chain '{}' already exists in ecosystem '{}' at {}\n\n\
                Resolution:\n  \
                1. Choose a different chain name with --name\n  \
                2. Or remove existing chain at {}",
                self.name,
                ecosystem_name,
                chain_path.display(),
                chain_path.display()
            ));
        }

        // Check if chain name already registered in ecosystem
        if ecosystem.chains.contains(&self.name) {
            return Err(eyre::eyre!(
                "Chain '{}' is already registered in ecosystem '{}'\n\n\
                Resolution:\n  \
                1. Choose a different chain name with --name",
                self.name,
                ecosystem_name
            ));
        }

        // Parse base token
        let base_token = if let Some(ref addr) = self.base_token_address {
            let address = addr
                .parse()
                .wrap_err_with(|| format!("Invalid base token address: {}", addr))?;
            BaseToken::Custom {
                address,
                symbol: "CGT".to_string(), // Default symbol, can be updated later
                decimals: 18,
            }
        } else {
            BaseToken::Eth
        };

        // Generate chain wallets
        ::log::info!("Generating chain wallet keypairs...");
        let wallets = ChainWallets::generate().wrap_err("Failed to generate chain wallets")?;

        ::log::info!(
            "Generated chain deployer wallet: {}",
            format!("{}", wallets.deployer.address).bright_yellow()
        );
        ::log::info!(
            "Generated chain governor wallet: {}",
            format!("{}", wallets.governor.address).bright_yellow()
        );
        ::log::info!(
            "Generated chain operator wallet: {}",
            format!("{}", wallets.operator.address).bright_yellow()
        );

        // Create chain configuration
        let now = Utc::now();
        let chain = Chain {
            name: self.name.clone(),
            chain_id: self.chain_id,
            ecosystem_name: ecosystem_name.clone(),
            base_token,
            prover_mode: ProverMode::from(self.prover_mode),
            contracts: None,
            wallets: wallets.clone(),
            state: ChainState::Initialized,
            created_at: now,
            updated_at: now,
        };

        // Validate chain configuration
        chain.validate().wrap_err("Invalid chain configuration")?;

        // Create chain directory structure
        ::log::info!("Creating chain directory structure...");
        chain
            .create_directory_structure(&ecosystem_path)
            .await
            .wrap_err("Failed to create chain directories")?;

        // Save chain wallets to configs/wallets.yaml
        let wallets_yaml =
            serde_yaml::to_string(&wallets).wrap_err("Failed to serialize wallets")?;
        let wallets_key = format!(
            "{}/chains/{}/configs/wallets.yaml",
            ecosystem_name, self.name
        );
        state_backend
            .set(&wallets_key, wallets_yaml.as_bytes())
            .await
            .wrap_err("Failed to save chain wallets")?;

        // Save chain genesis.yaml
        let genesis_content = self.generate_genesis_yaml(&chain);
        let genesis_key = format!(
            "{}/chains/{}/configs/genesis.yaml",
            ecosystem_name, self.name
        );
        state_backend
            .set(&genesis_key, genesis_content.as_bytes())
            .await
            .wrap_err("Failed to save chain genesis configuration")?;

        // Create empty contracts.yaml placeholder
        let contracts_key = format!(
            "{}/chains/{}/configs/contracts.yaml",
            ecosystem_name, self.name
        );
        state_backend
            .set(
                &contracts_key,
                b"# Chain contracts will be populated after deployment\n",
            )
            .await
            .wrap_err("Failed to create chain contracts placeholder")?;

        // Update ecosystem metadata to include this chain
        let mut updated_ecosystem = ecosystem;
        updated_ecosystem.chains.push(self.name.clone());
        updated_ecosystem.updated_at = now;

        let updated_metadata_yaml = serde_yaml::to_string(&updated_ecosystem)
            .wrap_err("Failed to serialize updated ecosystem metadata")?;
        state_backend
            .set(&metadata_key, updated_metadata_yaml.as_bytes())
            .await
            .wrap_err("Failed to update ecosystem metadata")?;

        // Success output
        success!("Chain '{}' initialized", self.name);

        println!();
        println!("{}", "Chain configuration saved to:".bright_white().bold());
        println!("  - chains/{}/configs/wallets.yaml", self.name);
        println!("  - chains/{}/configs/genesis.yaml", self.name);
        println!();
        println!("{}", "Next steps:".bright_white().bold());
        println!(
            "  1. Fund the chain deployer wallet ({}) with at least 1 ETH",
            wallets.deployer.address
        );
        println!(
            "  2. Fund the chain governor wallet ({}) with at least 1 ETH",
            wallets.governor.address
        );
        println!(
            "  3. Fund the chain operator wallet ({}) with at least 5 ETH",
            wallets.operator.address
        );
        println!(
            "  4. Run: {} to deploy chain contracts",
            format!("adi deploy chain --name {}", self.name).cyan()
        );

        Ok(())
    }

    /// Generates the genesis.yaml content for the chain.
    fn generate_genesis_yaml(&self, chain: &Chain) -> String {
        let base_token_section = match &chain.base_token {
            BaseToken::Eth => "base_token: eth\n".to_string(),
            BaseToken::Custom {
                address,
                symbol,
                decimals,
            } => {
                format!(
                    "base_token:\n  type: custom\n  address: \"{}\"\n  symbol: \"{}\"\n  decimals: {}\n",
                    address, symbol, decimals
                )
            }
        };

        format!(
            "# Genesis configuration for chain '{}'\n\
            chain_id: {}\n\
            prover_mode: {}\n\
            {}\n\
            # Additional genesis parameters will be added during deployment\n",
            chain.name, chain.chain_id, chain.prover_mode, base_token_section
        )
    }
}
