//! Implementation address reader for proxy contracts.
//!
//! Reads implementation contract addresses from EIP-1967 transparent proxies
//! by querying the standard implementation storage slot. Also reads addresses
//! from contract getters for DualVerifier, NativeTokenVault, and AvailL1DAValidator.

use adi_types::{EcosystemContracts, Logger};
use alloy_primitives::{Address, Bytes, B256, U256};
use alloy_provider::Provider;
use alloy_rpc_types::TransactionRequest;
use alloy_sol_types::{sol, SolCall};
use std::sync::Arc;

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

// Contract interfaces for reading addresses via RPC calls
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

/// Collected implementation addresses for all known proxy contracts.
#[derive(Debug, Clone, Default)]
pub struct ImplementationAddresses {
    /// Bridgehub implementation address.
    pub bridgehub_impl: Option<Address>,
    /// Message root implementation address.
    pub message_root_impl: Option<Address>,
    /// Native token vault implementation address.
    pub native_token_vault_impl: Option<Address>,
    /// STM deployment tracker implementation address.
    pub stm_deployment_tracker_impl: Option<Address>,
    /// Chain type manager implementation address.
    pub chain_type_manager_impl: Option<Address>,
    /// Server notifier implementation address.
    pub server_notifier_impl: Option<Address>,
    /// ERC20 bridge implementation address.
    pub erc20_bridge_impl: Option<Address>,
    /// Shared bridge (L1 Asset Router) implementation address.
    pub shared_bridge_impl: Option<Address>,
    /// L1 Nullifier implementation address.
    pub l1_nullifier_impl: Option<Address>,

    // New fields for remaining contracts
    /// Validator timelock implementation address.
    pub validator_timelock_impl: Option<Address>,
    /// Verifier Fflonk address (from DualVerifier).
    pub verifier_fflonk: Option<Address>,
    /// Verifier Plonk address (from DualVerifier).
    pub verifier_plonk: Option<Address>,
    /// Bridged token beacon address (from NativeTokenVault).
    pub bridged_token_beacon: Option<Address>,
    /// Bridged standard ERC20 implementation (from beacon).
    pub bridged_standard_erc20: Option<Address>,
    /// Dummy Avail Bridge address (from AvailL1DAValidator).
    pub dummy_avail_bridge: Option<Address>,
    /// Dummy VectorX address (from AvailL1DAValidator).
    pub dummy_vector_x: Option<Address>,
    /// Server notifier proxy admin address.
    pub server_notifier_proxy_admin: Option<Address>,
    /// Verifier owner address (for constructor arg).
    pub verifier_owner: Option<Address>,
    /// ChainAdmin owner address (for constructor arg).
    pub chain_admin_owner: Option<Address>,
    /// Whether the verifier is a testnet verifier (ZKsyncOSTestnetVerifier).
    /// None = unknown (no owner, likely EraDualVerifier)
    /// Some(true) = testnet verifier (ZKsyncOSTestnetVerifier or EraTestnetVerifier)
    /// Some(false) = production verifier (ZKsyncOSDualVerifier or EraDualVerifier)
    pub is_testnet_verifier: Option<bool>,
}

/// Read implementation address from a proxy contract's EIP-1967 storage slot.
///
/// # Arguments
///
/// * `provider` - Alloy provider for RPC calls
/// * `proxy_address` - Address of the proxy contract
///
/// # Returns
///
/// Implementation address if found and non-zero, None otherwise.
pub async fn read_implementation_address<P: Provider>(
    provider: &P,
    proxy_address: Address,
) -> Result<Option<Address>, String> {
    let storage = provider
        .get_storage_at(proxy_address, U256::from_be_bytes(IMPLEMENTATION_SLOT.0))
        .await
        .map_err(|e| format!("Failed to read storage at {}: {}", proxy_address, e))?;

    // Convert U256 to Address (last 20 bytes)
    let bytes: [u8; 32] = storage.to_be_bytes();
    let addr_bytes: [u8; 20] = bytes[12..32]
        .try_into()
        .map_err(|_| "Invalid address bytes")?;
    let addr = Address::from(addr_bytes);

    if addr == Address::ZERO {
        Ok(None)
    } else {
        Ok(Some(addr))
    }
}

