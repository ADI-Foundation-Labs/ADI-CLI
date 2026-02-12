//! Ecosystem contract deployment and validator role configuration.
//!
//! This module provides functionality for:
//! - Extracting deployed contract addresses from state
//! - Adding validator roles to operator wallets

use crate::error::{EcosystemError, Result};
use crate::validator::{build_add_validator_roles_calldata, ValidatorRoles};
use adi_types::{normalize_rpc_url, ChainContracts, Logger, Wallets};
use alloy_network::{EthereumWallet, TransactionBuilder};
use alloy_primitives::{Address, B256};
use alloy_provider::{Provider, ProviderBuilder};
use alloy_rpc_types::TransactionRequest;
use alloy_signer_local::PrivateKeySigner;
use console::Style;
use secrecy::{ExposeSecret, SecretString};

/// Contract addresses required for validator role configuration.
#[derive(Debug, Clone)]
pub struct DeployedContracts {
    /// ValidatorTimelock contract address.
    pub validator_timelock: Address,
    /// ChainAdmin contract address.
    pub chain_admin: Address,
    /// Diamond proxy contract address.
    pub diamond_proxy: Address,
}

impl DeployedContracts {
    /// Extract deployed contract addresses from chain contracts.
    ///
    /// # Errors
    ///
    /// Returns error if any required contract address is missing.
    pub fn try_from_chain_contracts(contracts: &ChainContracts) -> Result<Self> {
        let l1 = contracts
            .l1
            .as_ref()
            .ok_or_else(|| EcosystemError::MissingContract("l1".to_string()))?;

        let validator_timelock = l1.validator_timelock_addr.ok_or_else(|| {
            EcosystemError::MissingContract("validator_timelock_addr".to_string())
        })?;

        let chain_admin = l1
            .chain_admin_addr
            .ok_or_else(|| EcosystemError::MissingContract("chain_admin_addr".to_string()))?;

        let diamond_proxy = l1
            .diamond_proxy_addr
            .ok_or_else(|| EcosystemError::MissingContract("diamond_proxy_addr".to_string()))?;

        Ok(Self {
            validator_timelock,
            chain_admin,
            diamond_proxy,
        })
    }
}

/// Operator role assignment to be executed.
struct ValidatorRoleAssignment {
    /// Name of the operator for logging.
    name: &'static str,
    /// Operator wallet address.
    operator: Address,
    /// Roles to assign.
    roles: ValidatorRoles,
}

