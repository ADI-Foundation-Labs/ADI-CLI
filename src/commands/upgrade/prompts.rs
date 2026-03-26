//! Interactive prompts for upgrade command.

use crate::error::{Result, WrapErr};
use crate::ui;

/// Select chains to upgrade using multi-select picker.
///
/// # Arguments
///
/// * `available_chains` - List of chain names available in ecosystem
/// * `preselected` - Optional chain name from --chain flag
pub fn select_chains(
    available_chains: &[String],
    preselected: Option<&String>,
) -> Result<Vec<String>> {
    // If --chain flag provided, use it directly
    if let Some(chain) = preselected {
        if !available_chains.contains(chain) {
            return Err(eyre::eyre!(
                "Chain '{}' not found. Available: {}",
                chain,
                available_chains.join(", ")
            ));
        }
        return Ok(vec![chain.clone()]);
    }

    // Single chain - auto-select
    if available_chains.len() == 1 {
        let chain = available_chains
            .first()
            .ok_or_else(|| eyre::eyre!("No chains available"))?
            .clone();
        ui::info(format!("Auto-selected chain: {}", ui::green(&chain)))?;
        return Ok(vec![chain]);
    }

    // Multiple chains - show picker
    let items: Vec<(String, String, String)> = available_chains
        .iter()
        .map(|name| (name.clone(), name.clone(), String::new()))
        .collect();

    let selected: Vec<String> = cliclack::multiselect("Select chains to upgrade")
        .items(&items)
        .required(true)
        .interact()
        .wrap_err("Chain selection cancelled")?;

    Ok(selected)
}