/// Read proxy admin address from EIP-1967 admin storage slot.
///
/// # Arguments
///
/// * `provider` - Alloy provider for RPC calls
/// * `proxy_address` - Address of the proxy contract
///
/// # Returns
///
/// Admin address if found and non-zero, None otherwise.
pub async fn read_proxy_admin<P: Provider>(
    provider: &P,
    proxy_address: Address,
) -> Result<Option<Address>, String> {
    let storage = provider
        .get_storage_at(proxy_address, U256::from_be_bytes(ADMIN_SLOT.0))
        .await
        .map_err(|e| format!("Failed to read admin slot at {}: {}", proxy_address, e))?;

    // Convert U256 to Address (last 20 bytes)
    let bytes: [u8; 32] = storage.to_be_bytes();
    let addr_bytes: [u8; 20] = bytes[12..32]
        .try_into()
        .map_err(|_| "Invalid address bytes")?;
    let addr = Address::from(addr_bytes);

    if addr == Address::ZERO {
        Ok(None)
    } else {
        Ok(Some(addr))
    }
}

/// Helper to make a contract call and decode the address result.
async fn call_contract_address<P: Provider>(
    provider: &P,
    to: Address,
    calldata: Bytes,
) -> Option<Address> {
    let tx = TransactionRequest::default().to(to).input(calldata.into());

    let result = provider.call(tx).await.ok()?;
    if result.len() < 32 {
        return None;
    }

    // Address is in the last 20 bytes of the 32-byte word
    let bytes: &[u8] = result.as_ref();
    let addr_bytes: [u8; 20] = bytes.get(12..32)?.try_into().ok()?;
    let addr = Address::from(addr_bytes);

    if addr == Address::ZERO {
        None
    } else {
        Some(addr)
    }
}

/// Read verifier component addresses from DualVerifier contract.
///
/// # Arguments
///
/// * `provider` - Alloy provider for RPC calls
/// * `verifier_addr` - Address of the DualVerifier contract
///
/// # Returns
///
/// Tuple of (fflonk_verifier, plonk_verifier) addresses.
pub async fn read_verifier_components<P: Provider>(
    provider: &P,
    verifier_addr: Address,
) -> (Option<Address>, Option<Address>) {
    // Call fflonkVerifiers(0)
    let fflonk_call = IDualVerifier::fflonkVerifiersCall { version: 0 };
    let fflonk_data = Bytes::from(fflonk_call.abi_encode());
    let fflonk = call_contract_address(provider, verifier_addr, fflonk_data).await;

    // Call plonkVerifiers(0)
    let plonk_call = IDualVerifier::plonkVerifiersCall { version: 0 };
    let plonk_data = Bytes::from(plonk_call.abi_encode());
    let plonk = call_contract_address(provider, verifier_addr, plonk_data).await;

    (fflonk, plonk)
}

/// Read bridged token addresses from NativeTokenVault.
///
/// # Arguments
///
/// * `provider` - Alloy provider for RPC calls
/// * `ntv_addr` - Address of the NativeTokenVault contract
///
/// # Returns
///
/// Tuple of (beacon_address, erc20_implementation) addresses.
pub async fn read_bridged_token_addresses<P: Provider>(
    provider: &P,
    ntv_addr: Address,
) -> (Option<Address>, Option<Address>) {
    // Call bridgedTokenBeacon()
    let beacon_call = INativeTokenVault::bridgedTokenBeaconCall {};
    let beacon_data = Bytes::from(beacon_call.abi_encode());
    let beacon_addr = call_contract_address(provider, ntv_addr, beacon_data).await;

    // If we got the beacon, read its implementation
    let impl_addr = if let Some(beacon) = beacon_addr {
        let impl_call = IBeacon::implementationCall {};
        let impl_data = Bytes::from(impl_call.abi_encode());
        call_contract_address(provider, beacon, impl_data).await
    } else {
        None
    };

    (beacon_addr, impl_addr)
}

/// Read Avail addresses from AvailL1DAValidator.
///
/// # Arguments
///
/// * `provider` - Alloy provider for RPC calls
/// * `avail_validator_addr` - Address of the AvailL1DAValidator contract
///
/// # Returns
///
/// Tuple of (avail_bridge, vector_x) addresses.
pub async fn read_avail_addresses<P: Provider>(
    provider: &P,
    avail_validator_addr: Address,
) -> (Option<Address>, Option<Address>) {
    // Call AVAIL_BRIDGE()
    let bridge_call = IAvailValidator::AVAIL_BRIDGECall {};
    let bridge_data = Bytes::from(bridge_call.abi_encode());
    let bridge = call_contract_address(provider, avail_validator_addr, bridge_data).await;

    // Call VECTOR_X()
    let vectorx_call = IAvailValidator::VECTOR_XCall {};
    let vectorx_data = Bytes::from(vectorx_call.abi_encode());
    let vectorx = call_contract_address(provider, avail_validator_addr, vectorx_data).await;

    (bridge, vectorx)
}

