//! Per-contract ownership acceptance handlers.
//!
//! This module contains the acceptance logic for each contract type:
//! - Chain Admin (direct)
//! - Server Notifier (via multicall)
//! - Validator Timelock (direct)
//! - Verifier (direct)
//! - Governance (direct)
//! - Rollup DA Manager (via governance timelock)

use super::calldata::{
    build_accept_ownership_calldata, build_accept_ownership_multicall_calldata,
    build_governance_execute_calldata, build_governance_schedule_calldata,
};
use super::status::check_ownership_state;
use super::transaction::send_ownership_tx;
use super::types::{OwnershipResult, OwnershipState};
use adi_types::{ChainContracts, EcosystemContracts, Logger};
use alloy_primitives::{Address, B256, U256};
use alloy_provider::Provider;
use console::Style;

/// Accept ownership for Chain Admin contract (chain-level).
pub(crate) async fn accept_chain_admin<P>(
    provider: &P,
    contracts: &ChainContracts,
    governor: Address,
    chain_id: u64,
    nonce: &mut u64,
    gas_price: u128,
    logger: &dyn Logger,
) -> OwnershipResult
where
    P: Provider + Clone,
{
    let green = Style::new().green();

    let Some(chain_admin) = contracts.chain_admin_addr() else {
        return OwnershipResult::skipped("Chain Admin", "chain_admin_addr not configured");
    };

    // Check if ownership acceptance is needed
    match check_ownership_state(provider, chain_admin, governor, "Chain Admin", logger).await {
        OwnershipState::Accepted => {
            logger.info("Chain Admin: ownership already accepted");
            return OwnershipResult::skipped("Chain Admin", "ownership already accepted");
        }
        OwnershipState::NotTransferred => {
            logger.warning("Chain Admin: ownership not transferred");
            return OwnershipResult::skipped("Chain Admin", "ownership not transferred");
        }
        OwnershipState::Pending => {}
    }

    let spinner = cliclack::spinner();
    spinner.start("Chain Admin");

    let calldata = build_accept_ownership_calldata();

    match send_ownership_tx(
        provider,
        chain_admin,
        calldata,
        governor,
        chain_id,
        *nonce,
        gas_price,
    )
    .await
    {
        Ok(result) => {
            spinner.stop(format!(
                "Chain Admin → Accepted (block {})",
                green.apply_to(result.block_number)
            ));
            *nonce += 1;
            OwnershipResult::success("Chain Admin", result.tx_hash)
        }
        Err(e) => {
            spinner.error(format!("Chain Admin failed: {}", e));
            OwnershipResult::failure("Chain Admin", e.to_string())
        }
    }
}

/// Accept ownership for Chain Governance contract (chain-level).
pub(crate) async fn accept_chain_governance<P>(
    provider: &P,
    contracts: &ChainContracts,
    governor: Address,
    chain_id: u64,
    nonce: &mut u64,
    gas_price: u128,
    logger: &dyn Logger,
) -> OwnershipResult
where
    P: Provider + Clone,
{
    let green = Style::new().green();

    let Some(governance) = contracts.governance_addr() else {
        return OwnershipResult::skipped("Chain Governance", "governance_addr not configured");
    };

    // Check if ownership acceptance is needed
    match check_ownership_state(provider, governance, governor, "Chain Governance", logger).await {
        OwnershipState::Accepted => {
            logger.info("Chain Governance: ownership already accepted");
            return OwnershipResult::skipped("Chain Governance", "ownership already accepted");
        }
        OwnershipState::NotTransferred => {
            logger.warning("Chain Governance: ownership not transferred");
            return OwnershipResult::skipped("Chain Governance", "ownership not transferred");
        }
        OwnershipState::Pending => {}
    }

    let spinner = cliclack::spinner();
    spinner.start("Chain Governance");

    let calldata = build_accept_ownership_calldata();

    match send_ownership_tx(
        provider, governance, calldata, governor, chain_id, *nonce, gas_price,
    )
    .await
    {
        Ok(result) => {
            spinner.stop(format!(
                "Chain Governance → Accepted (block {})",
                green.apply_to(result.block_number)
            ));
            *nonce += 1;
            OwnershipResult::success("Chain Governance", result.tx_hash)
        }
        Err(e) => {
            spinner.error(format!("Chain Governance failed: {}", e));
            OwnershipResult::failure("Chain Governance", e.to_string())
        }
    }
}

