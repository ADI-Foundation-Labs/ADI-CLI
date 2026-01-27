//! Initialize ecosystem command implementation.
//!
//! This module implements the `adi init ecosystem` command which creates
//! a new ZkSync ecosystem configuration with generated wallets.

use std::path::PathBuf;

use clap::Args;
use eyre::WrapErr;
use serde::{Deserialize, Serialize};

use crate::chain::config::ProverMode;
use crate::context::Context;
use crate::ecosystem::config::SettlementNetwork;
use crate::error::Result;

/// Wallet creation mode for ecosystem initialization.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "snake_case")]
pub enum WalletCreation {
    /// Generate random wallets with new private keys.
    #[default]
    Random,
    /// Use wallets provided via configuration.
    Provided,
}

/// Initialize a new ZkSync ecosystem configuration.
///
/// Creates the ecosystem directory structure and generates wallet keypairs
/// for deployer and governor roles.
///
/// # Example
///
/// ```bash
/// # Initialize with defaults from config
/// adi init ecosystem
///
/// # Initialize with custom name
/// adi init ecosystem --name my_ecosystem
///
/// # Initialize on Sepolia testnet
/// adi init ecosystem --settlement-network sepolia
/// ```
#[derive(Debug, Clone, Args, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InitEcosystem {
    /// Ecosystem name (alphanumeric with underscores, max 64 chars).
    /// If not specified, uses the value from config file.
    #[arg(long)]
    pub name: Option<String>,

    /// Settlement network (mainnet, sepolia, localhost).
    #[arg(long, value_enum, default_value = "localhost")]
    pub settlement_network: SettlementNetworkArg,

    /// Settlement layer RPC endpoint URL.
    /// Overrides the default URL for the selected network.
    #[arg(long)]
    pub settlement_rpc_url: Option<String>,

    /// Initial chain name within the ecosystem.
    /// If not specified, uses the value from config file.
    #[arg(long)]
    pub chain_name: Option<String>,

    /// Initial chain ID.
    /// If not specified, uses the value from config file.
    #[arg(long)]
    pub chain_id: Option<u64>,

    /// Prover mode for the initial chain.
    #[arg(long, value_enum, default_value = "no-proofs")]
    pub prover_mode: ProverModeArg,

    /// Custom base token contract address (for Custom Gas Token chains).
    /// If not specified, ETH is used as the base token.
    #[arg(long)]
    pub base_token_address: Option<String>,

    /// Wallet creation mode.
    #[arg(long, value_enum, default_value = "random")]
    pub wallet_creation: WalletCreation,

    /// State directory path for storing ecosystem data.
    /// Overrides the default state directory from config.
    #[arg(long)]
    pub state_dir: Option<PathBuf>,
}

/// CLI argument for settlement network selection.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "snake_case")]
pub enum SettlementNetworkArg {
    /// Ethereum Mainnet.
    Mainnet,
    /// Sepolia testnet.
    Sepolia,
    /// Local development network.
    #[default]
    Localhost,
}

impl From<SettlementNetworkArg> for SettlementNetwork {
    fn from(arg: SettlementNetworkArg) -> Self {
        match arg {
            SettlementNetworkArg::Mainnet => SettlementNetwork::Mainnet,
            SettlementNetworkArg::Sepolia => SettlementNetwork::Sepolia,
            SettlementNetworkArg::Localhost => SettlementNetwork::Localhost,
        }
    }
}

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

