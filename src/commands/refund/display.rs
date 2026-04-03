//! Display and progress helpers for the refund command.

use std::sync::Mutex;

use adi_funding::{format_eth, format_with_decimals, FundingEvent, FundingEventHandler};
use cliclack::ProgressBar;
use console::style;

use crate::error::Result;
use crate::ui;

/// Display the refund plan as a boxed note (matching funding plan style).
pub fn display_plan_summary(plan: &adi_funding::RefundPlan) -> Result<()> {
    let mut lines = vec![format!("Receiver: {}", ui::green(plan.receiver))];

    for target in &plan.targets {
        let label = target.label();
        let mut line = format!(
            "{} ({}) — {} ETH",
            ui::cyan(&label),
            ui::green(target.address),
            ui::green(format_eth(target.sendable_eth)),
        );

        if !target.sendable_token.is_zero() {
            let symbol = plan.token_symbol.as_deref().unwrap_or("CGT");
            let decimals = plan.token_decimals.unwrap_or(18);
            line.push_str(&format!(
                " + {} {}",
                ui::green(format_with_decimals(
                    target.sendable_token,
                    usize::from(decimals)
                )),
                symbol
            ));
        }

        lines.push(line);
    }

    lines.push(format!(
        "\nTotal: {} ETH from {} wallet(s)",
        ui::green(format_eth(plan.total_eth_to_refund)),
        plan.target_count()
    ));

    ui::note("Refund Plan", lines.join("\n"))?;
    Ok(())
}

/// Display refund results with colored output.
pub fn display_results(
    result: &adi_funding::RefundResult,
    plan: &adi_funding::RefundPlan,
) -> Result<()> {
    ui::section("Results")?;

    if result.successful > 0 {
        ui::success(format!("{} transfer(s) successful", result.successful))?;
    }

    if result.failed > 0 {
        ui::warning(format!("{} transfer(s) failed", result.failed))?;
        for err in &result.errors {
            ui::warning(format!("  {err}"))?;
        }
    }

    if !result.total_eth_refunded.is_zero() {
        ui::info(format!(
            "Total ETH refunded: {}",
            style(format_eth(result.total_eth_refunded)).green()
        ))?;
    }

    if !result.total_token_refunded.is_zero() {
        let symbol = plan.token_symbol.as_deref().unwrap_or("tokens");
        let decimals = plan.token_decimals.unwrap_or(18);
        ui::info(format!(
            "Total {} refunded: {}",
            symbol,
            style(format_with_decimals(
                result.total_token_refunded,
                usize::from(decimals)
            ))
            .green()
        ))?;
    }

    Ok(())
}

/// Acquire a mutex lock, recovering from poison if needed.
fn lock_or_recover<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|p| p.into_inner())
}

/// Spinner-based handler for balance checking progress.
pub struct BalanceCheckHandler {
    spinner: Mutex<Option<ProgressBar>>,
    checked: Mutex<usize>,
    total: Mutex<usize>,
}

impl BalanceCheckHandler {
    /// Create a new balance check handler.
    pub fn new() -> Self {
        Self {
            spinner: Mutex::new(None),
            checked: Mutex::new(0),
            total: Mutex::new(0),
        }
    }
}

#[async_trait::async_trait]
impl FundingEventHandler for BalanceCheckHandler {
    async fn on_event(&self, event: FundingEvent) {
        match event {
            FundingEvent::CheckingBalances { wallet_count } => {
                let spinner = cliclack::spinner();
                spinner.start(format!("Checking balances for {wallet_count} wallet(s)..."));
                *lock_or_recover(&self.total) = wallet_count;
                *lock_or_recover(&self.spinner) = Some(spinner);
            }
            FundingEvent::BalanceChecked { role, address, .. } => {
                let mut checked = lock_or_recover(&self.checked);
                *checked += 1;
                let total = *lock_or_recover(&self.total);
                let guard = lock_or_recover(&self.spinner);
                if let Some(spinner) = guard.as_ref() {
                    spinner.set_message(format!(
                        "{} Checking balances [{}/{}] {} ({})",
                        style("◒").magenta(),
                        checked,
                        total,
                        style(role).cyan(),
                        style(address).green(),
                    ));
                }
            }
            FundingEvent::PlanCreated { .. } => {
                let mut guard = lock_or_recover(&self.spinner);
                if let Some(spinner) = guard.take() {
                    let total = *lock_or_recover(&self.total);
                    spinner.stop(format!("Checked balances for {total} wallet(s)"));
                }
            }
            _ => {}
        }
    }
}
