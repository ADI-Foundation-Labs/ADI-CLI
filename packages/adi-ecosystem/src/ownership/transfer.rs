//! Transfer ownership handlers for contracts.
//!
//! This module contains the transfer logic for each contract type.
//! These functions are used after accepting ownership to transfer
//! it to a final owner address.

use super::calldata::build_transfer_ownership_calldata;
use super::status::{check_ownership_state, check_ownership_state_for_ownable};
use super::transaction::send_ownership_tx;
use super::types::{bridgedTokenBeaconCall, OwnershipResult, OwnershipState};
use adi_types::{ChainContracts, EcosystemContracts, Logger};
use alloy_primitives::Address;
use alloy_provider::Provider;
use alloy_rpc_types::TransactionRequest;
use alloy_sol_types::SolCall;
use console::Style;

/// Shared context for ownership transfer operations.
pub(crate) struct TransferContext<'a> {
    /// Governor wallet address (current owner).
    pub governor: Address,
    /// Address to transfer ownership to.
    pub new_owner: Address,
    /// L1 chain ID.
    pub chain_id: u64,
    /// Current nonce (mutated after each tx).
    pub nonce: &'a mut u64,
    /// Gas price in wei.
    pub gas_price: u128,
    /// Logger for debug/info/warning output.
    pub logger: &'a dyn Logger,
}

/// Query bridged token beacon address from NativeTokenVault contract.
pub(crate) async fn get_bridged_token_beacon<P>(
    provider: &P,
    native_token_vault: Address,
) -> Option<Address>
where
    P: Provider + Clone,
{
    let calldata = bridgedTokenBeaconCall {}.abi_encode();
    let tx = TransactionRequest::default()
        .to(native_token_vault)
        .input(calldata.into());

    match provider.call(tx).await {
        Ok(result) => result.get(12..32).map(Address::from_slice),
        Err(_e) => {
            // Debug logging happens at call site
            None
        }
    }
}

/// Transfer ownership for Governance contract.
pub(crate) async fn transfer_governance<P>(
    provider: &P,
    contracts: &EcosystemContracts,
    ctx: &mut TransferContext<'_>,
) -> OwnershipResult
where
    P: Provider + Clone,
{
    let green = Style::new().green();

    let Some(governance) = contracts.governance_addr() else {
        return OwnershipResult::skipped("Governance", "governance_addr not configured");
    };

    // Verify governor is current owner before transferring
    match check_ownership_state(provider, governance, ctx.governor, "Governance", ctx.logger).await
    {
        OwnershipState::Accepted => {} // Good - we can transfer
        OwnershipState::Pending => {
            return OwnershipResult::skipped(
                "Governance",
                "ownership not yet accepted, accept first",
            );
        }
        OwnershipState::NotTransferred => {
            return OwnershipResult::skipped("Governance", "governor is not the current owner");
        }
    }

    let spinner = cliclack::spinner();
    spinner.start("Governance");

    let calldata = build_transfer_ownership_calldata(ctx.new_owner);

    match send_ownership_tx(
        provider,
        governance,
        calldata,
        ctx.governor,
        ctx.chain_id,
        *ctx.nonce,
        ctx.gas_price,
    )
    .await
    {
        Ok(result) => {
            spinner.stop(format!(
                "Governance → Transferred (block {})",
                green.apply_to(result.block_number)
            ));
            *ctx.nonce += 1;
            OwnershipResult::success("Governance", result.tx_hash)
        }
        Err(e) => {
            spinner.error(format!("Governance transfer failed: {}", e));
            OwnershipResult::failure("Governance", e.to_string())
        }
    }
}

