//! Interactive prompts for chain configuration.
//!
//! Provides prompts for all chain configuration options, skipping those
//! already provided via CLI arguments.

mod inputs;
mod sections;

use adi_ecosystem::ChainDefaults;
use alloy_primitives::Address;

use crate::error::Result;
use crate::ui;

use inputs::{prompt_chain_id, prompt_chain_name, prompt_evm_emulator, prompt_prover_mode};
use sections::{
    prompt_base_token, prompt_funding_section, prompt_operators_section, prompt_ownership_section,
};

/// Partial chain configuration from CLI arguments.
///
/// Fields that are `Some` were provided via CLI and should not be prompted.
/// Fields that are `None` need to be prompted interactively.
#[derive(Debug, Default)]
pub struct PartialChainDefaults {
    pub name: Option<String>,
    pub chain_id: Option<u64>,
    pub prover_mode: Option<adi_ecosystem::ProverMode>,
    pub base_token_address: Option<Address>,
    pub base_token_price_nominator: Option<u64>,
    pub base_token_price_denominator: Option<u64>,
    pub evm_emulator: Option<bool>,
    pub operator: Option<Address>,
    pub prove_operator: Option<Address>,
    pub execute_operator: Option<Address>,
    pub operator_eth: Option<f64>,
    pub prove_operator_eth: Option<f64>,
    pub execute_operator_eth: Option<f64>,
    pub new_owner: Option<Address>,
}

/// Prompt for complete chain configuration interactively.
///
/// Skips fields that are already provided in `partial`.
/// Uses provided values as defaults for prompts.
/// Validates chain name/ID uniqueness against `existing_chains`.
pub fn prompt_chain_defaults(
    partial: &PartialChainDefaults,
    existing_chains: &[(String, u64)],
) -> Result<ChainDefaults> {
    ui::section("Chain Configuration")?;

    // Core config
    let name = prompt_chain_name(partial.name.as_deref(), existing_chains)?;
    let chain_id = prompt_chain_id(partial.chain_id, existing_chains)?;
    let prover_mode = prompt_prover_mode(partial.prover_mode)?;
    let (base_token_address, nominator, denominator) = prompt_base_token(
        partial.base_token_address,
        partial.base_token_price_nominator,
        partial.base_token_price_denominator,
    )?;
    let evm_emulator = prompt_evm_emulator(partial.evm_emulator)?;

    // Operators section (grouped)
    let operators = prompt_operators_section(
        partial.operator,
        partial.prove_operator,
        partial.execute_operator,
    )?;

    // Funding section (grouped)
    let funding = prompt_funding_section(
        partial.operator_eth,
        partial.prove_operator_eth,
        partial.execute_operator_eth,
    )?;

    // Ownership section (grouped)
    let ownership = prompt_ownership_section(partial.new_owner)?;

    Ok(ChainDefaults {
        name,
        chain_id,
        prover_mode,
        base_token_address: if base_token_address == adi_types::ETH_TOKEN_ADDRESS {
            None
        } else {
            Some(base_token_address)
        },
        base_token_price_nominator: nominator,
        base_token_price_denominator: denominator,
        evm_emulator,
        blobs: false, // Default to calldata mode (L3)
        fee_collector_address: None,
        operators,
        funding,
        ownership,
    })
}
