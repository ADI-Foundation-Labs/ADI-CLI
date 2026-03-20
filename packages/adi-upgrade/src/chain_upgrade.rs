//! L2 chain upgrade logic.
//!
//! Generates chain upgrade calldata via zkstack, resolves contracts,
//! and executes upgrade transactions.

use std::path::Path;

use alloy_network::EthereumWallet;
use alloy_primitives::{Address, Bytes, U256};
use alloy_provider::{Provider, ProviderBuilder};
use alloy_rpc_types::TransactionRequest;
use alloy_signer_local::PrivateKeySigner;
use secrecy::ExposeSecret;

use crate::error::{Result, UpgradeError};
use crate::onchain;
use crate::simulation::ToolkitRunnerTrait;

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

/// Convert semver Version to protocol version uint256.
///
/// Formula: `(major << 40) | (minor << 32) | patch`
///
/// # Examples
///
/// - v0.30.0 -> `0x1e00000000`
/// - v0.30.1 -> `0x1e00000001`
pub fn version_to_protocol_uint(version: &semver::Version) -> U256 {
    let major = U256::from(version.major);
    let minor = U256::from(version.minor);
    let patch = U256::from(version.patch);

    (major << 40) | (minor << 32) | patch
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

/// Extracted calldatas from zkstack chain-upgrade output.
#[derive(Debug)]
pub struct ChainCalldatas {
    /// Schedule upgrade calldata (sent to ChainAdmin).
    pub schedule: Bytes,
    /// ChainAdmin full calldata (execute upgrade).
    pub chain_admin: Bytes,
}

/// Extract chain upgrade calldatas from zkstack output file.
///
/// Parses the `chain-upgrade.txt` output to find:
/// - Schedule calldata from "Calldata to schedule upgrade" section
/// - ChainAdmin calldata from "Full calldata to call `ChainAdmin` with" section
pub fn extract_chain_calldatas(output: &str) -> Result<ChainCalldatas> {
    // Extract schedule calldata: find "data": "0x..." in the schedule section
    let schedule_hex = extract_schedule_calldata(output)?;
    let schedule = hex::decode(schedule_hex.trim_start_matches("0x"))
        .map_err(|e| UpgradeError::Config(format!("Invalid schedule calldata hex: {e}")))?;

    // Extract chainadmin calldata: hex line after "Full calldata to call `ChainAdmin` with"
    let chain_admin_hex = extract_chainadmin_calldata(output)?;
    let chain_admin = hex::decode(chain_admin_hex.trim_start_matches("0x"))
        .map_err(|e| UpgradeError::Config(format!("Invalid chainadmin calldata hex: {e}")))?;

    Ok(ChainCalldatas {
        schedule: Bytes::from(schedule),
        chain_admin: Bytes::from(chain_admin),
    })
}

/// Extract schedule calldata from "Calldata to schedule upgrade" section.
fn extract_schedule_calldata(output: &str) -> Result<String> {
    let marker = "Calldata to schedule upgrade";
    let section_start = output
        .find(marker)
        .ok_or_else(|| UpgradeError::Config("Schedule calldata section not found".into()))?;

    let section = &output[section_start..];

    // Find "data": "0x..." pattern
    let data_marker = "\"data\":";
    let data_pos = section
        .find(data_marker)
        .ok_or_else(|| UpgradeError::Config("Schedule calldata data field not found".into()))?;

    let after_data = &section[data_pos + data_marker.len()..];

    // Find the hex value between quotes
    let quote_start = after_data
        .find('"')
        .ok_or_else(|| UpgradeError::Config("Schedule calldata: missing opening quote".into()))?;
    let hex_start = quote_start + 1;

    let quote_end = after_data[hex_start..]
        .find('"')
        .ok_or_else(|| UpgradeError::Config("Schedule calldata: missing closing quote".into()))?;

    Ok(after_data[hex_start..hex_start + quote_end].to_string())
}

/// Extract chainadmin calldata from "Full calldata to call `ChainAdmin` with" section.
fn extract_chainadmin_calldata(output: &str) -> Result<String> {
    let marker = "Full calldata to call `ChainAdmin` with";
    let section_start = output
        .find(marker)
        .ok_or_else(|| UpgradeError::Config("ChainAdmin calldata section not found".into()))?;

    let after_marker = &output[section_start + marker.len()..];

    // The hex appears on the next non-empty line
    for line in after_marker.lines().skip(1) {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        // Extract hex characters
        let hex: String = trimmed.chars().filter(|c| c.is_ascii_hexdigit()).collect();
        if hex.len() >= 8 {
            return Ok(format!("0x{hex}"));
        }
    }

    Err(UpgradeError::Config(
        "ChainAdmin calldata not found after marker".into(),
    ))
}

/// Send a raw transaction to a contract address.
async fn send_chain_tx<P: Provider + Clone>(
    provider: &P,
    signer_key: &secrecy::SecretString,
    to: Address,
    calldata: Bytes,
    label: &str,
) -> Result<alloy_primitives::B256> {
    let key_str = signer_key.expose_secret();
    let key_hex = key_str.strip_prefix("0x").unwrap_or(key_str);
    let key_bytes: [u8; 32] = hex::decode(key_hex)
        .map_err(|e| UpgradeError::GovernanceFailed(format!("Invalid key hex: {e}")))?
        .try_into()
        .map_err(|_| UpgradeError::GovernanceFailed("Key must be 32 bytes".into()))?;

    let signer = PrivateKeySigner::from_bytes(&key_bytes.into())
        .map_err(|e| UpgradeError::GovernanceFailed(format!("Invalid key: {e}")))?;

    let wallet = EthereumWallet::from(signer);
    let signing_provider = ProviderBuilder::new()
        .wallet(wallet)
        .connect_provider(provider.clone());

    log::info!("Sending {label} tx to {to}...");

    let tx = TransactionRequest::default()
        .to(to)
        .input(calldata.into())
        .value(U256::ZERO);

    let pending = signing_provider
        .send_transaction(tx)
        .await
        .map_err(|e| UpgradeError::GovernanceFailed(format!("{label} tx failed: {e}")))?;

    let receipt = pending
        .get_receipt()
        .await
        .map_err(|e| UpgradeError::GovernanceFailed(format!("{label} receipt failed: {e}")))?;

    log::info!("{label} tx: {}", receipt.transaction_hash);
    Ok(receipt.transaction_hash)
}

/// Call `ChainAdmin.setUpgradeTimestamp(uint256 protocolVersion, uint256 upgradeTimestamp)`.
async fn set_upgrade_timestamp<P: Provider + Clone>(
    provider: &P,
    signer_key: &secrecy::SecretString,
    chain_admin: Address,
    protocol_version: &semver::Version,
) -> Result<alloy_primitives::B256> {
    use alloy_sol_types::SolCall;

    alloy_sol_types::sol! {
        function setUpgradeTimestamp(uint256 protocolVersion, uint256 upgradeTimestamp) external;
    }

    let version_uint = version_to_protocol_uint(protocol_version);

    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let timestamp = U256::from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| UpgradeError::Config(format!("Failed to get timestamp: {e}")))?
            .as_secs()
            + 1,
    );

    log::info!(
        "Setting upgrade timestamp: version={}, timestamp={}",
        version_uint,
        timestamp
    );

    let calldata = setUpgradeTimestampCall {
        protocolVersion: version_uint,
        upgradeTimestamp: timestamp,
    }
    .abi_encode();

    send_chain_tx(
        provider,
        signer_key,
        chain_admin,
        Bytes::from(calldata),
        "setUpgradeTimestamp",
    )
    .await
}