/// Accept ownership for Ecosystem Chain Admin contract.
pub(crate) async fn accept_ecosystem_chain_admin<P>(
    provider: &P,
    contracts: &EcosystemContracts,
    governor: Address,
    chain_id: u64,
    nonce: &mut u64,
    gas_price: u128,
    logger: &dyn Logger,
) -> OwnershipResult
where
    P: Provider + Clone,
{
    let green = Style::new().green();

    let Some(chain_admin) = contracts.chain_admin_addr() else {
        return OwnershipResult::skipped(
            "Ecosystem Chain Admin",
            "chain_admin_addr not configured",
        );
    };

    // Check if ownership acceptance is needed
    match check_ownership_state(
        provider,
        chain_admin,
        governor,
        "Ecosystem Chain Admin",
        logger,
    )
    .await
    {
        OwnershipState::Accepted => {
            logger.info("Ecosystem Chain Admin: ownership already accepted");
            return OwnershipResult::skipped("Ecosystem Chain Admin", "ownership already accepted");
        }
        OwnershipState::NotTransferred => {
            logger.warning("Ecosystem Chain Admin: ownership not transferred");
            return OwnershipResult::skipped("Ecosystem Chain Admin", "ownership not transferred");
        }
        OwnershipState::Pending => {}
    }

    let spinner = cliclack::spinner();
    spinner.start("Ecosystem Chain Admin");

    let calldata = build_accept_ownership_calldata();

    match send_ownership_tx(
        provider,
        chain_admin,
        calldata,
        governor,
        chain_id,
        *nonce,
        gas_price,
    )
    .await
    {
        Ok(result) => {
            spinner.stop(format!(
                "Ecosystem Chain Admin → Accepted (block {})",
                green.apply_to(result.block_number)
            ));
            *nonce += 1;
            OwnershipResult::success("Ecosystem Chain Admin", result.tx_hash)
        }
        Err(e) => {
            spinner.error(format!("Ecosystem Chain Admin failed: {}", e));
            OwnershipResult::failure("Ecosystem Chain Admin", e.to_string())
        }
    }
}

/// Accept ownership for Server Notifier via multicall.
pub(crate) async fn accept_server_notifier<P>(
    provider: &P,
    contracts: &EcosystemContracts,
    governor: Address,
    chain_id: u64,
    nonce: &mut u64,
    gas_price: u128,
    logger: &dyn Logger,
) -> OwnershipResult
where
    P: Provider + Clone,
{
    let green = Style::new().green();

    let Some(server_notifier) = contracts.server_notifier_addr() else {
        return OwnershipResult::skipped(
            "Server Notifier",
            "server_notifier_proxy_addr not configured",
        );
    };

    let Some(chain_admin_addr) = contracts.chain_admin_addr() else {
        return OwnershipResult::skipped("Server Notifier", "chain_admin_addr not configured");
    };

    // Check if ownership acceptance is needed
    // Note: Server Notifier is owned by ChainAdmin, not governor
    match check_ownership_state(
        provider,
        server_notifier,
        chain_admin_addr,
        "Server Notifier",
        logger,
    )
    .await
    {
        OwnershipState::Accepted => {
            logger.info("Server Notifier: ownership already accepted");
            return OwnershipResult::skipped("Server Notifier", "ownership already accepted");
        }
        OwnershipState::NotTransferred => {
            logger.warning("Server Notifier: ownership not transferred");
            return OwnershipResult::skipped("Server Notifier", "ownership not transferred");
        }
        OwnershipState::Pending => {}
    }

    let spinner = cliclack::spinner();
    spinner.start("Server Notifier");

    let calldata = build_accept_ownership_multicall_calldata(server_notifier);

    match send_ownership_tx(
        provider,
        chain_admin_addr,
        calldata,
        governor,
        chain_id,
        *nonce,
        gas_price,
    )
    .await
    {
        Ok(result) => {
            spinner.stop(format!(
                "Server Notifier → Accepted (block {})",
                green.apply_to(result.block_number)
            ));
            *nonce += 1;
            OwnershipResult::success("Server Notifier", result.tx_hash)
        }
        Err(e) => {
            spinner.error(format!("Server Notifier failed: {}", e));
            OwnershipResult::failure("Server Notifier", e.to_string())
        }
    }
}

