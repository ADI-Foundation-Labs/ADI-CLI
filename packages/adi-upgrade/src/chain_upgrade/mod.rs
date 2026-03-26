//! L2 chain upgrade logic.
//!
//! Generates chain upgrade calldata via zkstack, resolves contracts,
//! and executes upgrade transactions.

mod parse;
mod tx;

use std::path::Path;

use alloy_primitives::Address;
use alloy_provider::Provider;

use crate::error::{Result, UpgradeError};
use crate::onchain;
use crate::simulation::ToolkitRunnerTrait;

pub use parse::{extract_chain_calldatas, ChainCalldatas};
pub use tx::version_to_protocol_uint;

/// Resolved chain contract addresses.
#[derive(Debug, Clone)]
pub struct ChainContracts {
    /// Diamond proxy address (ZK chain).
    pub diamond: Address,
    /// ChainAdmin contract address.
    pub chain_admin: Address,
    /// Chain governor address (owner of ChainAdmin).
    pub chain_governor: Address,
}

/// Result of a chain upgrade.
#[derive(Debug)]
pub struct ChainUpgradeResult {
    /// Chain name that was upgraded.
    pub chain_name: String,
    /// Chain ID.
    pub chain_id: u64,
    /// Whether protocol versions match after upgrade.
    pub versions_match: bool,
}

/// Resolve chain contracts from bridgehub.
///
/// Queries: `bridgehub.getZKChain(chainId)` -> diamond,
/// `diamond.getAdmin()` -> chainAdmin,
/// `chainAdmin.owner()` -> chain governor.
pub async fn resolve_chain_contracts<P: Provider + Clone>(
    provider: &P,
    bridgehub: Address,
    chain_id: u64,
) -> Result<ChainContracts> {
    log::info!("Resolving chain contracts for chain ID {}", chain_id);

    let diamond = onchain::query_zk_chain(provider, bridgehub, chain_id).await?;
    log::info!("Diamond proxy: {}", diamond);

    let chain_admin = onchain::query_admin(provider, diamond).await?;
    log::info!("ChainAdmin: {}", chain_admin);

    let chain_governor = onchain::query_owner(provider, chain_admin).await?;
    log::info!("Chain governor: {}", chain_governor);

    Ok(ChainContracts {
        diamond,
        chain_admin,
        chain_governor,
    })
}

/// Verify that CTM and Diamond protocol versions match.
///
/// Queries both the chain type manager and diamond proxy for their
/// protocol versions and compares them.
pub async fn verify_protocol_versions<P: Provider + Clone>(
    provider: &P,
    bridgehub: Address,
    diamond: Address,
    chain_id: u64,
) -> Result<bool> {
    let ctm = onchain::query_ctm(provider, bridgehub, chain_id).await?;
    let ctm_version = onchain::query_ctm_protocol_version(provider, ctm).await?;
    let diamond_version = onchain::query_diamond_protocol_version(provider, diamond).await?;

    log::info!("CTM protocol version: {}", ctm_version);
    log::info!("Diamond protocol version: {}", diamond_version);

    Ok(ctm_version == diamond_version)
}

/// Parameters for a chain upgrade operation.
pub struct ChainUpgradeParams<'a> {
    /// Chain name to upgrade.
    pub chain_name: &'a str,
    /// Chain ID.
    pub chain_id: u64,
    /// Bridgehub proxy address.
    pub bridgehub: Address,
    /// Governor private key.
    pub governor_key: &'a secrecy::SecretString,
    /// Upgrade name for zkstack.
    pub upgrade_name: &'a str,
    /// Path to the upgrade YAML file.
    pub upgrade_yaml_path: &'a Path,
    /// L1 RPC URL.
    pub l1_rpc_url: &'a str,
    /// L2 RPC URL.
    pub l2_rpc_url: &'a str,
    /// Ecosystem state directory.
    pub state_dir: &'a Path,
    /// Target protocol version.
    pub protocol_version: &'a semver::Version,
}