/// Transfer ownership for ecosystem Chain Admin contract.
pub(crate) async fn transfer_ecosystem_chain_admin<P>(
    provider: &P,
    contracts: &EcosystemContracts,
    ctx: &mut TransferContext<'_>,
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

    // Verify governor is current owner before transferring
    match check_ownership_state(
        provider,
        chain_admin,
        ctx.governor,
        "Ecosystem Chain Admin",
        ctx.logger,
    )
    .await
    {
        OwnershipState::Accepted => {} // Good - we can transfer
        OwnershipState::Pending => {
            return OwnershipResult::skipped(
                "Ecosystem Chain Admin",
                "ownership not yet accepted, accept first",
            );
        }
        OwnershipState::NotTransferred => {
            return OwnershipResult::skipped(
                "Ecosystem Chain Admin",
                "governor is not the current owner",
            );
        }
    }

    let spinner = cliclack::spinner();
    spinner.start("Ecosystem Chain Admin");

    let calldata = build_transfer_ownership_calldata(ctx.new_owner);

    match send_ownership_tx(
        provider,
        chain_admin,
        calldata,
        ctx.governor,
        ctx.chain_id,
        *ctx.nonce,
        ctx.gas_price,
    )
    .await
    {
        Ok(result) => {
            spinner.stop(format!(
                "Ecosystem Chain Admin → Transferred (block {})",
                green.apply_to(result.block_number)
            ));
            *ctx.nonce += 1;
            OwnershipResult::success("Ecosystem Chain Admin", result.tx_hash)
        }
        Err(e) => {
            spinner.error(format!("Ecosystem Chain Admin transfer failed: {}", e));
            OwnershipResult::failure("Ecosystem Chain Admin", e.to_string())
        }
    }
}

/// Transfer ownership for Validator Timelock contract.
pub(crate) async fn transfer_validator_timelock<P>(
    provider: &P,
    contracts: &EcosystemContracts,
    ctx: &mut TransferContext<'_>,
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

    // Verify governor is current owner before transferring
    match check_ownership_state(
        provider,
        timelock,
        ctx.governor,
        "Validator Timelock",
        ctx.logger,
    )
    .await
    {
        OwnershipState::Accepted => {} // Good - we can transfer
        OwnershipState::Pending => {
            return OwnershipResult::skipped(
                "Validator Timelock",
                "ownership not yet accepted, accept first",
            );
        }
        OwnershipState::NotTransferred => {
            return OwnershipResult::skipped(
                "Validator Timelock",
                "governor is not the current owner",
            );
        }
    }

    let spinner = cliclack::spinner();
    spinner.start("Validator Timelock");

    let calldata = build_transfer_ownership_calldata(ctx.new_owner);

    match send_ownership_tx(
        provider,
        timelock,
        calldata,
        ctx.governor,
        ctx.chain_id,
        *ctx.nonce,
        ctx.gas_price,
    )
    .await
    {
        Ok(result) => {
            spinner.stop(format!(
                "Validator Timelock → Transferred (block {})",
                green.apply_to(result.block_number)
            ));
            *ctx.nonce += 1;
            OwnershipResult::success("Validator Timelock", result.tx_hash)
        }
        Err(e) => {
            spinner.error(format!("Validator Timelock transfer failed: {}", e));
            OwnershipResult::failure("Validator Timelock", e.to_string())
        }
    }
}

/// Transfer ownership for Bridged Token Beacon contract.
///
/// Note: This contract uses Ownable (not Ownable2Step), so ownership
/// transfers immediately without requiring an accept step.
pub(crate) async fn transfer_bridged_token_beacon<P>(
    provider: &P,
    contracts: &EcosystemContracts,
    ctx: &mut TransferContext<'_>,
) -> OwnershipResult
where
    P: Provider + Clone,
{
    let green = Style::new().green();

    // Get native token vault address
    let Some(native_token_vault) = contracts.native_token_vault_addr() else {
        return OwnershipResult::skipped(
            "Bridged Token Beacon",
            "native_token_vault_addr not configured",
        );
    };

    // Query bridged token beacon address from native token vault
    let Some(beacon) = get_bridged_token_beacon(provider, native_token_vault).await else {
        return OwnershipResult::skipped(
            "Bridged Token Beacon",
            "failed to query bridgedTokenBeacon from native token vault",
        );
    };

    // Verify governor is current owner before transferring
    // Note: Bridged Token Beacon uses Ownable (not Ownable2Step), so we only check owner()
    match check_ownership_state_for_ownable(
        provider,
        beacon,
        ctx.governor,
        "Bridged Token Beacon",
        ctx.logger,
    )
    .await
    {
        OwnershipState::Accepted => {} // Good - we can transfer
        OwnershipState::NotTransferred => {
            return OwnershipResult::skipped(
                "Bridged Token Beacon",
                "governor is not the current owner",
            );
        }
        OwnershipState::Pending => {} // Unreachable for Ownable contracts
    }

    let spinner = cliclack::spinner();
    spinner.start("Bridged Token Beacon");

    let calldata = build_transfer_ownership_calldata(ctx.new_owner);

    match send_ownership_tx(
        provider,
        beacon,
        calldata,
        ctx.governor,
        ctx.chain_id,
        *ctx.nonce,
        ctx.gas_price,
    )
    .await
    {
        Ok(result) => {
            spinner.stop(format!(
                "Bridged Token Beacon → Transferred (block {})",
                green.apply_to(result.block_number)
            ));
            *ctx.nonce += 1;
            OwnershipResult::success("Bridged Token Beacon", result.tx_hash)
        }
        Err(e) => {
            spinner.error(format!("Bridged Token Beacon transfer failed: {}", e));
            OwnershipResult::failure("Bridged Token Beacon", e.to_string())
        }
    }
}