/// Add validator roles to all operators in the chain wallets.
///
/// This function sends transactions to assign:
/// - Commit operator: precommitter, committer, reverter roles
/// - Prove operator: prover role
/// - Execute operator: executor role
///
/// # Arguments
///
/// * `rpc_url` - L1 RPC endpoint URL.
/// * `contracts` - Deployed contract addresses.
/// * `chain_wallets` - Chain wallets containing operator addresses.
/// * `governor_key` - Chain governor private key for signing transactions.
/// * `gas_price_wei` - Optional gas price in wei (estimated if not provided).
/// * `logger` - Logger for debug/info/warning output.
///
/// # Returns
///
/// Vector of transaction hashes for each successful role assignment.
///
/// # Errors
///
/// Returns error if any transaction fails.
pub async fn add_validator_roles(
    rpc_url: &str,
    contracts: &DeployedContracts,
    chain_wallets: &Wallets,
    governor_key: &SecretString,
    gas_price_wei: Option<u128>,
    logger: &dyn Logger,
) -> Result<Vec<B256>> {
    logger.debug(&format!(
        "Adding validator roles via chain_admin: {}",
        contracts.chain_admin
    ));

    // Create signer from governor key
    let signer = create_signer(governor_key)?;
    let governor_address = signer.address();
    logger.debug(&format!("Governor address: {}", governor_address));

    // Create signing provider
    let wallet = EthereumWallet::from(signer);
    let normalized_rpc = normalize_rpc_url(rpc_url);
    let url: url::Url = normalized_rpc.parse().map_err(|e| {
        EcosystemError::InvalidConfig(format!("Invalid RPC URL '{}': {}", rpc_url, e))
    })?;
    let provider = ProviderBuilder::new().wallet(wallet).connect_http(url);

    // Get chain ID and nonce
    let chain_id =
        provider
            .get_chain_id()
            .await
            .map_err(|e| EcosystemError::TransactionFailed {
                reason: format!("Failed to get chain ID: {}", e),
            })?;

    let mut nonce = provider
        .get_transaction_count(governor_address)
        .await
        .map_err(|e| EcosystemError::TransactionFailed {
            reason: format!("Failed to get nonce: {}", e),
        })?;

    // Get gas price if not provided
    let gas_price = match gas_price_wei {
        Some(price) => price,
        None => provider
            .get_gas_price()
            .await
            .map_err(|e| EcosystemError::TransactionFailed {
                reason: format!("Failed to get gas price: {}", e),
            })?,
    };
    logger.debug(&format!("Using gas price: {} wei", gas_price));

    // Build list of role assignments
    let mut assignments: Vec<ValidatorRoleAssignment> = Vec::new();

    if let Some(wallet) = &chain_wallets.operator {
        assignments.push(ValidatorRoleAssignment {
            name: "commit_operator",
            operator: wallet.address,
            roles: ValidatorRoles::commit_operator(),
        });
    }

    if let Some(wallet) = &chain_wallets.prove_operator {
        assignments.push(ValidatorRoleAssignment {
            name: "prove_operator",
            operator: wallet.address,
            roles: ValidatorRoles::prove_operator(),
        });
    }

    if let Some(wallet) = &chain_wallets.execute_operator {
        assignments.push(ValidatorRoleAssignment {
            name: "execute_operator",
            operator: wallet.address,
            roles: ValidatorRoles::execute_operator(),
        });
    }

    if assignments.is_empty() {
        logger.warning("No operator wallets found - skipping validator role assignment");
        return Ok(Vec::new());
    }

    // Execute role assignments
    let mut tx_hashes = Vec::with_capacity(assignments.len());
    let green = Style::new().green();

    for assignment in assignments {
        logger.info(&format!(
            "Adding validator roles for {} ({})",
            assignment.name,
            green.apply_to(assignment.operator)
        ));

        let calldata = build_add_validator_roles_calldata(
            contracts.validator_timelock,
            contracts.diamond_proxy,
            assignment.operator,
            assignment.roles,
        );

        // Build transaction to chain_admin
        let tx = TransactionRequest::default()
            .with_from(governor_address)
            .with_to(contracts.chain_admin)
            .with_input(calldata)
            .with_nonce(nonce)
            .with_gas_limit(500_000) // Conservative gas limit for multicall
            .with_gas_price(gas_price)
            .with_chain_id(chain_id);

        // Send transaction
        let pending =
            provider
                .send_transaction(tx)
                .await
                .map_err(|e| EcosystemError::TransactionFailed {
                    reason: format!(
                        "Failed to send {} validator role tx: {}",
                        assignment.name, e
                    ),
                })?;

        let tx_hash = *pending.tx_hash();
        logger.info(&format!(
            "  Transaction submitted: {}",
            green.apply_to(tx_hash)
        ));

        // Wait for confirmation
        let receipt =
            pending
                .get_receipt()
                .await
                .map_err(|e| EcosystemError::TransactionFailed {
                    reason: format!(
                        "Failed to confirm {} validator role tx: {}",
                        assignment.name, e
                    ),
                })?;

        if !receipt.status() {
            return Err(EcosystemError::TransactionFailed {
                reason: format!("Transaction {} reverted for {}", tx_hash, assignment.name),
            });
        }

        logger.info(&format!(
            "  Confirmed in block {} (gas used: {})",
            green.apply_to(receipt.block_number.unwrap_or_default()),
            green.apply_to(receipt.gas_used)
        ));

        tx_hashes.push(tx_hash);
        nonce += 1;
    }

    Ok(tx_hashes)
}

/// Create a signer from a private key.
fn create_signer(key: &SecretString) -> Result<PrivateKeySigner> {
    let key_str = key.expose_secret();

    // Strip 0x prefix if present
    let key_hex = key_str.strip_prefix("0x").unwrap_or(key_str);

    let key_bytes: [u8; 32] = hex::decode(key_hex)
        .map_err(|e| EcosystemError::InvalidConfig(format!("Invalid private key hex: {}", e)))?
        .try_into()
        .map_err(|_| EcosystemError::InvalidConfig("Private key must be 32 bytes".to_string()))?;

    PrivateKeySigner::from_bytes(&key_bytes.into())
        .map_err(|e| EcosystemError::InvalidConfig(format!("Invalid private key: {}", e)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deployed_contracts_missing_l1() {
        let contracts = ChainContracts::default();
        let result = DeployedContracts::try_from_chain_contracts(&contracts);
        assert!(result.is_err());
    }
}