/// Run the full chain upgrade flow.
///
/// 1. Run zkstack to generate chain upgrade calldata
/// 2. Extract schedule + chainadmin calldatas from output
/// 3. Resolve chain contracts on-chain
/// 4. Send schedule + execute transactions to ChainAdmin
/// 5. Set upgrade timestamp
/// 6. Verify protocol versions match
pub async fn run_chain_upgrade<R, P>(
    runner: &R,
    provider: &P,
    params: &ChainUpgradeParams<'_>,
) -> Result<ChainUpgradeResult>
where
    R: ToolkitRunnerTrait,
    P: Provider + Clone,
{
    log::info!(
        "Starting chain upgrade for '{}' (ID: {})",
        params.chain_name,
        params.chain_id
    );

    // Step 1: Generate chain upgrade calldata via zkstack
    let yaml_filename = params
        .upgrade_yaml_path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| UpgradeError::Config("Invalid upgrade YAML path".into()))?;

    let chain_id_str = params.chain_id.to_string();
    let zkstack_args = vec![
        "dev",
        "generate-chain-upgrade",
        "--upgrade-version",
        params.upgrade_name,
        yaml_filename,
        &chain_id_str,
        &chain_id_str,
        &chain_id_str,
        params.l1_rpc_url,
        params.l2_rpc_url,
        params.l2_rpc_url,
        "0",
        "--force-display-finalization-params=true",
    ];

    let exit_code = runner
        .run_zkstack(
            &zkstack_args,
            params.state_dir,
            params.state_dir,
            params.protocol_version,
        )
        .await
        .map_err(|e| UpgradeError::Config(format!("zkstack generate-chain-upgrade failed: {e}")))?;

    if exit_code != 0 {
        return Err(UpgradeError::Config(format!(
            "zkstack generate-chain-upgrade failed with exit code {}",
            exit_code
        )));
    }

    // Step 2: Extract calldatas from zkstack output (find latest zkstack log)
    let zkstack_log_dir = params.state_dir.join("logs");
    let output_path = std::fs::read_dir(&zkstack_log_dir)
        .map_err(|e| {
            UpgradeError::Config(format!(
                "Failed to read log dir {}: {e}",
                zkstack_log_dir.display()
            ))
        })?
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_string_lossy().starts_with("zkstack_"))
        .max_by_key(|e| e.metadata().and_then(|m| m.modified()).ok())
        .map(|e| e.path())
        .ok_or_else(|| {
            UpgradeError::Config(format!(
                "No zkstack log found in {}",
                zkstack_log_dir.display()
            ))
        })?;

    log::info!("Reading zkstack output from {}", output_path.display());
    let output_content = std::fs::read_to_string(&output_path).map_err(|e| {
        UpgradeError::Config(format!(
            "Failed to read zkstack log at {}: {e}",
            output_path.display()
        ))
    })?;

    let calldatas = extract_chain_calldatas(&output_content)?;
    log::info!(
        "Extracted schedule calldata ({} bytes)",
        calldatas.schedule.len()
    );
    log::info!(
        "Extracted chainadmin calldata ({} bytes)",
        calldatas.chain_admin.len()
    );

    // Step 3: Resolve chain contracts
    let contracts = resolve_chain_contracts(provider, params.bridgehub, params.chain_id).await?;
    log::info!(
        "Chain contracts resolved: diamond={}, admin={}, governor={}",
        contracts.diamond,
        contracts.chain_admin,
        contracts.chain_governor,
    );

    // Step 4: Send schedule + execute transactions to ChainAdmin
    tx::send_chain_tx(
        provider,
        params.governor_key,
        contracts.chain_admin,
        calldatas.schedule,
        "schedule upgrade",
    )
    .await?;

    tx::send_chain_tx(
        provider,
        params.governor_key,
        contracts.chain_admin,
        calldatas.chain_admin,
        "execute upgrade",
    )
    .await?;

    // Step 5: Set upgrade timestamp
    tx::set_upgrade_timestamp(
        provider,
        params.governor_key,
        contracts.chain_admin,
        params.protocol_version,
    )
    .await?;

    // Step 6: Verify protocol versions
    let versions_match = verify_protocol_versions(
        provider,
        params.bridgehub,
        contracts.diamond,
        params.chain_id,
    )
    .await?;

    if versions_match {
        log::info!("Protocol versions match after upgrade");
    } else {
        log::warn!("Protocol versions do NOT match after upgrade");
    }

    Ok(ChainUpgradeResult {
        chain_name: params.chain_name.to_string(),
        chain_id: params.chain_id,
        versions_match,
    })
}