/// Read owner address from Ownable2Step contract.
///
/// # Arguments
///
/// * `provider` - Alloy provider for RPC calls
/// * `addr` - Address of the Ownable2Step contract
///
/// # Returns
///
/// Owner address if found and non-zero, None otherwise.
pub async fn read_owner<P: Provider>(provider: &P, addr: Address) -> Option<Address> {
    let call = IOwnable2Step::ownerCall {};
    let data = Bytes::from(call.abi_encode());
    call_contract_address(provider, addr, data).await
}

/// Check if a DualVerifier is a testnet verifier.
///
/// Testnet verifiers (ZKsyncOSTestnetVerifier, EraTestnetVerifier) support mock verification,
/// while production verifiers (ZKsyncOSDualVerifier, EraDualVerifier) revert with MockVerifierNotSupported.
///
/// # Arguments
///
/// * `provider` - Alloy provider for RPC calls
/// * `verifier_addr` - Address of the DualVerifier contract
///
/// # Returns
///
/// * `Some(true)` - Testnet verifier (mockVerify succeeds)
/// * `Some(false)` - Production verifier (mockVerify reverts)
/// * `None` - Unknown (call failed for other reasons)
pub async fn is_testnet_verifier<P: Provider>(
    provider: &P,
    verifier_addr: Address,
) -> Option<bool> {
    // Call mockVerify with test inputs: publicInputs=[1], proof=[13, 1]
    // ZKsyncOSTestnetVerifier: proof[0]=13, proof[1]=publicInputs[0]=1 → returns true
    // ZKsyncOSDualVerifier: reverts with MockVerifierNotSupported
    let public_inputs = vec![U256::from(1)];
    let proof = vec![U256::from(13), U256::from(1)];

    let call = IVerifierMock::mockVerifyCall {
        _publicInputs: public_inputs,
        _proof: proof,
    };
    let data = Bytes::from(call.abi_encode());

    let tx = TransactionRequest::default()
        .to(verifier_addr)
        .input(data.into());

    match provider.call(tx).await {
        Ok(_) => Some(true),   // Call succeeded → testnet verifier
        Err(_) => Some(false), // Call reverted → production verifier
    }
}

