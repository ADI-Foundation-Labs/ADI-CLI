//! Display helpers for the deploy command output.

use adi_funding::{AnvilFundingTarget, FundingConfig, FundingPlan, FundingTargetStatus};
use alloy_primitives::{Address, U256};

use crate::error::Result;
use crate::ui;

use super::ownership::OwnershipOperationResult;

/// Display Anvil funding plan with current balances and status.
pub fn display_anvil_funding_plan(targets: &[AnvilFundingTarget]) -> Result<()> {
    let mut lines = vec!["Using Anvil default account (account 0)".to_string()];

    for target in targets {
        let current = format_eth(target.current_balance);
        let required = format_eth(target.amount);
        let status = if target.needs_funding {
            format!("{}", ui::yellow("→ Will fund"))
        } else {
            format!("{}", ui::green("✓ Already funded"))
        };
        let label = format!("{} {}", target.source.prefix(), target.role);
        lines.push(format!(
            "{:20} {} ETH (current: {} ETH) {}",
            label,
            ui::green(&required),
            current,
            status
        ));
    }

    let needs_funding = targets.iter().filter(|t| t.needs_funding).count();
    let already_funded = targets.len() - needs_funding;
    lines.push(format!(
        "\nSummary: {} need funding, {} already funded",
        ui::green(needs_funding),
        already_funded
    ));

    ui::note("Anvil Funding Plan", lines.join("\n"))?;
    Ok(())
}

/// Display funding plan with current balances (production network).
pub fn display_funding_plan(
    funder_address: Address,
    targets: &[FundingTargetStatus],
    token_symbol: Option<&str>,
) -> Result<()> {
    let mut lines = vec![format!("Funder: {}", ui::green(funder_address))];

    for target in targets {
        let current = format_eth(target.current_eth);
        let required = format_eth(target.required_eth);
        let status = if target.needs_eth_funding {
            format!("{}", ui::yellow("→ Will fund"))
        } else {
            format!("{}", ui::green("✓ Already funded"))
        };
        let label = format!("{} {}", target.source.prefix(), target.role);
        lines.push(format!(
            "{:20} {} ETH (current: {} ETH) {}",
            label,
            ui::green(&required),
            current,
            status
        ));

        if let (Some(req_tok), Some(cur_tok)) = (target.required_token, target.current_token) {
            let symbol = token_symbol.unwrap_or("CGT");
            let tok_status = if target.needs_token_funding {
                format!("{}", ui::yellow("→ Will fund"))
            } else {
                format!("{}", ui::green("✓ Already funded"))
            };
            lines.push(format!(
                "{:20} {} {} (current: {} {}) {}",
                "",
                ui::green(format_token(req_tok)),
                symbol,
                format_token(cur_tok),
                symbol,
                tok_status
            ));
        }
    }

    let needs_funding = targets
        .iter()
        .filter(|t| t.needs_eth_funding || t.needs_token_funding)
        .count();
    let already_funded = targets.len() - needs_funding;
    lines.push(format!(
        "\nSummary: {} need funding, {} already funded",
        ui::green(needs_funding),
        already_funded
    ));

    ui::note("Funding Plan", lines.join("\n"))?;
    Ok(())
}

/// Display funding summary (transfer counts, gas cost, balances).
pub fn display_funding_summary(plan: &FundingPlan, config: &FundingConfig) -> Result<()> {
    let mut lines = vec![
        format!("Transfers needed: {}", ui::green(plan.transfer_count())),
        format!(
            "Total ETH to transfer: {} ETH",
            ui::green(format_eth(plan.total_eth_transfers()))
        ),
    ];

    if !plan.total_token_required.is_zero() {
        let symbol = config.token_symbol.as_deref().unwrap_or("tokens");
        lines.push(format!(
            "Total {} to transfer: {} {}",
            symbol,
            ui::green(format_token(plan.total_token_required)),
            symbol
        ));
    }

    lines.push(format!(
        "Estimated gas cost: {} ETH",
        ui::green(format_eth(plan.total_gas_cost))
    ));
    lines.push(format!(
        "Total ETH required: {} ETH",
        ui::green(format_eth(plan.total_eth_required))
    ));
    lines.push(format!(
        "Funder balance: {} ETH",
        ui::green(format_eth(plan.funder_eth_balance))
    ));

    if let Some(token_balance) = plan.funder_token_balance {
        let symbol = config.token_symbol.as_deref().unwrap_or("tokens");
        lines.push(format!(
            "Funder {} balance: {} {}",
            symbol,
            ui::green(format_token(token_balance)),
            symbol
        ));
    }

    let status = if plan.is_valid() {
        format!("{}", ui::green("Sufficient balance"))
    } else {
        format!("{}", ui::yellow("Insufficient balance"))
    };
    lines.push(format!("Status: {}", status));

    ui::note("Funding Summary", lines.join("\n"))?;
    Ok(())
}

/// Display detailed funding plan transfers (for dry-run mode).
pub fn display_plan_details(plan: &FundingPlan) -> Result<()> {
    ui::info("Planned Transfers:")?;
    for (i, transfer) in plan.transfers.iter().enumerate() {
        let amount_str = match &transfer.transfer_type {
            adi_funding::TransferType::Eth { amount } => {
                format!("{} {}", ui::green(format_eth(*amount)), ui::green("ETH"))
            }
            adi_funding::TransferType::Token { amount, symbol, .. } => {
                format!("{} {}", ui::green(format_token(*amount)), ui::green(symbol))
            }
        };
        ui::info(format!(
            "  [{}] {:24} -> {}  ({})",
            i + 1,
            transfer.role,
            ui::green(transfer.to),
            amount_str
        ))?;
    }
    Ok(())
}

/// Display deployment summary and completion message.
pub fn display_deployment_summary(
    ecosystem_name: &str,
    chain_name: &str,
    deployed: &adi_ecosystem::DeployedContracts,
    ownership_result: &OwnershipOperationResult,
    fee_adjuster: Option<Address>,
) -> Result<()> {
    let mut body = format!(
        "Ecosystem: {}\nChain: {}\nDiamond proxy: {}",
        ui::green(ecosystem_name),
        ui::green(chain_name),
        ui::green(deployed.diamond_proxy)
    );
    if let Some(addr) = fee_adjuster {
        body.push_str(&format!("\nFee Adjuster Config: {}", ui::green(addr)));
    }
    ui::note("Deployment Summary", body)?;

    let outro_msg = if ownership_result.new_owner_accepted {
        "Deployment complete with ownership transferred and accepted!"
    } else if ownership_result.transferred {
        "Deployment complete with ownership transferred! New owner must run 'adi accept'."
    } else {
        "Deployment complete! You can now start containers and operate the rollup."
    };
    ui::outro(outro_msg)?;

    Ok(())
}

/// Format U256 as ETH (with 4 decimal places).
pub fn format_eth(wei: U256) -> String {
    let eth_str = wei.to_string();
    let len = eth_str.len();

    if len <= 18 {
        let padded = format!("{:0>18}", eth_str);
        if padded.trim_start_matches('0').is_empty() {
            "0.0000".to_string()
        } else {
            format!("0.{:.4}", &padded[..4.min(padded.len())])
        }
    } else {
        let whole = &eth_str[..len - 18];
        let decimal = &eth_str[len - 18..len - 14];
        format!("{}.{}", whole, decimal)
    }
}

/// Format U256 as token amount (assuming 18 decimals).
pub fn format_token(amount: U256) -> String {
    format_eth(amount)
}
