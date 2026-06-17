//! Transfer ownership handlers for contracts.
//!
//! Standard `Ownable2Step` contracts share a single generic handler
//! ([`transfer_ownable2step`]). The Bridged Token Beacon keeps a dedicated
//! handler because it uses plain `Ownable` (single-step) and its address must
//! be queried from the Native Token Vault.

use super::calldata::build_transfer_ownership_calldata;
use super::status::{check_ownership_state, check_ownership_state_for_ownable};
use super::transaction::send_ownership_tx;
use super::types::{bridgedTokenBeaconCall, OwnershipResult, OwnershipState};
use adi_types::{EcosystemContracts, Logger};
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

/// Transfer ownership of a standard `Ownable2Step` contract to `ctx.new_owner`,
/// signed by the current governor.
///
/// `name` labels logs/results and `address` is the target contract (a `None`
/// yields a skipped result). Skipped when the governor is not the current owner
/// (transfer not possible) or a transfer is still pending acceptance.
pub(crate) async fn transfer_ownable2step<P>(
    provider: &P,
    address: Option<Address>,
    name: &str,
    ctx: &mut TransferContext<'_>,
) -> OwnershipResult
where
    P: Provider + Clone,
{
    let green = Style::new().green();

    let Some(addr) = address else {
        return OwnershipResult::skipped(name, "address not configured");
    };

    // Verify governor is current owner before transferring
    match check_ownership_state(provider, addr, ctx.governor, name, ctx.logger).await {
        OwnershipState::Accepted => {} // Good - we can transfer
        OwnershipState::Pending => {
            return OwnershipResult::skipped(name, "ownership not yet accepted, accept first");
        }
        OwnershipState::NotTransferred => {
            return OwnershipResult::skipped(name, "governor is not the current owner");
        }
    }

    let spinner = cliclack::spinner();
    spinner.start(name.to_string());

    let calldata = build_transfer_ownership_calldata(ctx.new_owner);

    match send_ownership_tx(
        provider,
        addr,
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
                "{} → Transferred (block {})",
                name,
                green.apply_to(result.block_number)
            ));
            *ctx.nonce += 1;
            OwnershipResult::success(name, result.tx_hash)
        }
        Err(e) => {
            spinner.error(format!("{} transfer failed: {}", name, e));
            OwnershipResult::failure(name, e.to_string())
        }
    }
}

/// Transfer ownership for Bridged Token Beacon contract.
///
/// Note: This contract uses Ownable (not Ownable2Step), so ownership
/// transfers immediately without requiring an accept step. Its address is
/// queried from the Native Token Vault rather than configured directly.
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
