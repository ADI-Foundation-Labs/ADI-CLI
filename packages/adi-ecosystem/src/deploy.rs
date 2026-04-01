//! Ecosystem contract deployment and validator role configuration.
//!
//! This module provides functionality for:
//! - Extracting deployed contract addresses from state
//! - Adding and removing validator roles for operator wallets

use crate::error::{EcosystemError, Result};
use crate::signer::create_signer;
use crate::validator::{
    build_add_validator_roles_calldata, build_remove_validator_roles_calldata, ValidatorRoles,
};
use adi_types::{normalize_rpc_url, ChainContracts, Logger, Operators};
use alloy_network::{EthereumWallet, TransactionBuilder};
use alloy_primitives::{Address, B256};
use alloy_provider::{Provider, ProviderBuilder};
use alloy_rpc_types::TransactionRequest;
use console::Style;
use secrecy::SecretString;

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

/// Mode for validator role operations.
enum ValidatorRoleMode {
    /// Adding roles to operators.
    Add,
    /// Removing roles from operators.
    Remove,
}

/// Add validator roles to all operators in the chain.
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
/// * `operators` - Operator addresses to assign roles to.
/// * `governor_key` - Chain governor private key for signing transactions.
/// * `gas_multiplier` - Gas price multiplier percentage (e.g., 120 = 20% buffer). None to use raw estimate.
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
    operators: &Operators,
    governor_key: &SecretString,
    gas_multiplier: Option<u64>,
    logger: &dyn Logger,
) -> Result<Vec<B256>> {
    logger.debug(&format!(
        "Adding validator roles via chain_admin: {}",
        contracts.chain_admin
    ));

    let mut assignments: Vec<ValidatorRoleAssignment> = Vec::new();

    if let Some(addr) = operators.operator {
        assignments.push(ValidatorRoleAssignment {
            name: "commit_operator",
            operator: addr,
            roles: ValidatorRoles::commit_operator(),
        });
    }
    if let Some(addr) = operators.prove_operator {
        assignments.push(ValidatorRoleAssignment {
            name: "prove_operator",
            operator: addr,
            roles: ValidatorRoles::prove_operator(),
        });
    }
    if let Some(addr) = operators.execute_operator {
        assignments.push(ValidatorRoleAssignment {
            name: "execute_operator",
            operator: addr,
            roles: ValidatorRoles::execute_operator(),
        });
    }

    if assignments.is_empty() {
        logger.warning("No operators configured - skipping validator role assignment");
        return Ok(Vec::new());
    }

    execute_validator_role_txs(ValidatorRoleTxParams {
        rpc_url,
        contracts,
        assignments,
        governor_key,
        gas_multiplier,
        mode: ValidatorRoleMode::Add,
        logger,
    })
    .await
}

/// Remove validator roles from all operators in the chain.
///
/// This function sends transactions to revoke roles from operators:
/// - Commit operator: precommitter, committer, reverter roles
/// - Prove operator: prover role
/// - Execute operator: executor role
///
/// # Arguments
///
/// * `rpc_url` - L1 RPC endpoint URL.
/// * `contracts` - Deployed contract addresses.
/// * `operators` - Operator addresses to revoke roles from.
/// * `governor_key` - Chain governor private key for signing transactions.
/// * `gas_multiplier` - Gas price multiplier percentage (e.g., 120 = 20% buffer). None to use raw estimate.
/// * `logger` - Logger for debug/info/warning output.
///
/// # Returns
///
/// Vector of transaction hashes for each successful role revocation.
///
/// # Errors
///
/// Returns error if any transaction fails.
pub async fn remove_validator_roles(
    rpc_url: &str,
    contracts: &DeployedContracts,
    operators: &Operators,
    governor_key: &SecretString,
    gas_multiplier: Option<u64>,
    logger: &dyn Logger,
) -> Result<Vec<B256>> {
    logger.debug(&format!(
        "Removing validator roles via chain_admin: {}",
        contracts.chain_admin
    ));

    let mut revocations: Vec<ValidatorRoleAssignment> = Vec::new();

    if let Some(addr) = operators.operator {
        revocations.push(ValidatorRoleAssignment {
            name: "commit_operator",
            operator: addr,
            roles: ValidatorRoles::commit_operator(),
        });
    }
    if let Some(addr) = operators.prove_operator {
        revocations.push(ValidatorRoleAssignment {
            name: "prove_operator",
            operator: addr,
            roles: ValidatorRoles::prove_operator(),
        });
    }
    if let Some(addr) = operators.execute_operator {
        revocations.push(ValidatorRoleAssignment {
            name: "execute_operator",
            operator: addr,
            roles: ValidatorRoles::execute_operator(),
        });
    }
    if let Some(addr) = operators.blob_operator {
        revocations.push(ValidatorRoleAssignment {
            name: "blob_operator",
            operator: addr,
            roles: ValidatorRoles::commit_operator(),
        });
    }

    if revocations.is_empty() {
        logger.debug("No operators to revoke - skipping");
        return Ok(Vec::new());
    }

    execute_validator_role_txs(ValidatorRoleTxParams {
        rpc_url,
        contracts,
        assignments: revocations,
        governor_key,
        gas_multiplier,
        mode: ValidatorRoleMode::Remove,
        logger,
    })
    .await
}

