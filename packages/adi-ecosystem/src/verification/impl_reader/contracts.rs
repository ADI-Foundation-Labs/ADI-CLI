//! Contract-specific address readers via RPC calls.

use super::slots::call_contract_address;
use alloy_primitives::{Address, Bytes, U256};
use alloy_provider::Provider;
use alloy_rpc_types::TransactionRequest;
use alloy_sol_types::{sol, SolCall};

sol! {
    /// DualVerifier interface for reading verifier components.
    interface IDualVerifier {
        function fflonkVerifiers(uint32 version) external view returns (address);
        function plonkVerifiers(uint32 version) external view returns (address);
    }

    /// NativeTokenVault interface for reading beacon address.
    interface INativeTokenVault {
        function bridgedTokenBeacon() external view returns (address);
    }

    /// Beacon interface for reading implementation address.
    interface IBeacon {
        function implementation() external view returns (address);
    }

    /// AvailL1DAValidator interface for reading Avail addresses.
    interface IAvailValidator {
        function AVAIL_BRIDGE() external view returns (address);
        function VECTOR_X() external view returns (address);
    }

    /// Ownable2Step interface for reading owner.
    interface IOwnable2Step {
        function owner() external view returns (address);
    }

    /// DualVerifier mockVerify interface for testnet detection.
    /// Production verifier reverts with MockVerifierNotSupported.
    /// Testnet verifier returns true for valid inputs.
    interface IVerifierMock {
        function mockVerify(uint256[] memory _publicInputs, uint256[] memory _proof) external view returns (bool);
    }
}

/// Read verifier component addresses from DualVerifier contract.
///
/// Returns tuple of (fflonk_verifier, plonk_verifier) addresses.
pub async fn read_verifier_components<P: Provider>(
    provider: &P,
    verifier_addr: Address,
) -> (Option<Address>, Option<Address>) {
    let fflonk_data = Bytes::from(IDualVerifier::fflonkVerifiersCall { version: 0 }.abi_encode());
    let fflonk = call_contract_address(provider, verifier_addr, fflonk_data).await;

    let plonk_data = Bytes::from(IDualVerifier::plonkVerifiersCall { version: 0 }.abi_encode());
    let plonk = call_contract_address(provider, verifier_addr, plonk_data).await;

    (fflonk, plonk)
}

/// Read bridged token addresses from NativeTokenVault.
///
/// Returns tuple of (beacon_address, erc20_implementation) addresses.
pub async fn read_bridged_token_addresses<P: Provider>(
    provider: &P,
    ntv_addr: Address,
) -> (Option<Address>, Option<Address>) {
    let beacon_data = Bytes::from(INativeTokenVault::bridgedTokenBeaconCall {}.abi_encode());
    let beacon_addr = call_contract_address(provider, ntv_addr, beacon_data).await;

    let impl_addr = match beacon_addr {
        Some(beacon) => {
            let impl_data = Bytes::from(IBeacon::implementationCall {}.abi_encode());
            call_contract_address(provider, beacon, impl_data).await
        }
        None => None,
    };

    (beacon_addr, impl_addr)
}

/// Read Avail addresses from AvailL1DAValidator.
///
/// Returns tuple of (avail_bridge, vector_x) addresses.
pub async fn read_avail_addresses<P: Provider>(
    provider: &P,
    avail_validator_addr: Address,
) -> (Option<Address>, Option<Address>) {
    let bridge_data = Bytes::from(IAvailValidator::AVAIL_BRIDGECall {}.abi_encode());
    let bridge = call_contract_address(provider, avail_validator_addr, bridge_data).await;

    let vectorx_data = Bytes::from(IAvailValidator::VECTOR_XCall {}.abi_encode());
    let vectorx = call_contract_address(provider, avail_validator_addr, vectorx_data).await;

    (bridge, vectorx)
}

/// Read owner address from Ownable2Step contract.
pub async fn read_owner<P: Provider>(provider: &P, addr: Address) -> Option<Address> {
    let data = Bytes::from(IOwnable2Step::ownerCall {}.abi_encode());
    call_contract_address(provider, addr, data).await
}

/// Check if a DualVerifier is a testnet verifier.
///
/// Testnet verifiers support mock verification, while production verifiers revert.
///
/// * `Some(true)` - Testnet verifier (mockVerify succeeds)
/// * `Some(false)` - Production verifier (mockVerify reverts)
/// * `None` - Unknown (call failed for other reasons)
pub async fn is_testnet_verifier<P: Provider>(
    provider: &P,
    verifier_addr: Address,
) -> Option<bool> {
    let call = IVerifierMock::mockVerifyCall {
        _publicInputs: vec![U256::from(1)],
        _proof: vec![U256::from(13), U256::from(1)],
    };
    let data = Bytes::from(call.abi_encode());

    let tx = TransactionRequest::default()
        .to(verifier_addr)
        .input(data.into());

    match provider.call(tx).await {
        Ok(_) => Some(true),
        Err(_) => Some(false),
    }
}
