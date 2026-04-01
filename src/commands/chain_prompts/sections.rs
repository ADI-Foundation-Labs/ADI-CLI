//! Grouped prompt sections for chain configuration.
//!
//! Each function prompts for a logical group of related settings
//! (operators, funding, ownership, base token).

use adi_ecosystem::{ChainFundingDefaults, ChainOwnershipDefaults, OperatorsDefaults};
use alloy_primitives::Address;

use crate::error::{Result, WrapErr};
use crate::ui;

use super::inputs::{prompt_optional_address, prompt_optional_eth, prompt_required_address};

/// Prompt for operators configuration (grouped).
/// All 3 addresses are required when user opts-in, or none.
pub(crate) fn prompt_operators_section(
    provided_op: Option<Address>,
    provided_prove: Option<Address>,
    provided_exec: Option<Address>,
) -> Result<OperatorsDefaults> {
    // If all are provided via CLI, use them
    if provided_op.is_some() && provided_prove.is_some() && provided_exec.is_some() {
        return Ok(OperatorsDefaults {
            operator: provided_op,
            prove_operator: provided_prove,
            execute_operator: provided_exec,
        });
    }

    // Ask if user wants to configure operators
    let configure: bool = ui::confirm("Configure custom operator addresses?")
        .initial_value(false)
        .interact()
        .wrap_err("Failed to read confirmation")?;

    if !configure {
        return Ok(OperatorsDefaults::default());
    }

    ui::section("Operator Addresses")?;

    // All 3 are required when user opts-in
    let operator = prompt_required_address("Operator address (commit/precommit/revert)")?;
    let prove_operator = prompt_required_address("Prove operator address (prover)")?;
    let execute_operator = prompt_required_address("Execute operator address (executor)")?;

    Ok(OperatorsDefaults {
        operator: Some(operator),
        prove_operator: Some(prove_operator),
        execute_operator: Some(execute_operator),
    })
}

/// Prompt for funding configuration (grouped).
pub(crate) fn prompt_funding_section(
    provided_op_eth: Option<f64>,
    provided_prove_eth: Option<f64>,
    provided_exec_eth: Option<f64>,
) -> Result<ChainFundingDefaults> {
    // If all are provided, use them
    if provided_op_eth.is_some() && provided_prove_eth.is_some() && provided_exec_eth.is_some() {
        return Ok(ChainFundingDefaults {
            operator_eth: provided_op_eth,
            prove_operator_eth: provided_prove_eth,
            execute_operator_eth: provided_exec_eth,
        });
    }

    // Ask if user wants to configure funding
    let configure: bool = ui::confirm("Configure operator funding amounts?")
        .initial_value(false)
        .interact()
        .wrap_err("Failed to read confirmation")?;

    if !configure {
        return Ok(ChainFundingDefaults {
            operator_eth: provided_op_eth,
            prove_operator_eth: provided_prove_eth,
            execute_operator_eth: provided_exec_eth,
        });
    }

    ui::section("Operator Funding (ETH)")?;

    let operator_eth = if provided_op_eth.is_some() {
        provided_op_eth
    } else {
        prompt_optional_eth("Operator ETH amount")?
    };

    let prove_operator_eth = if provided_prove_eth.is_some() {
        provided_prove_eth
    } else {
        prompt_optional_eth("Prove operator ETH amount")?
    };

    let execute_operator_eth = if provided_exec_eth.is_some() {
        provided_exec_eth
    } else {
        prompt_optional_eth("Execute operator ETH amount")?
    };

    Ok(ChainFundingDefaults {
        operator_eth,
        prove_operator_eth,
        execute_operator_eth,
    })
}

/// Prompt for ownership configuration (grouped).
pub(crate) fn prompt_ownership_section(
    provided_owner: Option<Address>,
) -> Result<ChainOwnershipDefaults> {
    // If provided, use it
    if provided_owner.is_some() {
        return Ok(ChainOwnershipDefaults {
            new_owner: provided_owner,
            private_key: None,
        });
    }

    // Ask if user wants to configure ownership
    let configure: bool = ui::confirm("Configure chain ownership?")
        .initial_value(false)
        .interact()
        .wrap_err("Failed to read confirmation")?;

    if !configure {
        return Ok(ChainOwnershipDefaults::default());
    }

    let new_owner = prompt_optional_address("New owner address")?;

    Ok(ChainOwnershipDefaults {
        new_owner,
        private_key: None,
    })
}

/// Prompt for base token configuration.
///
/// Returns (address, nominator, denominator).
pub(crate) fn prompt_base_token(
    provided_addr: Option<Address>,
    provided_nom: Option<u64>,
    provided_denom: Option<u64>,
) -> Result<(Address, u64, u64)> {
    // If all values are provided, use them
    if let (Some(addr), Some(nom), Some(denom)) = (provided_addr, provided_nom, provided_denom) {
        return Ok((addr, nom, denom));
    }

    // If address is provided but not ratios, use defaults for ratios
    if let Some(addr) = provided_addr {
        return Ok((addr, provided_nom.unwrap_or(1), provided_denom.unwrap_or(1)));
    }

    let use_custom: bool = ui::confirm("Use custom base token?")
        .initial_value(false)
        .interact()
        .wrap_err("Failed to read confirmation")?;

    if !use_custom {
        return Ok((adi_types::ETH_TOKEN_ADDRESS, 1, 1));
    }

    // Prompt for address with validation
    let addr_str: String = ui::input("Base token contract address")
        .placeholder("0x...")
        .validate(|input: &String| {
            input
                .parse::<Address>()
                .map(|_| ())
                .map_err(|_| "Invalid Ethereum address format")
        })
        .interact()
        .wrap_err("Failed to read base token address")?;

    let address: Address = addr_str
        .parse()
        .wrap_err("Invalid address after validation")?;

    // Prompt for price ratio with validation
    let default_ratio = format!(
        "{}:{}",
        provided_nom.unwrap_or(1),
        provided_denom.unwrap_or(1)
    );
    let ratio_str: String = ui::input("Price ratio (nominator:denominator)")
        .default_input(&default_ratio)
        .validate(|input: &String| {
            let parts: Vec<&str> = input.split(':').collect();
            match parts.as_slice() {
                [n, d] => {
                    n.trim().parse::<u64>().map_err(|_| "Invalid nominator")?;
                    d.trim().parse::<u64>().map_err(|_| "Invalid denominator")?;
                    Ok(())
                }
                _ => Err("Use format 'nominator:denominator'"),
            }
        })
        .interact()
        .wrap_err("Failed to read price ratio")?;

    let parts: Vec<&str> = ratio_str.split(':').collect();
    let (nom, denom) = match parts.as_slice() {
        [n, d] => (
            n.trim()
                .parse::<u64>()
                .wrap_err("Invalid nominator after validation")?,
            d.trim()
                .parse::<u64>()
                .wrap_err("Invalid denominator after validation")?,
        ),
        _ => return Err(eyre::eyre!("Invalid price ratio format after validation")),
    };

    Ok((address, nom, denom))
}