/// Parameters for executing validator role transactions.
struct ValidatorRoleTxParams<'a> {
    /// L1 RPC endpoint URL.
    rpc_url: &'a str,
    /// Deployed contract addresses.
    contracts: &'a DeployedContracts,
    /// Role assignments to execute.
    assignments: Vec<ValidatorRoleAssignment>,
    /// Chain governor private key for signing transactions.
    governor_key: &'a SecretString,
    /// Gas price multiplier percentage. None to use raw estimate.
    gas_multiplier: Option<u64>,
    /// Whether adding or removing roles.
    mode: ValidatorRoleMode,
    /// Logger for debug/info/warning output.
    logger: &'a dyn Logger,
}

/// Execute validator role transactions for a list of assignments.
async fn execute_validator_role_txs(params: ValidatorRoleTxParams<'_>) -> Result<Vec<B256>> {
    let signer = create_signer(params.governor_key)?;
    let governor_address = signer.address();
    params
        .logger
        .debug(&format!("Governor address: {}", governor_address));

    let wallet = EthereumWallet::from(signer);
    let normalized_rpc = normalize_rpc_url(params.rpc_url);
    let url: url::Url = normalized_rpc.parse().map_err(|e| {
        EcosystemError::InvalidConfig(format!("Invalid RPC URL '{}': {}", params.rpc_url, e))
    })?;
    let provider = ProviderBuilder::new().wallet(wallet).connect_http(url);

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

    let estimated =
        provider
            .get_gas_price()
            .await
            .map_err(|e| EcosystemError::TransactionFailed {
                reason: format!("Failed to get gas price: {}", e),
            })?;
    let gas_price = params
        .gas_multiplier
        .map_or(estimated, |m| estimated * u128::from(m) / 100);
    params
        .logger
        .debug(&format!("Using gas price: {} wei", gas_price));

    let (verb, past_verb, style) = match params.mode {
        ValidatorRoleMode::Add => ("Assigning", "Confirmed", Style::new().green()),
        ValidatorRoleMode::Remove => ("Revoking", "Revoked", Style::new().yellow()),
    };

    let mut tx_hashes = Vec::with_capacity(params.assignments.len());

    for assignment in params.assignments {
        params.logger.debug(&format!(
            "{} roles {} {} ({}): [{}]",
            verb,
            if matches!(params.mode, ValidatorRoleMode::Add) {
                "to"
            } else {
                "from"
            },
            assignment.name,
            assignment.operator,
            assignment.roles
        ));

        let spinner = cliclack::spinner();
        spinner.start(format!(
            "{} {} ({})",
            verb,
            assignment.name,
            style.apply_to(assignment.operator)
        ));

        let calldata = match params.mode {
            ValidatorRoleMode::Add => build_add_validator_roles_calldata(
                params.contracts.validator_timelock,
                params.contracts.diamond_proxy,
                assignment.operator,
                assignment.roles,
            ),
            ValidatorRoleMode::Remove => build_remove_validator_roles_calldata(
                params.contracts.validator_timelock,
                params.contracts.diamond_proxy,
                assignment.operator,
                assignment.roles,
            ),
        };

        let tx = TransactionRequest::default()
            .with_from(governor_address)
            .with_to(params.contracts.chain_admin)
            .with_input(calldata)
            .with_nonce(nonce)
            .with_gas_limit(500_000)
            .with_gas_price(gas_price)
            .with_chain_id(chain_id);

        let pending = provider.send_transaction(tx).await.map_err(|e| {
            spinner.error(format!("Failed to send tx: {}", e));
            EcosystemError::TransactionFailed {
                reason: format!(
                    "Failed to send {} validator role tx: {}",
                    assignment.name, e
                ),
            }
        })?;

        let tx_hash = *pending.tx_hash();
        params
            .logger
            .debug(&format!("Transaction sent: {}", tx_hash));

        let receipt = pending.get_receipt().await.map_err(|e| {
            spinner.error(format!("Confirmation failed: {}", e));
            EcosystemError::TransactionFailed {
                reason: format!(
                    "Failed to confirm {} validator role tx: {}",
                    assignment.name, e
                ),
            }
        })?;

        if !receipt.status() {
            spinner.error("Transaction reverted");
            return Err(EcosystemError::TransactionFailed {
                reason: format!("Transaction {} reverted for {}", tx_hash, assignment.name),
            });
        }

        spinner.stop(format!(
            "{} {} ({}) → Confirmed in block {} (gas: {})",
            past_verb,
            assignment.name,
            style.apply_to(assignment.operator),
            style.apply_to(receipt.block_number.unwrap_or_default()),
            receipt.gas_used
        ));

        params.logger.debug(&format!(
            "Confirmed {} in block {}: tx_hash={}",
            assignment.name,
            receipt.block_number.unwrap_or_default(),
            tx_hash
        ));

        tx_hashes.push(tx_hash);
        nonce += 1;
    }

    Ok(tx_hashes)
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