/// Read all implementation addresses for known proxy contracts.
///
/// Reads implementation addresses from the EIP-1967 storage slot for each
/// proxy contract defined in the ecosystem contracts.
///
/// # Arguments
///
/// * `provider` - Alloy provider for RPC calls
/// * `ecosystem` - Ecosystem contracts containing proxy addresses
/// * `logger` - Logger for debug output
///
/// # Returns
///
/// Collected implementation addresses. Missing addresses are set to None.
pub async fn read_all_implementations<P: Provider>(
    provider: &P,
    ecosystem: &EcosystemContracts,
    logger: Arc<dyn Logger>,
) -> ImplementationAddresses {
    let mut impls = ImplementationAddresses::default();

    // Helper to read impl with logging
    async fn read_impl<P: Provider>(
        provider: &P,
        name: &str,
        proxy_addr: Option<Address>,
        logger: &dyn Logger,
    ) -> Option<Address> {
        let proxy = proxy_addr?;
        logger.debug(&format!(
            "Reading {} implementation from proxy {}",
            name, proxy
        ));
        match read_implementation_address(provider, proxy).await {
            Ok(Some(impl_addr)) => {
                logger.debug(&format!("  {} impl: {}", name, impl_addr));
                Some(impl_addr)
            }
            Ok(None) => {
                logger.debug(&format!("  {} impl: not set (zero address)", name));
                None
            }
            Err(e) => {
                logger.warning(&format!("Failed to read {} impl: {}", name, e));
                None
            }
        }
    }

    // Read implementations for core ecosystem proxies
    if let Some(ref core) = ecosystem.core_ecosystem_contracts {
        impls.bridgehub_impl =
            read_impl(provider, "Bridgehub", core.bridgehub_proxy_addr, &*logger).await;
        impls.message_root_impl = read_impl(
            provider,
            "MessageRoot",
            core.message_root_proxy_addr,
            &*logger,
        )
        .await;
        impls.native_token_vault_impl = read_impl(
            provider,
            "NativeTokenVault",
            core.native_token_vault_addr,
            &*logger,
        )
        .await;
        impls.stm_deployment_tracker_impl = read_impl(
            provider,
            "StmDeploymentTracker",
            core.stm_deployment_tracker_proxy_addr,
            &*logger,
        )
        .await;
    }

    // Read implementations for ZkSync OS CTM proxies
    if let Some(ref ctm) = ecosystem.zksync_os_ctm {
        impls.chain_type_manager_impl = read_impl(
            provider,
            "ChainTypeManager",
            ctm.state_transition_proxy_addr,
            &*logger,
        )
        .await;
        impls.server_notifier_impl = read_impl(
            provider,
            "ServerNotifier",
            ctm.server_notifier_proxy_addr,
            &*logger,
        )
        .await;

        // ValidatorTimelock implementation (EIP-1967)
        impls.validator_timelock_impl = read_impl(
            provider,
            "ValidatorTimelock",
            ctm.validator_timelock_addr,
            &*logger,
        )
        .await;

        // Server notifier proxy admin (EIP-1967 admin slot)
        if let Some(proxy_addr) = ctm.server_notifier_proxy_addr {
            logger.debug(&format!(
                "Reading ServerNotifier proxy admin from {}",
                proxy_addr
            ));
            match read_proxy_admin(provider, proxy_addr).await {
                Ok(Some(admin)) => {
                    logger.debug(&format!("  ServerNotifier proxy admin: {}", admin));
                    impls.server_notifier_proxy_admin = Some(admin);
                }
                Ok(None) => {
                    logger.debug("  ServerNotifier proxy admin: not set (zero address)");
                }
                Err(e) => {
                    logger.warning(&format!("Failed to read ServerNotifier proxy admin: {}", e));
                }
            }
        }
    }

    // Read verifier components from DualVerifier
    if let Some(verifier_addr) = ecosystem.verifier_addr() {
        logger.debug(&format!(
            "Reading verifier components from DualVerifier {}",
            verifier_addr
        ));
        let (fflonk, plonk) = read_verifier_components(provider, verifier_addr).await;
        if let Some(addr) = fflonk {
            logger.debug(&format!("  VerifierFflonk: {}", addr));
            impls.verifier_fflonk = Some(addr);
        }
        if let Some(addr) = plonk {
            logger.debug(&format!("  VerifierPlonk: {}", addr));
            impls.verifier_plonk = Some(addr);
        }

        // Read verifier owner (for constructor args)
        logger.debug(&format!("Reading verifier owner from {}", verifier_addr));
        if let Some(owner) = read_owner(provider, verifier_addr).await {
            logger.debug(&format!("  VerifierOwner: {}", owner));
            impls.verifier_owner = Some(owner);

            // Detect if testnet verifier (only relevant when owner exists)
            logger.debug(&format!(
                "Detecting verifier type via mockVerify on {}",
                verifier_addr
            ));
            if let Some(is_testnet) = is_testnet_verifier(provider, verifier_addr).await {
                logger.debug(&format!(
                    "  Verifier type: {}",
                    if is_testnet { "testnet" } else { "production" }
                ));
                impls.is_testnet_verifier = Some(is_testnet);
            }
        }
    }

    // Read chain admin owner (for constructor args)
    if let Some(chain_admin_addr) = ecosystem.chain_admin_addr() {
        logger.debug(&format!(
            "Reading ChainAdmin owner from {}",
            chain_admin_addr
        ));
        if let Some(owner) = read_owner(provider, chain_admin_addr).await {
            logger.debug(&format!("  ChainAdmin owner: {}", owner));
            impls.chain_admin_owner = Some(owner);
        }
    }

    // Read bridged token addresses from NativeTokenVault
    if let Some(ntv_addr) = ecosystem.native_token_vault_addr() {
        logger.debug(&format!(
            "Reading bridged token addresses from NativeTokenVault {}",
            ntv_addr
        ));
        let (beacon, erc20_impl) = read_bridged_token_addresses(provider, ntv_addr).await;
        if let Some(addr) = beacon {
            logger.debug(&format!("  BridgedTokenBeacon: {}", addr));
            impls.bridged_token_beacon = Some(addr);
        }
        if let Some(addr) = erc20_impl {
            logger.debug(&format!("  BridgedStandardERC20: {}", addr));
            impls.bridged_standard_erc20 = Some(addr);
        }
    }

    // Read Avail addresses from AvailL1DAValidator
    if let Some(avail_addr) = ecosystem
        .zksync_os_ctm
        .as_ref()
        .and_then(|c| c.avail_l1_da_validator_addr)
    {
        logger.debug(&format!(
            "Reading Avail addresses from AvailL1DAValidator {}",
            avail_addr
        ));
        let (bridge, vectorx) = read_avail_addresses(provider, avail_addr).await;
        if let Some(addr) = bridge {
            logger.debug(&format!("  DummyAvailBridge: {}", addr));
            impls.dummy_avail_bridge = Some(addr);
        }
        if let Some(addr) = vectorx {
            logger.debug(&format!("  DummyVectorX: {}", addr));
            impls.dummy_vector_x = Some(addr);
        }
    }

    // Read implementations for bridge proxies
    if let Some(ref bridges) = ecosystem.bridges {
        impls.erc20_bridge_impl = read_impl(
            provider,
            "Erc20Bridge",
            bridges.erc20.as_ref().and_then(|b| b.l1_address),
            &*logger,
        )
        .await;
        impls.shared_bridge_impl = read_impl(
            provider,
            "SharedBridge",
            bridges.shared.as_ref().and_then(|b| b.l1_address),
            &*logger,
        )
        .await;
        impls.l1_nullifier_impl =
            read_impl(provider, "L1Nullifier", bridges.l1_nullifier_addr, &*logger).await;
    }

    impls
}