/// Run the full chain upgrade flow.
///
/// 1. Run zkstack to generate chain upgrade calldata
/// 2. Extract schedule + chainadmin calldatas from output
/// 3. Resolve chain contracts on-chain
/// 4. Send schedule + execute transactions to ChainAdmin
/// 5. Set upgrade timestamp
/// 6. Verify protocol versions match
#[allow(clippy::too_many_arguments)]
pub async fn run_chain_upgrade<R, P>(
    runner: &R,
    provider: &P,
    chain_name: &str,
    chain_id: u64,
    bridgehub: Address,
    governor_key: &secrecy::SecretString,
    upgrade_name: &str,
    upgrade_yaml_path: &Path,
    l1_rpc_url: &str,
    l2_rpc_url: &str,
    state_dir: &Path,
    protocol_version: &semver::Version,
) -> Result<ChainUpgradeResult>
where
    R: ToolkitRunnerTrait,
    P: Provider + Clone,
{
    log::info!(
        "Starting chain upgrade for '{}' (ID: {})",
        chain_name,
        chain_id
    );

    // Step 1: Generate chain upgrade calldata via zkstack
    let yaml_filename = upgrade_yaml_path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| UpgradeError::Config("Invalid upgrade YAML path".into()))?;

    let log_dir = state_dir.join("logs");
    std::fs::create_dir_all(&log_dir)
        .map_err(|e| UpgradeError::Config(format!("Failed to create log dir: {e}")))?;

    let chain_id_str = chain_id.to_string();
    let zkstack_args = vec![
        "dev",
        "generate-chain-upgrade",
        "--upgrade-version",
        upgrade_name,
        yaml_filename,
        &chain_id_str,
        &chain_id_str,
        &chain_id_str,
        l1_rpc_url,
        l2_rpc_url,
        l2_rpc_url,
        "0",
        "--force-display-finalization-params=true",
    ];

    let exit_code = runner
        .run_zkstack(&zkstack_args, state_dir, &log_dir, protocol_version)
        .await
        .map_err(|e| UpgradeError::Config(format!("zkstack generate-chain-upgrade failed: {e}")))?;

    if exit_code != 0 {
        return Err(UpgradeError::Config(format!(
            "zkstack generate-chain-upgrade failed with exit code {}",
            exit_code
        )));
    }

    // Step 2: Extract calldatas from zkstack output
    let output_path = log_dir.join("chain-upgrade.txt");
    let output_content = std::fs::read_to_string(&output_path).map_err(|e| {
        UpgradeError::Config(format!(
            "Failed to read chain-upgrade.txt at {}: {e}",
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
    let contracts = resolve_chain_contracts(provider, bridgehub, chain_id).await?;
    log::info!(
        "Chain contracts resolved: diamond={}, admin={}, governor={}",
        contracts.diamond,
        contracts.chain_admin,
        contracts.chain_governor,
    );

    // Step 4: Send schedule + execute transactions to ChainAdmin
    send_chain_tx(
        provider,
        governor_key,
        contracts.chain_admin,
        calldatas.schedule,
        "schedule upgrade",
    )
    .await?;

    send_chain_tx(
        provider,
        governor_key,
        contracts.chain_admin,
        calldatas.chain_admin,
        "execute upgrade",
    )
    .await?;

    // Step 5: Set upgrade timestamp
    set_upgrade_timestamp(
        provider,
        governor_key,
        contracts.chain_admin,
        protocol_version,
    )
    .await?;

    // Step 6: Verify protocol versions
    let versions_match =
        verify_protocol_versions(provider, bridgehub, contracts.diamond, chain_id).await?;

    if versions_match {
        log::info!("Protocol versions match after upgrade");
    } else {
        log::warn!("Protocol versions do NOT match after upgrade");
    }

    Ok(ChainUpgradeResult {
        chain_name: chain_name.to_string(),
        chain_id,
        versions_match,
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_version_to_protocol_uint_v0_30_0() {
        let version = semver::Version::new(0, 30, 0);
        let result = version_to_protocol_uint(&version);
        // (0 << 40) | (30 << 32) | 0 = 30 * 2^32 = 0x1e00000000
        assert_eq!(result, U256::from(0x1e00000000u64));
    }

    #[test]
    fn test_version_to_protocol_uint_v0_30_1() {
        let version = semver::Version::new(0, 30, 1);
        let result = version_to_protocol_uint(&version);
        assert_eq!(result, U256::from(0x1e00000001u64));
    }

    #[test]
    fn test_version_to_protocol_uint_v1_0_0() {
        let version = semver::Version::new(1, 0, 0);
        let result = version_to_protocol_uint(&version);
        // (1 << 40) = 0x10000000000
        assert_eq!(result, U256::from(0x10000000000u64));
    }
}
