//! Low-level input prompts for individual chain configuration values.
//!
//! Each function prompts for a single typed value with validation,
//! returning early if a value was already provided via CLI arguments.

use adi_ecosystem::ProverMode;
use alloy_primitives::Address;

use crate::error::{Result, WrapErr};
use crate::ui;

/// Prompt for a required Ethereum address.
pub(crate) fn prompt_required_address(label: &str) -> Result<Address> {
    let addr_str: String = ui::input(label)
        .placeholder("0x...")
        .validate(|input: &String| {
            if input.is_empty() {
                return Err("Address is required");
            }
            input
                .parse::<Address>()
                .map(|_| ())
                .map_err(|_| "Invalid Ethereum address format")
        })
        .interact()
        .wrap_err("Failed to read address")?;

    addr_str
        .parse()
        .wrap_err("Invalid address after validation")
}

/// Prompt for an optional Ethereum address.
pub(crate) fn prompt_optional_address(label: &str) -> Result<Option<Address>> {
    let addr_str: String = ui::input(label)
        .placeholder("0x...")
        .required(false)
        .validate(|input: &String| {
            if input.is_empty() {
                return Ok(()); // Empty is allowed
            }
            input
                .parse::<Address>()
                .map(|_| ())
                .map_err(|_| "Invalid Ethereum address format")
        })
        .interact()
        .wrap_err("Failed to read address")?;

    if addr_str.is_empty() {
        return Ok(None);
    }

    let address: Address = addr_str
        .parse()
        .wrap_err("Invalid address after validation")?;
    Ok(Some(address))
}

/// Prompt for an optional ETH amount.
pub(crate) fn prompt_optional_eth(label: &str) -> Result<Option<f64>> {
    let amount_str: String = ui::input(label)
        .placeholder("e.g., 10.0")
        .required(false)
        .validate(|input: &String| {
            if input.is_empty() {
                return Ok(()); // Empty is allowed
            }
            input
                .parse::<f64>()
                .map(|_| ())
                .map_err(|_| "Must be a valid number")
        })
        .interact()
        .wrap_err("Failed to read ETH amount")?;

    if amount_str.is_empty() {
        return Ok(None);
    }

    let amount: f64 = amount_str
        .parse()
        .wrap_err("Invalid ETH amount after validation")?;
    Ok(Some(amount))
}

/// Prompt for chain name if not already provided.
/// Validates uniqueness against existing chains in the ecosystem.
pub(crate) fn prompt_chain_name(
    provided: Option<&str>,
    existing_chains: &[(String, u64)],
) -> Result<String> {
    // Extract existing names for validation (owned for closure capture)
    let existing_names: Vec<String> = existing_chains.iter().map(|(n, _)| n.clone()).collect();

    if let Some(name) = provided {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err(eyre::eyre!("Chain name cannot be empty"));
        }
        if existing_names.iter().any(|n| n == trimmed) {
            return Err(eyre::eyre!(
                "Chain name '{}' already exists in this ecosystem",
                trimmed
            ));
        }
        return Ok(trimmed.to_string());
    }

    let name: String = ui::input("Chain name")
        .placeholder("my_chain")
        .validate(move |input: &String| {
            let trimmed = input.trim();
            if trimmed.is_empty() {
                return Err("Chain name is required");
            }
            if existing_names.iter().any(|n| n == trimmed) {
                return Err("Chain name already exists in this ecosystem");
            }
            Ok(())
        })
        .interact()
        .wrap_err("Failed to read chain name")?;

    Ok(name.trim().to_string())
}

/// Prompt for chain ID if not already provided.
/// Validates uniqueness against existing chains in the ecosystem.
pub(crate) fn prompt_chain_id(
    provided: Option<u64>,
    existing_chains: &[(String, u64)],
) -> Result<u64> {
    // Extract existing IDs for validation
    let existing_ids: Vec<u64> = existing_chains.iter().map(|(_, id)| *id).collect();

    if let Some(id) = provided {
        if existing_ids.contains(&id) {
            let chain_name = existing_chains
                .iter()
                .find(|(_, cid)| *cid == id)
                .map(|(n, _)| n.as_str())
                .unwrap_or("unknown");
            return Err(eyre::eyre!(
                "Chain ID {} is already used by chain '{}'",
                id,
                chain_name
            ));
        }
        return Ok(id);
    }

    let input_str: String = ui::input("Chain ID")
        .default_input("222")
        .validate(move |input: &String| {
            let id = input
                .parse::<u64>()
                .map_err(|_| "Must be a positive integer")?;
            if existing_ids.contains(&id) {
                return Err("Chain ID already used by another chain in this ecosystem");
            }
            Ok(())
        })
        .interact()
        .wrap_err("Failed to read chain ID")?;

    input_str
        .parse::<u64>()
        .wrap_err("Invalid chain ID after validation")
}

/// Prompt for prover mode if not already provided.
pub(crate) fn prompt_prover_mode(provided: Option<ProverMode>) -> Result<ProverMode> {
    if let Some(mode) = provided {
        return Ok(mode);
    }

    let items = vec![
        (
            ProverMode::NoProofs,
            "no-proofs",
            "Development mode, no ZK proofs".to_string(),
        ),
        (
            ProverMode::Gpu,
            "gpu",
            "Production mode with ZK proofs".to_string(),
        ),
    ];

    ui::select("Prover mode")
        .items(&items)
        .interact()
        .wrap_err("Failed to select prover mode")
}

/// Prompt for EVM emulator if not already provided.
pub(crate) fn prompt_evm_emulator(provided: Option<bool>) -> Result<bool> {
    if let Some(enabled) = provided {
        return Ok(enabled);
    }

    ui::confirm("Enable EVM emulator?")
        .initial_value(false)
        .interact()
        .wrap_err("Failed to read confirmation")
}