/// Accept ownership for Validator Timelock.
pub(crate) async fn accept_validator_timelock<P>(
    provider: &P,
    contracts: &EcosystemContracts,
    governor: Address,
    chain_id: u64,
    nonce: &mut u64,
    gas_price: u128,
    logger: &dyn Logger,
) -> OwnershipResult
where
    P: Provider + Clone,
{
    let green = Style::new().green();

    let Some(timelock) = contracts.validator_timelock_addr() else {
        return OwnershipResult::skipped(
            "Validator Timelock",
            "validator_timelock_addr not configured",
        );
    };

    // Check if ownership acceptance is needed
    match check_ownership_state(provider, timelock, governor, "Validator Timelock", logger).await {
        OwnershipState::Accepted => {
            logger.info("Validator Timelock: ownership already accepted");
            return OwnershipResult::skipped("Validator Timelock", "ownership already accepted");
        }
        OwnershipState::NotTransferred => {
            logger.warning("Validator Timelock: ownership not transferred");
            return OwnershipResult::skipped("Validator Timelock", "ownership not transferred");
        }
        OwnershipState::Pending => {}
    }

    let spinner = cliclack::spinner();
    spinner.start("Validator Timelock");

    let calldata = build_accept_ownership_calldata();

    match send_ownership_tx(
        provider, timelock, calldata, governor, chain_id, *nonce, gas_price,
    )
    .await
    {
        Ok(result) => {
            spinner.stop(format!(
                "Validator Timelock → Accepted (block {})",
                green.apply_to(result.block_number)
            ));
            *nonce += 1;
            OwnershipResult::success("Validator Timelock", result.tx_hash)
        }
        Err(e) => {
            spinner.error(format!("Validator Timelock failed: {}", e));
            OwnershipResult::failure("Validator Timelock", e.to_string())
        }
    }
}

/// Accept ownership for Verifier.
pub(crate) async fn accept_verifier<P>(
    provider: &P,
    contracts: &EcosystemContracts,
    governor: Address,
    chain_id: u64,
    nonce: &mut u64,
    gas_price: u128,
    logger: &dyn Logger,
) -> OwnershipResult
where
    P: Provider + Clone,
{
    let green = Style::new().green();

    let Some(verifier) = contracts.verifier_addr() else {
        return OwnershipResult::skipped("Verifier", "verifier_addr not configured");
    };

    // Check if ownership acceptance is needed
    match check_ownership_state(provider, verifier, governor, "Verifier", logger).await {
        OwnershipState::Accepted => {
            logger.info("Verifier: ownership already accepted");
            return OwnershipResult::skipped("Verifier", "ownership already accepted");
        }
        OwnershipState::NotTransferred => {
            logger.warning("Verifier: ownership not transferred");
            return OwnershipResult::skipped("Verifier", "ownership not transferred");
        }
        OwnershipState::Pending => {}
    }

    let spinner = cliclack::spinner();
    spinner.start("Verifier");

    let calldata = build_accept_ownership_calldata();

    match send_ownership_tx(
        provider, verifier, calldata, governor, chain_id, *nonce, gas_price,
    )
    .await
    {
        Ok(result) => {
            spinner.stop(format!(
                "Verifier → Accepted (block {})",
                green.apply_to(result.block_number)
            ));
            *nonce += 1;
            OwnershipResult::success("Verifier", result.tx_hash)
        }
        Err(e) => {
            spinner.error(format!("Verifier failed: {}", e));
            OwnershipResult::failure("Verifier", e.to_string())
        }
    }
}

