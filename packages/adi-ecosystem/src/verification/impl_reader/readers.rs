//! Focused reader functions for each contract group.

use super::contracts::{
    is_testnet_verifier, read_avail_addresses, read_bridged_token_addresses, read_owner,
    read_verifier_components,
};
use super::slots::{read_implementation_address, read_proxy_admin};
use super::types::ImplementationAddresses;
use adi_types::{BridgesConfig, CoreEcosystemContracts, Logger, ZkSyncOsCtm};
use alloy_primitives::Address;
use alloy_provider::Provider;

/// Read implementation address with logging, returning None on failure.
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

/// Read implementations for core ecosystem proxies.
pub(super) async fn read_core_ecosystem_impls<P: Provider>(
    provider: &P,
    core: &CoreEcosystemContracts,
    impls: &mut ImplementationAddresses,
    logger: &dyn Logger,
) {
    impls.bridgehub_impl =
        read_impl(provider, "Bridgehub", core.bridgehub_proxy_addr, logger).await;
    impls.message_root_impl = read_impl(
        provider,
        "MessageRoot",
        core.message_root_proxy_addr,
        logger,
    )
    .await;
    impls.native_token_vault_impl = read_impl(
        provider,
        "NativeTokenVault",
        core.native_token_vault_addr,
        logger,
    )
    .await;
    impls.stm_deployment_tracker_impl = read_impl(
        provider,
        "StmDeploymentTracker",
        core.stm_deployment_tracker_proxy_addr,
        logger,
    )
    .await;
}

/// Read implementations for ZkSync OS CTM proxies.
pub(super) async fn read_ctm_impls<P: Provider>(
    provider: &P,
    ctm: &ZkSyncOsCtm,
    impls: &mut ImplementationAddresses,
    logger: &dyn Logger,
) {
    impls.chain_type_manager_impl = read_impl(
        provider,
        "ChainTypeManager",
        ctm.state_transition_proxy_addr,
        logger,
    )
    .await;
    impls.server_notifier_impl = read_impl(
        provider,
        "ServerNotifier",
        ctm.server_notifier_proxy_addr,
        logger,
    )
    .await;
    impls.validator_timelock_impl = read_impl(
        provider,
        "ValidatorTimelock",
        ctm.validator_timelock_addr,
        logger,
    )
    .await;

    // Server notifier proxy admin (EIP-1967 admin slot)
    let Some(proxy_addr) = ctm.server_notifier_proxy_addr else {
        return;
    };
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

/// Read verifier components, owner, and testnet detection.
pub(super) async fn read_verifier_impls<P: Provider>(
    provider: &P,
    verifier_addr: Address,
    impls: &mut ImplementationAddresses,
    logger: &dyn Logger,
) {
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
    let Some(owner) = read_owner(provider, verifier_addr).await else {
        return;
    };
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

/// Read chain admin owner address.
pub(super) async fn read_chain_admin_impls<P: Provider>(
    provider: &P,
    chain_admin_addr: Address,
    impls: &mut ImplementationAddresses,
    logger: &dyn Logger,
) {
    logger.debug(&format!(
        "Reading ChainAdmin owner from {}",
        chain_admin_addr
    ));
    if let Some(owner) = read_owner(provider, chain_admin_addr).await {
        logger.debug(&format!("  ChainAdmin owner: {}", owner));
        impls.chain_admin_owner = Some(owner);
    }
}

/// Read bridged token addresses from NativeTokenVault.
pub(super) async fn read_token_vault_impls<P: Provider>(
    provider: &P,
    ntv_addr: Address,
    impls: &mut ImplementationAddresses,
    logger: &dyn Logger,
) {
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

/// Read Avail addresses from AvailL1DAValidator.
pub(super) async fn read_avail_impls<P: Provider>(
    provider: &P,
    avail_addr: Address,
    impls: &mut ImplementationAddresses,
    logger: &dyn Logger,
) {
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

/// Read implementations for bridge proxies.
pub(super) async fn read_bridge_impls<P: Provider>(
    provider: &P,
    bridges: &BridgesConfig,
    impls: &mut ImplementationAddresses,
    logger: &dyn Logger,
) {
    impls.erc20_bridge_impl = read_impl(
        provider,
        "Erc20Bridge",
        bridges.erc20.as_ref().and_then(|b| b.l1_address),
        logger,
    )
    .await;
    impls.shared_bridge_impl = read_impl(
        provider,
        "SharedBridge",
        bridges.shared.as_ref().and_then(|b| b.l1_address),
        logger,
    )
    .await;
    impls.l1_nullifier_impl =
        read_impl(provider, "L1Nullifier", bridges.l1_nullifier_addr, logger).await;
}
