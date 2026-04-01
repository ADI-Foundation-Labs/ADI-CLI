//! Implementation address collection type.

use alloy_primitives::Address;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_implementation_addresses_default() {
        let impls = ImplementationAddresses::default();
        assert!(impls.bridgehub_impl.is_none());
        assert!(impls.message_root_impl.is_none());
        assert!(impls.chain_type_manager_impl.is_none());
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
