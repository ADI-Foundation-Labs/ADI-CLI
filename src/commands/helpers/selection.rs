//! Interactive chain selection helpers for CLI commands.

use adi_state::StateManager;

use super::ChainSelection;
use crate::config::Config;
use crate::error::{Result, WrapErr};
use crate::ui;

/// Select a chain interactively from deployed state.
///
/// Behavior:
/// - If `--chain` provided via CLI: validate it exists and use it
/// - If 1 chain in state: auto-select with info message
/// - If multiple chains: show interactive selection
///
/// # Arguments
///
/// * `chain_arg` - Optional chain name from `--chain` CLI argument
/// * `state_manager` - StateManager to query chains from state
/// * `ecosystem_name` - Ecosystem name for error messages
///
/// # Errors
///
/// Returns error if no chains exist in state or user cancels selection.
pub async fn select_chain_from_state(
    chain_arg: Option<&String>,
    state_manager: &StateManager,
    ecosystem_name: &str,
) -> Result<String> {
    // If CLI arg provided, validate and use it
    if let Some(chain_name) = chain_arg {
        let chains = state_manager.list_chains().await?;
        if chains.contains(chain_name) {
            return Ok(chain_name.clone());
        }
        return Err(eyre::eyre!(
            "Chain '{}' not found in ecosystem '{}'. Available chains: {}",
            chain_name,
            ecosystem_name,
            chains.join(", ")
        ));
    }

    // List available chains from state
    let chains = state_manager.list_chains().await?;

    match chains.len() {
        0 => Err(eyre::eyre!(
            "No chains found in ecosystem '{}'. Run 'adi init' first.",
            ecosystem_name
        )),
        1 => {
            let chain_name = chains
                .into_iter()
                .next()
                .ok_or_else(|| eyre::eyre!("Failed to get chain name"))?;
            ui::info(format!("Auto-selected chain: {}", ui::green(&chain_name)))?;
            Ok(chain_name)
        }
        _ => {
            // Multiple chains - show interactive selection
            let items: Vec<(String, String, String)> = chains
                .into_iter()
                .map(|name| (name.clone(), name, String::new()))
                .collect();

            let selected: String = ui::select("Select a chain")
                .items(&items)
                .interact()
                .wrap_err("Chain selection cancelled")?;

            Ok(selected)
        }
    }
}

/// Select a chain from config defaults or prompt for new chain.
///
/// Behavior:
/// - If `--chain` provided via CLI: existing if in config, else New
/// - If 0 chains in config + allow_new: prompt for new chain name
/// - If 1 chain in config + !allow_new: auto-select
/// - If 1+ chains + allow_new: selection with "Create new" option
///
/// # Arguments
///
/// * `chain_arg` - Optional chain name from CLI argument
/// * `allow_new` - If true, shows "Create new chain..." option
/// * `config` - Config containing `ecosystem.chains[]` defaults
///
/// # Returns
///
/// * `ChainSelection::Existing(name)` - Selected existing chain from config
/// * `ChainSelection::New(name)` - User wants to create new chain
///
/// # Errors
///
/// Returns error if no chains in config and allow_new is false, or user cancels.
pub fn select_chain_from_config(
    chain_arg: Option<&String>,
    allow_new: bool,
    config: &Config,
) -> Result<ChainSelection> {
    let chain_names = config.ecosystem.chain_names();

    // If CLI arg provided, check if it exists in config
    if let Some(chain_name) = chain_arg {
        if chain_names.iter().any(|n| n == chain_name) {
            return Ok(ChainSelection::Existing(chain_name.clone()));
        }
        // CLI user specified a name not in config - treat as new
        return Ok(ChainSelection::New(chain_name.clone()));
    }

    match (chain_names.len(), allow_new) {
        // No chains and can't create new
        (0, false) => Err(eyre::eyre!(
            "No chains defined in config. Add chains to ecosystem.chains[] or provide --chain-name."
        )),

        // No chains but can create new - prompt for name
        (0, true) => {
            let name: String = ui::input("Enter chain name")
                .placeholder("my_chain")
                .interact()
                .wrap_err("Failed to read chain name")?;

            if name.is_empty() {
                return Err(eyre::eyre!("Chain name cannot be empty"));
            }

            Ok(ChainSelection::New(name))
        }

        // Single chain and can't create new - auto-select
        (1, false) => {
            let chain_name = chain_names
                .first()
                .ok_or_else(|| eyre::eyre!("Failed to get chain name"))?
                .to_string();

            ui::info(format!("Auto-selected chain: {}", ui::green(&chain_name)))?;
            Ok(ChainSelection::Existing(chain_name))
        }

        // Has chains - show selection (with or without "Create new" option)
        _ => {
            const CREATE_NEW_VALUE: &str = "__create_new__";

            let mut items: Vec<(String, String, String)> = chain_names
                .iter()
                .map(|name| {
                    (
                        name.to_string(),
                        name.to_string(),
                        "Use config defaults".to_string(),
                    )
                })
                .collect();

            if allow_new {
                items.push((
                    CREATE_NEW_VALUE.to_string(),
                    "[Create new chain...]".to_string(),
                    "Enter custom chain name".to_string(),
                ));
            }

            let selected: String = ui::select("Select a chain")
                .items(&items)
                .interact()
                .wrap_err("Chain selection cancelled")?;

            if selected == CREATE_NEW_VALUE {
                let name: String = ui::input("Enter chain name")
                    .placeholder("my_chain")
                    .interact()
                    .wrap_err("Failed to read chain name")?;

                if name.is_empty() {
                    return Err(eyre::eyre!("Chain name cannot be empty"));
                }

                return Ok(ChainSelection::New(name));
            }

            Ok(ChainSelection::Existing(selected))
        }
    }
}
