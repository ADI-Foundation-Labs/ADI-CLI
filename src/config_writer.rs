//! Config file update functionality.
//!
//! This module provides functions to update the CLI configuration file
//! after adding new chains. Uses append-based modification to preserve
//! comments and formatting.

use adi_ecosystem::ChainDefaults;
use std::path::Path;

use crate::config::Config;
use crate::error::{Result, WrapErr};
use crate::ui;

/// Add a chain to the config file.
///
/// Uses append-based modification to preserve comments and formatting.
/// If the config file doesn't exist, creates it with the new chain.
/// If the config file exists, appends the chain to `ecosystem.chains[]`.
///
/// # Arguments
///
/// * `chain` - Chain configuration to add
/// * `config_path` - Path to the config file
///
/// # Errors
///
/// Returns error if read/write fails. Idempotent: succeeds silently if chain exists.
pub fn add_chain_to_config(chain: &ChainDefaults, config_path: &Path) -> Result<()> {
    // Serialize chain to minimal YAML (omitting default values)
    let chain_yaml = chain.to_minimal_yaml();

    if !config_path.exists() {
        // Create new file with just ecosystem.chains section
        let content = format!(
            "ecosystem:\n  chains:\n{}",
            indent_as_array_item(&chain_yaml, 4)
        );
        std::fs::write(config_path, content).wrap_err("Failed to write config file")?;

        ui::success(format!(
            "Created config file at {}",
            ui::green(config_path.display())
        ))?;
        return Ok(());
    }

    // Read existing content
    let content = std::fs::read_to_string(config_path).wrap_err("Failed to read config file")?;

    // Parse to check if chain already exists (idempotent - skip if exists)
    let config: Config = serde_yaml::from_str(&content).wrap_err("Failed to parse config file")?;
    if config.ecosystem.get_chain(&chain.name).is_some() {
        return Ok(());
    }

    // Determine where to insert and build new content
    let new_content = if let Some(pos) = find_chains_insertion_point(&content) {
        // Append to existing chains array
        let (before, after) = content.split_at(pos);
        format!(
            "{}{}{}",
            before,
            indent_as_array_item(&chain_yaml, 4),
            after
        )
    } else if content.contains("ecosystem:") {
        // Add chains section under ecosystem
        insert_chains_under_ecosystem(&content, &chain_yaml)?
    } else {
        // Add ecosystem.chains section at end
        format!(
            "{}\necosystem:\n  chains:\n{}",
            content.trim_end(),
            indent_as_array_item(&chain_yaml, 4)
        )
    };

    // Backup existing config
    let backup_path = format!("{}.bak", config_path.display());
    std::fs::copy(config_path, &backup_path).wrap_err("Failed to backup config file")?;

    std::fs::write(config_path, new_content).wrap_err("Failed to write config file")?;

    ui::success(format!(
        "Updated config file at {}",
        ui::green(config_path.display())
    ))?;

    Ok(())
}

/// Indent YAML as an array item (first line gets "- ", rest get "  ").
fn indent_as_array_item(yaml: &str, spaces: usize) -> String {
    let indent = " ".repeat(spaces);
    let mut result = String::new();
    for (i, line) in yaml.lines().enumerate() {
        if i == 0 {
            result.push_str(&format!("{}- {}\n", indent, line));
        } else if !line.is_empty() {
            result.push_str(&format!("{}  {}\n", indent, line));
        }
    }
    result
}

/// Find the position after the last chain entry in the chains array.
/// Returns the byte offset where a new chain should be inserted.
fn find_chains_insertion_point(content: &str) -> Option<usize> {
    // Find "chains:" line
    let chains_start = content.find("chains:")?;
    let after_chains = &content[chains_start..];

    // Find all "- name:" entries under chains and return position after the last one
    let mut last_entry_end = None;
    let mut in_chains_section = false;
    let mut chains_indent = 0;

    for (line_start, line) in after_chains.lines().zip_positions() {
        let trimmed = line.trim_start();
        let current_indent = line.len() - trimmed.len();

        if trimmed.starts_with("chains:") {
            in_chains_section = true;
            chains_indent = current_indent;
            continue;
        }

        if in_chains_section {
            // Check if we've exited the chains section (less indentation)
            if !trimmed.is_empty() && current_indent <= chains_indent && !trimmed.starts_with('-') {
                break;
            }

            // Track end of each array item
            if trimmed.starts_with("- ") && current_indent > chains_indent {
                // Find the end of this array item (next item or section end)
                last_entry_end = Some(chains_start + line_start + line.len() + 1);
            } else if last_entry_end.is_some() && current_indent > chains_indent + 2 {
                // Continuation of current item
                last_entry_end = Some(chains_start + line_start + line.len() + 1);
            }
        }
    }

    last_entry_end
}

/// Helper trait to get line positions.
trait ZipPositions<'a>: Iterator<Item = &'a str> {
    fn zip_positions(self) -> impl Iterator<Item = (usize, &'a str)>;
}

impl<'a, I: Iterator<Item = &'a str>> ZipPositions<'a> for I {
    fn zip_positions(self) -> impl Iterator<Item = (usize, &'a str)> {
        let mut pos = 0;
        self.map(move |line| {
            let start = pos;
            pos += line.len() + 1; // +1 for newline
            (start, line)
        })
    }
}

/// Insert chains section under existing ecosystem section.
fn insert_chains_under_ecosystem(content: &str, chain_yaml: &str) -> Result<String> {
    // Find "ecosystem:" line and insert chains after it
    let eco_pos = content
        .find("ecosystem:")
        .ok_or_else(|| eyre::eyre!("ecosystem section not found"))?;

    // Find end of ecosystem line
    let after_eco = &content[eco_pos..];
    let line_end = after_eco.find('\n').unwrap_or(after_eco.len());

    let insert_pos = eco_pos + line_end + 1;
    let (before, after) = content.split_at(insert_pos);

    Ok(format!(
        "{}  chains:\n{}{}",
        before,
        indent_as_array_item(chain_yaml, 4),
        after
    ))
}

/// Prompt user to save chain config and handle the save.
///
/// Shows a confirmation prompt and saves if user confirms.
///
/// # Arguments
///
/// * `chain` - Chain configuration to save
/// * `config_path` - Path to the config file
///
/// # Returns
///
/// * `Ok(true)` - User confirmed and save succeeded
/// * `Ok(false)` - User declined to save
/// * `Err(_)` - Save failed
pub fn prompt_and_save_chain_config(chain: &ChainDefaults, config_path: &Path) -> Result<bool> {
    let save: bool = ui::confirm(format!(
        "Save chain configuration to {}?",
        ui::green(config_path.display())
    ))
    .initial_value(true)
    .interact()
    .wrap_err("Failed to read confirmation")?;

    if !save {
        return Ok(false);
    }

    add_chain_to_config(chain, config_path)?;
    Ok(true)
}

impl Default for Config {
    fn default() -> Self {
        Self {
            state_dir: crate::config::default_state_dir(),
            debug: false,
            protocol_version: Some("v0.30.1".to_string()),
            ecosystem: adi_ecosystem::EcosystemDefaults::default(),
            state_backend: adi_state::BackendType::default(),
            funding: crate::config::FundingDefaults::default(),
            toolkit: crate::config::ToolkitDefaults::default(),
            ownership: crate::config::OwnershipDefaults::default(),
            verification: crate::config::VerificationDefaults::default(),
            gas_multiplier: 200,
            s3: crate::config::S3Config::default(),
            operators: crate::config::OperatorsConfig::default(),
        }
    }
}
