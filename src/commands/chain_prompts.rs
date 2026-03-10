//! Interactive prompts for chain configuration.
//!
//! Provides prompts for all chain configuration options, skipping those
//! already provided via CLI arguments.

use adi_ecosystem::{
    ChainDefaults, ChainFundingDefaults, ChainOwnershipDefaults, OperatorsDefaults, ProverMode,
};
use alloy_primitives::Address;

use crate::error::{Result, WrapErr};
use crate::ui;

/// Partial chain configuration from CLI arguments.
///
/// Fields that are `Some` were provided via CLI and should not be prompted.
/// Fields that are `None` need to be prompted interactively.
#[derive(Debug, Default)]
pub struct PartialChainDefaults {
    pub name: Option<String>,
    pub chain_id: Option<u64>,
    pub prover_mode: Option<ProverMode>,
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
        operators,
        funding,
        ownership,
    })
}

/// Prompt for chain name if not already provided.
/// Validates uniqueness against existing chains in the ecosystem.
fn prompt_chain_name(provided: Option<&str>, existing_chains: &[(String, u64)]) -> Result<String> {
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
fn prompt_chain_id(provided: Option<u64>, existing_chains: &[(String, u64)]) -> Result<u64> {
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
fn prompt_prover_mode(provided: Option<ProverMode>) -> Result<ProverMode> {
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

/// Prompt for base token configuration.
///
/// Returns (address, nominator, denominator).
fn prompt_base_token(
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

/// Prompt for EVM emulator if not already provided.
fn prompt_evm_emulator(provided: Option<bool>) -> Result<bool> {
    if let Some(enabled) = provided {
        return Ok(enabled);
    }

    ui::confirm("Enable EVM emulator?")
        .initial_value(false)
        .interact()
        .wrap_err("Failed to read confirmation")
}

/// Prompt for operators configuration (grouped).
/// All 3 addresses are required when user opts-in, or none.
fn prompt_operators_section(
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
fn prompt_funding_section(
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
fn prompt_ownership_section(provided_owner: Option<Address>) -> Result<ChainOwnershipDefaults> {
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

/// Prompt for a required Ethereum address.
fn prompt_required_address(label: &str) -> Result<Address> {
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
fn prompt_optional_address(label: &str) -> Result<Option<Address>> {
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
fn prompt_optional_eth(label: &str) -> Result<Option<f64>> {
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