/// Apply collected implementation addresses to ecosystem contracts.
///
/// Mutates the ZkSyncOsCtm struct to include the implementation addresses.
pub fn apply_implementations(ecosystem: &mut EcosystemContracts, impls: &ImplementationAddresses) {
    if let Some(ref mut ctm) = ecosystem.zksync_os_ctm {
        // Original implementation addresses
        ctm.bridgehub_impl_addr = impls.bridgehub_impl;
        ctm.message_root_impl_addr = impls.message_root_impl;
        ctm.native_token_vault_impl_addr = impls.native_token_vault_impl;
        ctm.stm_deployment_tracker_impl_addr = impls.stm_deployment_tracker_impl;
        ctm.chain_type_manager_impl_addr = impls.chain_type_manager_impl;
        ctm.server_notifier_impl_addr = impls.server_notifier_impl;
        ctm.erc20_bridge_impl_addr = impls.erc20_bridge_impl;
        ctm.shared_bridge_impl_addr = impls.shared_bridge_impl;
        ctm.l1_nullifier_impl_addr = impls.l1_nullifier_impl;

        // New addresses (8 additional contracts)
        ctm.validator_timelock_impl_addr = impls.validator_timelock_impl;
        ctm.verifier_fflonk_addr = impls.verifier_fflonk;
        ctm.verifier_plonk_addr = impls.verifier_plonk;
        ctm.bridged_token_beacon_addr = impls.bridged_token_beacon;
        ctm.bridged_standard_erc20_addr = impls.bridged_standard_erc20;
        ctm.dummy_avail_bridge_addr = impls.dummy_avail_bridge;
        ctm.dummy_vector_x_addr = impls.dummy_vector_x;
        ctm.server_notifier_proxy_admin_addr = impls.server_notifier_proxy_admin;

        // Verifier owner (for constructor args)
        ctm.verifier_owner_addr = impls.verifier_owner;

        // Testnet verifier flag
        ctm.is_testnet_verifier = impls.is_testnet_verifier;
    }

    // ChainAdmin owner (for constructor args)
    ecosystem.chain_admin_owner = impls.chain_admin_owner;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_implementation_slot_constant() {
        // Verify the EIP-1967 slot is correct
        // keccak256('eip1967.proxy.implementation') - 1
        let expected = alloy_primitives::b256!(
            "360894a13ba1a3210667c828492db98dca3e2076cc3735a920a3ca505d382bbc"
        );
        assert_eq!(IMPLEMENTATION_SLOT, expected);
    }

    #[test]
    fn test_admin_slot_constant() {
        // Verify the EIP-1967 admin slot is correct
        // keccak256('eip1967.proxy.admin') - 1
        let expected = alloy_primitives::b256!(
            "b53127684a568b3173ae13b9f8a6016e243e63b6e8ee1178d6a717850b5d6103"
        );
        assert_eq!(ADMIN_SLOT, expected);
    }

    #[test]
    fn test_implementation_addresses_default() {
        let impls = ImplementationAddresses::default();
        assert!(impls.bridgehub_impl.is_none());
        assert!(impls.message_root_impl.is_none());
        assert!(impls.chain_type_manager_impl.is_none());
        // Test new fields
        assert!(impls.validator_timelock_impl.is_none());
        assert!(impls.verifier_fflonk.is_none());
        assert!(impls.verifier_plonk.is_none());
        assert!(impls.bridged_token_beacon.is_none());
        assert!(impls.bridged_standard_erc20.is_none());
        assert!(impls.dummy_avail_bridge.is_none());
        assert!(impls.dummy_vector_x.is_none());
        assert!(impls.server_notifier_proxy_admin.is_none());
        assert!(impls.verifier_owner.is_none());
        assert!(impls.chain_admin_owner.is_none());
    }
}
