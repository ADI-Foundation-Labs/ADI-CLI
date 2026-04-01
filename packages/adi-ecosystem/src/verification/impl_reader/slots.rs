//! EIP-1967 storage slot reading for proxy contracts.

use alloy_primitives::{Address, Bytes, B256, U256};
use alloy_provider::Provider;
use alloy_rpc_types::TransactionRequest;

/// Size of an ABI-encoded word in bytes.
const ABI_WORD_SIZE: usize = 32;

/// Byte offset where a 20-byte address starts within a 32-byte ABI word.
const ABI_ADDRESS_OFFSET: usize = 12;

/// EIP-1967 implementation storage slot.
///
/// `bytes32(uint256(keccak256('eip1967.proxy.implementation')) - 1)`
const IMPLEMENTATION_SLOT: B256 =
    alloy_primitives::b256!("360894a13ba1a3210667c828492db98dca3e2076cc3735a920a3ca505d382bbc");

/// EIP-1967 admin storage slot.
///
/// `bytes32(uint256(keccak256('eip1967.proxy.admin')) - 1)`
const ADMIN_SLOT: B256 =
    alloy_primitives::b256!("b53127684a568b3173ae13b9f8a6016e243e63b6e8ee1178d6a717850b5d6103");

/// Read implementation address from a proxy contract's EIP-1967 storage slot.
pub async fn read_implementation_address<P: Provider>(
    provider: &P,
    proxy_address: Address,
) -> Result<Option<Address>, String> {
    let storage = provider
        .get_storage_at(proxy_address, U256::from_be_bytes(IMPLEMENTATION_SLOT.0))
        .await
        .map_err(|e| format!("Failed to read storage at {}: {}", proxy_address, e))?;

    Ok(u256_to_address(storage))
}

/// Read proxy admin address from EIP-1967 admin storage slot.
pub async fn read_proxy_admin<P: Provider>(
    provider: &P,
    proxy_address: Address,
) -> Result<Option<Address>, String> {
    let storage = provider
        .get_storage_at(proxy_address, U256::from_be_bytes(ADMIN_SLOT.0))
        .await
        .map_err(|e| format!("Failed to read admin slot at {}: {}", proxy_address, e))?;

    Ok(u256_to_address(storage))
}

/// Convert a U256 storage value to an Address (last 20 bytes), returning None for zero.
fn u256_to_address(value: U256) -> Option<Address> {
    let bytes: [u8; 32] = value.to_be_bytes();
    let addr = Address::from_slice(&bytes[ABI_ADDRESS_OFFSET..ABI_WORD_SIZE]);
    if addr == Address::ZERO {
        None
    } else {
        Some(addr)
    }
}

/// Make a contract call and decode the address result.
pub(super) async fn call_contract_address<P: Provider>(
    provider: &P,
    to: Address,
    calldata: Bytes,
) -> Option<Address> {
    let tx = TransactionRequest::default().to(to).input(calldata.into());
    let result = provider.call(tx).await.ok()?;

    if result.len() < ABI_WORD_SIZE {
        return None;
    }

    let bytes: &[u8] = result.as_ref();
    let addr = Address::from_slice(bytes.get(ABI_ADDRESS_OFFSET..ABI_WORD_SIZE)?);
    if addr == Address::ZERO {
        None
    } else {
        Some(addr)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_implementation_slot_constant() {
        let expected = alloy_primitives::b256!(
            "360894a13ba1a3210667c828492db98dca3e2076cc3735a920a3ca505d382bbc"
        );
        assert_eq!(IMPLEMENTATION_SLOT, expected);
    }

    #[test]
    fn test_admin_slot_constant() {
        let expected = alloy_primitives::b256!(
            "b53127684a568b3173ae13b9f8a6016e243e63b6e8ee1178d6a717850b5d6103"
        );
        assert_eq!(ADMIN_SLOT, expected);
    }
}
