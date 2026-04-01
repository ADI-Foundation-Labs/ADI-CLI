//! Contract loading and RPC data enhancement for the verify command.

use adi_ecosystem::verification::{
    apply_implementations, parse_diamond_cut_data, read_all_implementations, read_owner,
};
use adi_types::{ChainContracts, EcosystemContracts, Logger};
use std::sync::Arc;
use url::Url;

use crate::error::{Result, WrapErr};

/// Load ecosystem contracts and extract facet addresses from diamond_cut_data.
pub(super) async fn load_ecosystem_contracts(
    state_manager: &adi_state::StateManager,
    logger: &Arc<dyn Logger>,
) -> Result<EcosystemContracts> {
    let mut contracts: EcosystemContracts = state_manager
        .ecosystem()
        .contracts()
        .await
        .wrap_err("Failed to load ecosystem contracts. Have you deployed the ecosystem?")?;

    extract_facet_addresses(&mut contracts, logger);
    Ok(contracts)
}

/// Extract facet addresses from diamond_cut_data if present but not yet extracted.
fn extract_facet_addresses(contracts: &mut EcosystemContracts, logger: &Arc<dyn Logger>) {
    let ctm = match contracts.zksync_os_ctm.as_mut() {
        Some(ctm) if ctm.admin_facet_addr.is_none() => ctm,
        _ => return,
    };

    let diamond_cut_data = match ctm.diamond_cut_data.as_ref() {
        Some(data) => data,
        None => return,
    };

    match parse_diamond_cut_data(diamond_cut_data) {
        Ok(facets) => {
            logger.debug("Extracted facet addresses from diamond_cut_data");
            ctm.admin_facet_addr = facets.admin_facet;
            ctm.executor_facet_addr = facets.executor_facet;
            ctm.mailbox_facet_addr = facets.mailbox_facet;
            ctm.getters_facet_addr = facets.getters_facet;
            ctm.diamond_init_addr = facets.diamond_init;
        }
        Err(e) => {
            logger.warning(&format!("Could not parse diamond_cut_data: {}", e));
        }
    }
}

/// Load chain contracts if a chain name is available.
pub(super) async fn load_chain_contracts(
    chain_name: Option<&str>,
    state_manager: &adi_state::StateManager,
    logger: &Arc<dyn Logger>,
) -> Option<ChainContracts> {
    let name = chain_name?;
    match state_manager.chain(name).contracts().await {
        Ok(contracts) => Some(contracts),
        Err(e) => {
            logger.warning(&format!("Could not load chain '{}' contracts: {}", name, e));
            None
        }
    }
}

/// Read implementation addresses and ChainAdmin owner from RPC.
pub(super) async fn enhance_from_rpc(
    rpc_url: &Url,
    ecosystem_contracts: &mut EcosystemContracts,
    chain_contracts: &mut Option<ChainContracts>,
    logger: Arc<dyn Logger>,
) {
    let spinner = cliclack::spinner();
    spinner.start("Reading contract implementations from RPC...");

    let provider = alloy_provider::ProviderBuilder::new().connect_http(rpc_url.clone());
    let impls = read_all_implementations(&provider, ecosystem_contracts, Arc::clone(&logger)).await;
    apply_implementations(ecosystem_contracts, &impls);

    // Read chain-level ChainAdmin owner if available
    if let Some(ref mut chain) = chain_contracts.as_mut() {
        if let Some(ref mut l1) = chain.l1.as_mut() {
            if let Some(chain_admin_addr) = l1.chain_admin_addr {
                if let Some(owner) = read_owner(&provider, chain_admin_addr).await {
                    l1.chain_admin_owner = Some(owner);
                }
            }
        }
    }

    spinner.stop("Contract implementations loaded");
}
