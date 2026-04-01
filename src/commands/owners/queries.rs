//! Query functions for contract ownership fields.

use adi_ecosystem::verification::read_proxy_admin;
use alloy_primitives::Address;
use alloy_provider::Provider;
use alloy_rpc_types::TransactionRequest;
use alloy_sol_types::SolCall;

use super::{getAdminCall, getPendingAdminCall, ownerCall, pendingOwnerCall, OwnerQueryResult};

const ERR_NOT_DEPLOYED: &str = "contract not deployed";

/// ABI-encoded address starts at byte 12 (32-byte word, left-padded).
const ADDR_START: usize = 12;
/// ABI-encoded address ends at byte 32.
const ADDR_END: usize = 32;

/// Query a contract function that returns an address.
///
/// Generic helper for owner(), pendingOwner(), getAdmin(), getPendingAdmin().
pub(super) async fn query_address_field<P: Provider + Clone>(
    provider: &P,
    contract_address: Address,
    contract_name: &str,
    calldata: Vec<u8>,
    field_name: &str,
) -> OwnerQueryResult {
    let tx = TransactionRequest::default()
        .to(contract_address)
        .input(calldata.into());

    let bytes = match provider.call(tx).await {
        Ok(b) => b,
        Err(e) => return err_result(field_name, contract_name, format_rpc_error(&e)),
    };

    let Some(addr_bytes) = bytes.get(ADDR_START..ADDR_END) else {
        let err = if bytes.is_empty() {
            ERR_NOT_DEPLOYED.to_string()
        } else {
            format!("invalid response: {} bytes", bytes.len())
        };
        return err_result(field_name, contract_name, err);
    };

    OwnerQueryResult::Ok(Address::from_slice(addr_bytes))
}

/// Query owner() on a contract.
pub(super) async fn query_owner<P: Provider + Clone>(
    provider: &P,
    contract_address: Address,
    contract_name: &str,
) -> OwnerQueryResult {
    query_address_field(
        provider,
        contract_address,
        contract_name,
        ownerCall {}.abi_encode(),
        "owner",
    )
    .await
}

/// Query pendingOwner() on a contract.
pub(super) async fn query_pending_owner<P: Provider + Clone>(
    provider: &P,
    contract_address: Address,
    contract_name: &str,
) -> OwnerQueryResult {
    query_address_field(
        provider,
        contract_address,
        contract_name,
        pendingOwnerCall {}.abi_encode(),
        "pendingOwner",
    )
    .await
}

/// Query getAdmin() on Diamond Proxy contract.
pub(super) async fn query_admin<P: Provider + Clone>(
    provider: &P,
    contract_address: Address,
    contract_name: &str,
) -> OwnerQueryResult {
    query_address_field(
        provider,
        contract_address,
        contract_name,
        getAdminCall {}.abi_encode(),
        "getAdmin",
    )
    .await
}

/// Query getPendingAdmin() on Diamond Proxy contract.
pub(super) async fn query_pending_admin<P: Provider + Clone>(
    provider: &P,
    contract_address: Address,
    contract_name: &str,
) -> OwnerQueryResult {
    query_address_field(
        provider,
        contract_address,
        contract_name,
        getPendingAdminCall {}.abi_encode(),
        "getPendingAdmin",
    )
    .await
}

/// Query admin from EIP-1967 transparent proxy storage slot.
///
/// Transparent proxies don't expose admin via a function - the admin address
/// is stored in the EIP-1967 admin slot and must be read via eth_getStorageAt.
pub(super) async fn query_proxy_admin<P: Provider + Clone>(
    provider: &P,
    contract_address: Address,
    contract_name: &str,
) -> OwnerQueryResult {
    match read_proxy_admin(provider, contract_address).await {
        Ok(Some(addr)) => OwnerQueryResult::Ok(addr),
        Ok(None) => OwnerQueryResult::Err("admin not set".to_string()),
        Err(e) => {
            log::debug!("Query proxy admin failed for {}: {}", contract_name, e);
            OwnerQueryResult::Err(format_rpc_error(&e))
        }
    }
}

/// Log a query failure and return an error result.
fn err_result(field_name: &str, contract_name: &str, err: String) -> OwnerQueryResult {
    log::debug!(
        "Query {}() failed for {}: {}",
        field_name,
        contract_name,
        err
    );
    OwnerQueryResult::Err(err)
}

/// Format RPC error to a short, readable message.
pub(super) fn format_rpc_error(e: &impl std::fmt::Display) -> String {
    let full = e.to_string();
    // Extract just the meaningful part from verbose RPC errors
    if full.contains("execution reverted") {
        "execution reverted".to_string()
    } else if full.contains("invalid opcode") {
        "invalid opcode".to_string()
    } else if full.contains("out of gas") {
        "out of gas".to_string()
    } else if let Some(start) = full.find("message:") {
        let end = start.saturating_add(50);
        full.get(start..end).unwrap_or(&full).to_string()
    } else {
        // Truncate long errors
        let truncated: String = full.chars().take(60).collect();
        if full.len() > 60 {
            format!("{}...", truncated)
        } else {
            truncated
        }
    }
}
