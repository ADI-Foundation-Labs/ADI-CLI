//! Display formatting for contract ownership results.

use alloy_primitives::Address;
use std::collections::HashMap;

use crate::error::Result;
use crate::ui;

use super::{has_pending_transfer, ContractOwnership, OwnerQueryResult};

/// Display ownership results in formatted output.
pub(super) fn display_ownership_results(
    title: &str,
    ownerships: &[ContractOwnership],
    known_map: &HashMap<Address, &'static str>,
) -> Result<()> {
    let mut lines = Vec::new();

    // Count pending transfers for summary
    let pending_count = ownerships
        .iter()
        .filter(|o| has_pending_transfer(&o.pending_owner))
        .count();

    for ownership in ownerships {
        format_ownership_entry(ownership, known_map, &mut lines);
        lines.push(String::new());
    }

    // Remove trailing empty line
    if lines.last().is_some_and(|l| l.is_empty()) {
        lines.pop();
    }

    // Add summary if there are pending transfers
    let title_with_summary = if pending_count > 0 {
        format!("{} ({} pending)", title, ui::yellow(pending_count))
    } else {
        title.to_string()
    };

    ui::note(&title_with_summary, lines.join("\n"))?;
    Ok(())
}

/// Format a single ownership entry into display lines.
fn format_ownership_entry(
    ownership: &ContractOwnership,
    known_map: &HashMap<Address, &'static str>,
    lines: &mut Vec<String>,
) {
    let Some(addr) = ownership.address else {
        lines.push(format!(
            "{}: {}",
            ownership.name,
            ui::cyan("not configured")
        ));
        return;
    };

    let has_pending = has_pending_transfer(&ownership.pending_owner);
    if has_pending {
        lines.push(format!(
            "{} {}: {}",
            ui::yellow("[PENDING]"),
            ownership.name,
            ui::green(addr)
        ));
    } else {
        lines.push(format!("{}: {}", ownership.name, ui::green(addr)));
    }

    lines.push(format!(
        "  Owner: {}",
        format_owner_result(&ownership.owner, known_map)
    ));
    lines.push(format!(
        "  Pending: {}",
        format_pending_result(&ownership.pending_owner, known_map)
    ));
}

/// Format owner query result with known address mapping.
fn format_owner_result(
    result: &OwnerQueryResult,
    known_map: &HashMap<Address, &'static str>,
) -> String {
    match result {
        OwnerQueryResult::Ok(addr) => {
            let role = known_map
                .get(addr)
                .map(|r| format!(" ({})", r))
                .unwrap_or_default();
            format!("{}{}", ui::green(addr), ui::cyan(role))
        }
        OwnerQueryResult::Err(e) => ui::yellow(format!("query failed: {}", e)).to_string(),
        OwnerQueryResult::NotConfigured => ui::cyan("not configured").to_string(),
    }
}

/// Format pending owner query result.
/// Active pending transfers are highlighted in yellow for visibility.
fn format_pending_result(
    result: &OwnerQueryResult,
    known_map: &HashMap<Address, &'static str>,
) -> String {
    match result {
        OwnerQueryResult::Ok(addr) if *addr == Address::ZERO => ui::cyan("not set").to_string(),
        OwnerQueryResult::Ok(addr) => {
            // Highlight pending transfers in yellow for visibility
            let role = known_map
                .get(addr)
                .map(|r| format!(" ({})", r))
                .unwrap_or_default();
            format!("{}{}", ui::yellow(addr), ui::cyan(role))
        }
        OwnerQueryResult::Err(_) => {
            // pendingOwner() not implemented is common, show as "not set"
            ui::cyan("not set").to_string()
        }
        OwnerQueryResult::NotConfigured => ui::cyan("not configured").to_string(),
    }
}