/// Transfer ownership for chain Governance contract.
pub(crate) async fn transfer_chain_governance<P>(
    provider: &P,
    contracts: &ChainContracts,
    ctx: &mut TransferContext<'_>,
) -> OwnershipResult
where
    P: Provider + Clone,
{
    let green = Style::new().green();

    let Some(governance) = contracts.governance_addr() else {
        return OwnershipResult::skipped("Chain Governance", "governance_addr not configured");
    };

    // Verify governor is current owner before transferring
    match check_ownership_state(
        provider,
        governance,
        ctx.governor,
        "Chain Governance",
        ctx.logger,
    )
    .await
    {
        OwnershipState::Accepted => {} // Good - we can transfer
        OwnershipState::Pending => {
            return OwnershipResult::skipped(
                "Chain Governance",
                "ownership not yet accepted, accept first",
            );
        }
        OwnershipState::NotTransferred => {
            return OwnershipResult::skipped(
                "Chain Governance",
                "governor is not the current owner",
            );
        }
    }

    let spinner = cliclack::spinner();
    spinner.start("Chain Governance");

    let calldata = build_transfer_ownership_calldata(ctx.new_owner);

    match send_ownership_tx(
        provider,
        governance,
        calldata,
        ctx.governor,
        ctx.chain_id,
        *ctx.nonce,
        ctx.gas_price,
    )
    .await
    {
        Ok(result) => {
            spinner.stop(format!(
                "Chain Governance → Transferred (block {})",
                green.apply_to(result.block_number)
            ));
            *ctx.nonce += 1;
            OwnershipResult::success("Chain Governance", result.tx_hash)
        }
        Err(e) => {
            spinner.error(format!("Chain Governance transfer failed: {}", e));
            OwnershipResult::failure("Chain Governance", e.to_string())
        }
    }
}

/// Transfer ownership for chain Chain Admin contract.
pub(crate) async fn transfer_chain_chain_admin<P>(
    provider: &P,
    contracts: &ChainContracts,
    ctx: &mut TransferContext<'_>,
) -> OwnershipResult
where
    P: Provider + Clone,
{
    let green = Style::new().green();

    let Some(chain_admin) = contracts.chain_admin_addr() else {
        return OwnershipResult::skipped("Chain Chain Admin", "chain_admin_addr not configured");
    };

    // Verify governor is current owner before transferring
    match check_ownership_state(
        provider,
        chain_admin,
        ctx.governor,
        "Chain Chain Admin",
        ctx.logger,
    )
    .await
    {
        OwnershipState::Accepted => {} // Good - we can transfer
        OwnershipState::Pending => {
            return OwnershipResult::skipped(
                "Chain Chain Admin",
                "ownership not yet accepted, accept first",
            );
        }
        OwnershipState::NotTransferred => {
            return OwnershipResult::skipped(
                "Chain Chain Admin",
                "governor is not the current owner",
            );
        }
    }

    let spinner = cliclack::spinner();
    spinner.start("Chain Chain Admin");

    let calldata = build_transfer_ownership_calldata(ctx.new_owner);

    match send_ownership_tx(
        provider,
        chain_admin,
        calldata,
        ctx.governor,
        ctx.chain_id,
        *ctx.nonce,
        ctx.gas_price,
    )
    .await
    {
        Ok(result) => {
            spinner.stop(format!(
                "Chain Chain Admin → Transferred (block {})",
                green.apply_to(result.block_number)
            ));
            *ctx.nonce += 1;
            OwnershipResult::success("Chain Chain Admin", result.tx_hash)
        }
        Err(e) => {
            spinner.error(format!("Chain Chain Admin transfer failed: {}", e));
            OwnershipResult::failure("Chain Chain Admin", e.to_string())
        }
    }
}
