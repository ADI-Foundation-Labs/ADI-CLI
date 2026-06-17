//! Per-contract ownership acceptance handlers.
//!
//! Standard `Ownable2Step` contracts share a single generic handler
//! ([`accept_ownable2step`]). Contracts with a non-standard acceptance flow
//! keep dedicated handlers:
//! - Server Notifier (via multicall through ChainAdmin)
//! - Rollup DA Manager (via Governance timelock schedule + execute)

use super::calldata::{
    build_accept_ownership_calldata, build_accept_ownership_multicall_calldata,
    build_governance_execute_calldata, build_governance_schedule_calldata,
};
use super::context::SigningContext;
use super::status::check_ownership_state;
use super::transaction::send_ownership_tx;
use super::types::{OwnershipResult, OwnershipState};
use adi_types::{EcosystemContracts, Logger};
use alloy_primitives::{Address, B256, U256};
use alloy_provider::Provider;
use console::Style;

/// Accept ownership for a standard `Ownable2Step` contract via a direct
/// `acceptOwnership()` call signed by the governor.
///
/// `name` is used for logging and result labels, `address` is the contract to
/// accept (a `None` yields a skipped result). The contract is skipped when the
/// governor is already the owner or no transfer is pending to it.
pub(crate) async fn accept_ownable2step<P>(
    ctx: &mut SigningContext<P>,
    address: Option<Address>,
    name: &str,
    logger: &dyn Logger,
) -> OwnershipResult
where
    P: Provider + Clone,
{
    let green = Style::new().green();

    let Some(addr) = address else {
        return OwnershipResult::skipped(name, "address not configured");
    };

    // Check if ownership acceptance is needed
    match check_ownership_state(&ctx.provider, addr, ctx.governor_address, name, logger).await {
        OwnershipState::Accepted => {
            logger.info(&format!("{}: ownership already accepted", name));
            return OwnershipResult::skipped(name, "ownership already accepted");
        }
        OwnershipState::NotTransferred => {
            logger.warning(&format!("{}: ownership not transferred", name));
            return OwnershipResult::skipped(name, "ownership not transferred");
        }
        OwnershipState::Pending => {}
    }

    let spinner = cliclack::spinner();
    spinner.start(name.to_string());

    let calldata = build_accept_ownership_calldata();

    match send_ownership_tx(
        &ctx.provider,
        addr,
        calldata,
        ctx.governor_address,
        ctx.chain_id,
        ctx.nonce,
        ctx.gas_price,
    )
    .await
    {
        Ok(result) => {
            spinner.stop(format!(
                "{} → Accepted (block {})",
                name,
                green.apply_to(result.block_number)
            ));
            ctx.nonce += 1;
            OwnershipResult::success(name, result.tx_hash)
        }
        Err(e) => {
            spinner.error(format!("{} failed: {}", name, e));
            OwnershipResult::failure(name, e.to_string())
        }
    }
}

/// Accept ownership for Server Notifier (owned by ChainAdmin, via multicall).
pub(crate) async fn accept_server_notifier<P>(
    ctx: &mut SigningContext<P>,
    contracts: &EcosystemContracts,
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
        &ctx.provider,
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
        &ctx.provider,
        chain_admin_addr,
        calldata,
        ctx.governor_address,
        ctx.chain_id,
        ctx.nonce,
        ctx.gas_price,
    )
    .await
    {
        Ok(result) => {
            spinner.stop(format!(
                "Server Notifier → Accepted (block {})",
                green.apply_to(result.block_number)
            ));
            ctx.nonce += 1;
            OwnershipResult::success("Server Notifier", result.tx_hash)
        }
        Err(e) => {
            spinner.error(format!("Server Notifier failed: {}", e));
            OwnershipResult::failure("Server Notifier", e.to_string())
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
    ctx: &mut SigningContext<P>,
    contracts: &EcosystemContracts,
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
        &ctx.provider,
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
    let salt = B256::from(U256::from(ctx.nonce));

    let spinner = cliclack::spinner();
    spinner.start("Rollup DA Manager");

    // Step 1: Schedule the operation via Governance
    let schedule_calldata = build_governance_schedule_calldata(da_manager, salt);

    let schedule_block = match send_ownership_tx(
        &ctx.provider,
        governance,
        schedule_calldata,
        ctx.governor_address,
        ctx.chain_id,
        ctx.nonce,
        ctx.gas_price,
    )
    .await
    {
        Ok(result) => {
            ctx.nonce += 1;
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
        &ctx.provider,
        governance,
        execute_calldata,
        ctx.governor_address,
        ctx.chain_id,
        ctx.nonce,
        ctx.gas_price,
    )
    .await
    {
        Ok(result) => {
            spinner.stop(format!(
                "Rollup DA Manager → Accepted (blocks {}, {})",
                green.apply_to(schedule_block),
                green.apply_to(result.block_number)
            ));
            ctx.nonce += 1;
            OwnershipResult::success("Rollup DA Manager", result.tx_hash)
        }
        Err(e) => {
            spinner.error(format!("Rollup DA Manager execute failed: {}", e));
            OwnershipResult::failure("Rollup DA Manager", format!("Execute failed: {}", e))
        }
    }
}