impl InitEcosystem {
    /// Execute the init ecosystem command.
    ///
    /// # Steps
    ///
    /// 1. Validate inputs and resolve defaults from config
    /// 2. Check that ecosystem doesn't already exist
    /// 3. Generate wallet keypairs (deployer, governor)
    /// 4. Create ecosystem directory structure
    /// 5. Save ecosystem metadata, wallets, and initial config
    /// 6. Create initial chain configuration
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Ecosystem already exists
    /// - Wallet generation fails
    /// - Directory creation fails
    /// - State persistence fails
    pub async fn run(&self, context: &Context) -> Result<()> {
        use chrono::Utc;
        use colored::Colorize;
        use semver::Version;

        use crate::ecosystem::config::Ecosystem;
        use crate::ecosystem::wallets::EcosystemWallets;
        use crate::state::FilesystemBackend;
        use crate::state::StateBackend;

        let config = context.config();

        // Resolve ecosystem name
        let ecosystem_name = self
            .name
            .clone()
            .unwrap_or_else(|| config.ecosystem.name.clone());

        // Resolve chain name and ID
        let chain_name = self
            .chain_name
            .clone()
            .unwrap_or_else(|| config.ecosystem.chain_name.clone());
        let chain_id = self.chain_id.unwrap_or(config.ecosystem.chain_id);

        // Resolve state directory
        let state_dir = self
            .state_dir
            .clone()
            .unwrap_or_else(|| config.state_dir.clone());
        let ecosystem_path = state_dir.join(&ecosystem_name);

        // Resolve settlement network (with custom RPC URL if provided)
        let settlement_network = if let Some(ref rpc_url) = self.settlement_rpc_url {
            SettlementNetwork::Custom {
                rpc_url: rpc_url.clone(),
                chain_id: SettlementNetwork::from(self.settlement_network).chain_id(),
            }
        } else {
            SettlementNetwork::from(self.settlement_network)
        };

        context.info(&format!(
            "Initializing ecosystem '{}'",
            ecosystem_name.cyan()
        ));

        // Check if ecosystem already exists
        if ecosystem_path.exists() {
            return Err(eyre::eyre!(
                "Ecosystem '{}' already exists at {}\n\n\
                Resolution:\n  \
                1. Choose a different ecosystem name with --name\n  \
                2. Or remove existing state at {}",
                ecosystem_name,
                ecosystem_path.display(),
                ecosystem_path.display()
            ));
        }

        // Generate wallets
        context.info("Generating wallet keypairs...");
        let wallets =
            EcosystemWallets::generate().wrap_err("Failed to generate ecosystem wallets")?;

        context.info(&format!(
            "Generated deployer wallet: {}",
            format!("{}", wallets.deployer.address).bright_yellow()
        ));
        context.info(&format!(
            "Generated governor wallet: {}",
            format!("{}", wallets.governor.address).bright_yellow()
        ));

        // Create ecosystem structure
        let now = Utc::now();
        let ecosystem = Ecosystem {
            name: ecosystem_name.clone(),
            settlement_network,
            state_path: ecosystem_path.clone(),
            contracts: None,
            wallets: wallets.clone(),
            chains: vec![chain_name.clone()],
            protocol_version: Version::new(29, 0, 11), // Default protocol version
            created_at: now,
            updated_at: now,
        };

        // Validate ecosystem configuration
        ecosystem
            .validate()
            .wrap_err("Invalid ecosystem configuration")?;

        // Create directory structure
        context.info("Creating ecosystem directory structure...");
        ecosystem
            .create_directory_structure()
            .await
            .wrap_err("Failed to create ecosystem directories")?;

        // Create initial chain directory
        ecosystem
            .create_chain_directory(&chain_name)
            .await
            .wrap_err_with(|| format!("Failed to create chain directory for '{}'", chain_name))?;

        context.info(&format!(
            "Creating initial chain '{}' with ID {}",
            chain_name.cyan(),
            chain_id.to_string().cyan()
        ));

        // Save ecosystem state using filesystem backend
        let state_backend = FilesystemBackend::new(state_dir.clone())
            .wrap_err("Failed to initialize state backend")?;

        // Save ecosystem metadata
        let metadata_yaml =
            serde_yaml::to_string(&ecosystem).wrap_err("Failed to serialize ecosystem metadata")?;
        let metadata_key = format!("{}/ZkStack.yaml", ecosystem_name);
        state_backend
            .set(&metadata_key, metadata_yaml.as_bytes())
            .await
            .wrap_err("Failed to save ecosystem metadata")?;

        // Save wallets to configs/wallets.yaml
        // Note: Private keys are not serialized (skipped in serde), which is intentional for security
        // In a production system, private keys would be stored separately with encryption
        let wallets_yaml =
            serde_yaml::to_string(&wallets).wrap_err("Failed to serialize wallets")?;
        let wallets_key = format!("{}/configs/wallets.yaml", ecosystem_name);
        state_backend
            .set(&wallets_key, wallets_yaml.as_bytes())
            .await
            .wrap_err("Failed to save wallets")?;

        // Create empty contracts.yaml placeholder
        let contracts_key = format!("{}/configs/contracts.yaml", ecosystem_name);
        state_backend
            .set(
                &contracts_key,
                b"# Contracts will be populated after deployment\n",
            )
            .await
            .wrap_err("Failed to create contracts placeholder")?;

        // Create chain wallets.yaml placeholder
        let chain_wallets_key = format!(
            "{}/chains/{}/configs/wallets.yaml",
            ecosystem_name, chain_name
        );
        state_backend
            .set(
                &chain_wallets_key,
                b"# Chain wallets will be generated during chain initialization\n",
            )
            .await
            .wrap_err("Failed to create chain wallets placeholder")?;

        // Create chain genesis.yaml placeholder
        let chain_genesis_key = format!(
            "{}/chains/{}/configs/genesis.yaml",
            ecosystem_name, chain_name
        );
        let genesis_content = format!(
            "# Genesis configuration for chain '{}'\nchain_id: {}\nprover_mode: {:?}\n",
            chain_name,
            chain_id,
            ProverMode::from(self.prover_mode)
        );
        state_backend
            .set(&chain_genesis_key, genesis_content.as_bytes())
            .await
            .wrap_err("Failed to create chain genesis placeholder")?;

        // Success output
        context.success(&format!(
            "Ecosystem initialized at {}",
            ecosystem_path.display()
        ));

        println!();
        println!("{}", "State files created:".bright_white().bold());
        println!("  - ZkStack.yaml");
        println!("  - configs/wallets.yaml");
        println!("  - configs/contracts.yaml");
        println!("  - chains/{}/configs/wallets.yaml", chain_name);
        println!("  - chains/{}/configs/genesis.yaml", chain_name);
        println!();
        println!("{}", "Next steps:".bright_white().bold());
        println!(
            "  1. Fund the deployer wallet ({}) with at least 1 ETH",
            wallets.deployer.address
        );
        println!(
            "  2. Fund the governor wallet ({}) with at least 1 ETH",
            wallets.governor.address
        );
        println!(
            "  3. Run: {} to deploy ecosystem contracts",
            "adi deploy ecosystem".cyan()
        );

        Ok(())
    }
}
