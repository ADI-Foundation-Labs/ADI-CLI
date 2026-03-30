//! Implementation address reader for proxy contracts.
//!
//! Reads implementation contract addresses from EIP-1967 transparent proxies
//! by querying the standard implementation storage slot. Also reads addresses
//! from contract getters for DualVerifier, NativeTokenVault, and AvailL1DAValidator.

mod apply;
mod contracts;
mod readers;
mod slots;
mod types;

pub use apply::apply_implementations;
pub use contracts::read_owner;
pub use slots::{read_implementation_address, read_proxy_admin};
pub use types::ImplementationAddresses;

use adi_types::{EcosystemContracts, Logger};
use alloy_provider::Provider;
use std::sync::Arc;

/// Read all implementation addresses for known proxy contracts.
///
/// Reads implementation addresses from the EIP-1967 storage slot for each
/// proxy contract defined in the ecosystem contracts.
pub async fn read_all_implementations<P: Provider>(
    provider: &P,
    ecosystem: &EcosystemContracts,
    logger: Arc<dyn Logger>,
) -> ImplementationAddresses {
    let mut impls = ImplementationAddresses::default();

    if let Some(ref core) = ecosystem.core_ecosystem_contracts {
        readers::read_core_ecosystem_impls(provider, core, &mut impls, &*logger).await;
    }
    if let Some(ref ctm) = ecosystem.zksync_os_ctm {
        readers::read_ctm_impls(provider, ctm, &mut impls, &*logger).await;
    }
    if let Some(verifier_addr) = ecosystem.verifier_addr() {
        readers::read_verifier_impls(provider, verifier_addr, &mut impls, &*logger).await;
    }
    if let Some(chain_admin_addr) = ecosystem.chain_admin_addr() {
        readers::read_chain_admin_impls(provider, chain_admin_addr, &mut impls, &*logger).await;
    }
    if let Some(ntv_addr) = ecosystem.native_token_vault_addr() {
        readers::read_token_vault_impls(provider, ntv_addr, &mut impls, &*logger).await;
    }
    if let Some(avail_addr) = ecosystem
        .zksync_os_ctm
        .as_ref()
        .and_then(|c| c.avail_l1_da_validator_addr)
    {
        readers::read_avail_impls(provider, avail_addr, &mut impls, &*logger).await;
    }
    if let Some(ref bridges) = ecosystem.bridges {
        readers::read_bridge_impls(provider, bridges, &mut impls, &*logger).await;
    }

    impls
}