/// Accept ownership for Governance contract.
pub(crate) async fn accept_governance<P>(
    provider: &P,
    contracts: &EcosystemContracts,
    governor: Address,
    chain_id: u64,
    nonce: &mut u64,
    gas_price: u128,
    logger: &dyn Logger,
) -> OwnershipResult
where
    P: Provider + Clone,
{
    let green = Style::new().green();

    let Some(governance) = contracts.governance_addr() else {
        return OwnershipResult::skipped("Governance", "governance_addr not configured");
    };

    // Check if ownership acceptance is needed
    match check_ownership_state(provider, governance, governor, "Governance", logger).await {
        OwnershipState::Accepted => {
            logger.info("Governance: ownership already accepted");
            return OwnershipResult::skipped("Governance", "ownership already accepted");
        }
        OwnershipState::NotTransferred => {
            logger.warning("Governance: ownership not transferred");
            return OwnershipResult::skipped("Governance", "ownership not transferred");
        }
        OwnershipState::Pending => {}
    }

    let spinner = cliclack::spinner();
    spinner.start("Governance");

    let calldata = build_accept_ownership_calldata();

    match send_ownership_tx(
        provider, governance, calldata, governor, chain_id, *nonce, gas_price,
    )
    .await
    {
        Ok(result) => {
            spinner.stop(format!(
                "Governance → Accepted (block {})",
                green.apply_to(result.block_number)
            ));
            *nonce += 1;
            OwnershipResult::success("Governance", result.tx_hash)
        }
        Err(e) => {
            spinner.error(format!("Governance failed: {}", e));
            OwnershipResult::failure("Governance", e.to_string())
        }
    }
}

/// Accept ownership for Rollup DA Manager via Governance.
///
/// This uses the Governance timelock pattern:
/// 1. Call scheduleTransparent(operation, 0) to schedule the acceptOwnership call
/// 2. Call execute(operation) to execute the scheduled operation
///
/// The operation contains a Call to the DA Manager's acceptOwnership() function.
pub(crate) async fn accept_rollup_da_manager<P>(
    provider: &P,
    contracts: &EcosystemContracts,
    governor: Address,
    chain_id: u64,
    nonce: &mut u64,
    gas_price: u128,
    logger: &dyn Logger,
) -> OwnershipResult
where
    P: Provider + Clone,
{
    let green = Style::new().green();

    let Some(da_manager) = contracts.l1_rollup_da_manager_addr() else {
        return OwnershipResult::skipped(
            "Rollup DA Manager",
            "l1_rollup_da_manager not configured",
        );
    };

    let Some(governance) = contracts.governance_addr() else {
        return OwnershipResult::skipped(
            "Rollup DA Manager",
            "governance_addr not configured (required for Governance timelock)",
        );
    };

    // Check if ownership acceptance is needed by checking pendingOwner on DA Manager
    // Note: for DA Manager, the expected pending owner is the governance contract
    match check_ownership_state(
        provider,
        da_manager,
        governance,
        "Rollup DA Manager",
        logger,
    )
    .await
    {
        OwnershipState::Accepted => {
            logger.info("Rollup DA Manager: ownership already accepted");
            return OwnershipResult::skipped("Rollup DA Manager", "ownership already accepted");
        }
        OwnershipState::NotTransferred => {
            logger.warning("Rollup DA Manager: ownership not transferred");
            return OwnershipResult::skipped("Rollup DA Manager", "ownership not transferred");
        }
        OwnershipState::Pending => {}
    }

    // Generate a unique salt for this operation (using current nonce as entropy)
    let salt = B256::from(U256::from(*nonce));

    let spinner = cliclack::spinner();
    spinner.start("Rollup DA Manager");

    // Step 1: Schedule the operation via Governance
    let schedule_calldata = build_governance_schedule_calldata(da_manager, salt);

    let schedule_block = match send_ownership_tx(
        provider,
        governance,
        schedule_calldata,
        governor,
        chain_id,
        *nonce,
        gas_price,
    )
    .await
    {
        Ok(result) => {
            *nonce += 1;
            result.block_number
        }
        Err(e) => {
            spinner.error(format!("Rollup DA Manager schedule failed: {}", e));
            return OwnershipResult::failure(
                "Rollup DA Manager",
                format!("Schedule failed: {}", e),
            );
        }
    };

    // Step 2: Execute the scheduled operation
    let execute_calldata = build_governance_execute_calldata(da_manager, salt);

    match send_ownership_tx(
        provider,
        governance,
        execute_calldata,
        governor,
        chain_id,
        *nonce,
        gas_price,
    )
    .await
    {
        Ok(result) => {
            spinner.stop(format!(
                "Rollup DA Manager → Accepted (blocks {}, {})",
                green.apply_to(schedule_block),
                green.apply_to(result.block_number)
            ));
            *nonce += 1;
            OwnershipResult::success("Rollup DA Manager", result.tx_hash)
        }
        Err(e) => {
            spinner.error(format!("Rollup DA Manager execute failed: {}", e));
            OwnershipResult::failure("Rollup DA Manager", format!("Execute failed: {}", e))
        }
    }
}
