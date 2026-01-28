//! Wallet funding logic for deployment operations.
//!
//! This module handles automatic funding of ecosystem and chain wallets
//! before deployment operations. It supports both ETH transfers and
//! ERC-20 token transfers for custom gas tokens (CGT).
//!
//! # Funding Requirements
//!
//! Wallet funding varies based on the chain's base token configuration:
//!
//! - **ETH base token**: Fund wallets with ETH only
//! - **Custom base token (CGT)**: Fund wallets with ETH + CGT
//!
//! | Wallet           | ETH Required | CGT Required* |
//! |------------------|--------------|---------------|
//! | Ecosystem Deployer | 1 ETH       | -             |
//! | Ecosystem Governor | 1 ETH       | 5 CGT         |
//! | Chain Governor     | 1 ETH       | 5 CGT         |
//! | Chain Operator     | 5 ETH       | -             |
//! | Prove Operator     | 5 ETH       | -             |
//! | Execute Operator   | 5 ETH       | -             |
//!
//! *CGT only required when base token != ETH

pub mod transfer;

pub use transfer::{
    check_eth_balance, check_token_balance, transfer_eth, transfer_token, FundingRequirement,
    FundingStatus,
};

use alloy_primitives::{Address, U256};
use colored::Colorize;
use eyre::WrapErr;
use secrecy::ExposeSecret;

use crate::chain::wallets::ChainWallets;
use crate::config::FunderConfig;
use crate::ecosystem::wallets::EcosystemWallets;
use crate::error::Result;
use crate::external::CastCli;

/// Default ETH requirement for deployer wallet (1 ETH in wei).
pub const DEFAULT_DEPLOYER_ETH: u128 = 1_000_000_000_000_000_000;

/// Default ETH requirement for governor wallet (1 ETH in wei).
pub const DEFAULT_GOVERNOR_ETH: u128 = 1_000_000_000_000_000_000;

/// Default CGT requirement for governor wallet (5 tokens with 18 decimals).
pub const DEFAULT_GOVERNOR_CGT: u128 = 5_000_000_000_000_000_000;

/// Default ETH requirement for operator wallets (5 ETH in wei).
pub const DEFAULT_OPERATOR_ETH: u128 = 5_000_000_000_000_000_000;

/// Result of a funding check operation.
#[derive(Debug)]
pub struct FundingCheckResult {
    /// Status of each wallet.
    pub statuses: Vec<FundingStatus>,
    /// Whether all wallets are sufficiently funded.
    pub all_funded: bool,
}

impl FundingCheckResult {
    /// Returns wallets that need funding.
    pub fn underfunded_wallets(&self) -> Vec<&FundingStatus> {
        self.statuses.iter().filter(|s| !s.is_funded()).collect()
    }
}

/// Check funding status for ecosystem wallets.
///
/// # Arguments
///
/// * `wallets` - The ecosystem wallets to check.
/// * `rpc_url` - RPC endpoint URL.
/// * `cgt_address` - Optional CGT contract address (for custom base token chains).
///
/// # Returns
///
/// A `FundingCheckResult` with the status of each wallet.
///
/// # Errors
///
/// Returns an error if balance checking fails.
pub async fn check_ecosystem_funding(
    wallets: &EcosystemWallets,
    rpc_url: &str,
    cgt_address: Option<Address>,
) -> Result<FundingCheckResult> {
    let cast = CastCli::new();
    let mut statuses = Vec::new();

    // Check deployer wallet
    let deployer_req = FundingRequirement::new(
        wallets.deployer.address,
        "deployer",
        U256::from(DEFAULT_DEPLOYER_ETH),
    );

    let deployer_eth = check_eth_balance(&cast, wallets.deployer.address, rpc_url)
        .await
        .wrap_err("Failed to check deployer ETH balance")?;

    statuses.push(FundingStatus {
        requirement: deployer_req,
        current_eth: deployer_eth.balance,
        current_token: None,
    });

    // Check governor wallet
    let mut governor_req = FundingRequirement::new(
        wallets.governor.address,
        "governor",
        U256::from(DEFAULT_GOVERNOR_ETH),
    );

    let governor_eth = check_eth_balance(&cast, wallets.governor.address, rpc_url)
        .await
        .wrap_err("Failed to check governor ETH balance")?;

    let governor_token = if let Some(token) = cgt_address {
        governor_req = governor_req.with_token(U256::from(DEFAULT_GOVERNOR_CGT));
        let balance = check_token_balance(&cast, token, wallets.governor.address, rpc_url)
            .await
            .wrap_err("Failed to check governor CGT balance")?;
        Some(balance.balance)
    } else {
        None
    };

    statuses.push(FundingStatus {
        requirement: governor_req,
        current_eth: governor_eth.balance,
        current_token: governor_token,
    });

    let all_funded = statuses.iter().all(|s| s.is_funded());

    Ok(FundingCheckResult {
        statuses,
        all_funded,
    })
}

/// Fund ecosystem wallets from a funder wallet.
///
/// This function performs pre-flight validation and then funds any
/// wallets that are below their required balance.
///
/// # Arguments
///
/// * `wallets` - The ecosystem wallets to fund.
/// * `funder` - The funder wallet configuration.
/// * `rpc_url` - RPC endpoint URL.
///
/// # Returns
///
/// `Ok(())` if all wallets are successfully funded.
///
/// # Errors
///
/// Returns an error if:
/// - Funder wallet has insufficient balance
/// - Any transfer fails
///
/// # Example
///
/// ```rust,ignore
/// let funder = FunderConfig {
///     private_key: SecretString::new("0x...".to_string()),
///     cgt_address: None,
/// };
///
/// fund_ecosystem_wallets(&wallets, &funder, "http://localhost:8545").await?;
/// ```
pub async fn fund_ecosystem_wallets(
    wallets: &EcosystemWallets,
    funder: &FunderConfig,
    rpc_url: &str,
) -> Result<()> {
    let cast = CastCli::new();

    // Get funder address from private key
    let funder_wallet = crate::ecosystem::wallets::Wallet::from_private_key(&funder.private_key)
        .wrap_err("Failed to derive funder address from private key")?;

    let funder_address = funder_wallet.address;

    ::log::info!(
        "Checking funder wallet balance: {}",
        format!("{funder_address}").bright_yellow()
    );

    // Check current funding status
    let funding_status = check_ecosystem_funding(wallets, rpc_url, funder.cgt_address).await?;

    if funding_status.all_funded {
        ::log::info!("All ecosystem wallets are already funded");
        return Ok(());
    }

    // Calculate total ETH and CGT needed
    let mut total_eth_needed = U256::ZERO;
    let mut total_cgt_needed = U256::ZERO;

    for status in &funding_status.statuses {
        total_eth_needed += status.eth_deficit();
        if let Some(deficit) = status.token_deficit() {
            total_cgt_needed += deficit;
        }
    }

    // Check funder has enough ETH
    let funder_eth = check_eth_balance(&cast, funder_address, rpc_url)
        .await
        .wrap_err("Failed to check funder ETH balance")?;

    if funder_eth.balance < total_eth_needed {
        let needed_str = format_wei_as_eth(total_eth_needed);
        let available_str = format_wei_as_eth(funder_eth.balance);
        return Err(eyre::eyre!(
            "Funder wallet has insufficient ETH\n\n\
             Details:\n  \
             Wallet: {}\n  \
             Required: {}\n  \
             Available: {}\n\n\
             Resolution:\n  \
             1. Fund the funder wallet with at least {} more ETH\n  \
             2. Re-run the deployment command",
            funder_address,
            needed_str,
            available_str,
            format_wei_as_eth(total_eth_needed - funder_eth.balance)
        ));
    }

    // Check funder has enough CGT (if applicable)
    if let Some(cgt_address) = funder.cgt_address {
        if total_cgt_needed > U256::ZERO {
            let funder_cgt = check_token_balance(&cast, cgt_address, funder_address, rpc_url)
                .await
                .wrap_err("Failed to check funder CGT balance")?;

            if funder_cgt.balance < total_cgt_needed {
                return Err(eyre::eyre!(
                    "Funder wallet has insufficient CGT\n\n\
                     Details:\n  \
                     Wallet: {}\n  \
                     Required: {} tokens\n  \
                     Available: {} tokens\n\n\
                     Resolution:\n  \
                     1. Fund the funder wallet with more CGT tokens\n  \
                     2. Re-run the deployment command",
                    funder_address,
                    total_cgt_needed,
                    funder_cgt.balance
                ));
            }
        }
    }

    // Fund each wallet that needs it
    let private_key = funder.private_key.expose_secret();

    for status in &funding_status.statuses {
        if !status.is_funded() {
            // Fund ETH if needed
            if !status.has_sufficient_eth() {
                let deficit = status.eth_deficit();
                ::log::info!(
                    "Funding {} ({}) with {}",
                    status.requirement.name.cyan(),
                    format!("{}", status.requirement.address).bright_yellow(),
                    format_wei_as_eth(deficit)
                );

                let result = transfer_eth(
                    &cast,
                    funder_address,
                    status.requirement.address,
                    deficit,
                    private_key,
                    rpc_url,
                )
                .await
                .wrap_err_with(|| format!("Failed to fund {} with ETH", status.requirement.name))?;

                if !result.success {
                    return Err(eyre::eyre!(
                        "ETH transfer to {} failed: {}",
                        status.requirement.name,
                        result.output
                    ));
                }

                ::log::info!(
                    "  {} Funded {} with {}",
                    "✓".green(),
                    status.requirement.name,
                    format_wei_as_eth(deficit)
                );
            }

            // Fund CGT if needed
            if let Some(cgt_address) = funder.cgt_address {
                if let Some(deficit) = status.token_deficit() {
                    if deficit > U256::ZERO {
                        ::log::info!(
                            "Funding {} ({}) with {} CGT",
                            status.requirement.name.cyan(),
                            format!("{}", status.requirement.address).bright_yellow(),
                            deficit
                        );

                        let result = transfer_token(
                            &cast,
                            cgt_address,
                            funder_address,
                            status.requirement.address,
                            deficit,
                            private_key,
                            rpc_url,
                        )
                        .await
                        .wrap_err_with(|| {
                            format!("Failed to fund {} with CGT", status.requirement.name)
                        })?;

                        if !result.success {
                            return Err(eyre::eyre!(
                                "CGT transfer to {} failed: {}",
                                status.requirement.name,
                                result.output
                            ));
                        }

                        ::log::info!(
                            "  {} Funded {} with {} CGT",
                            "✓".green(),
                            status.requirement.name,
                            deficit
                        );
                    }
                }
            }
        }
    }

    ::log::info!("{} All ecosystem wallets funded", "✓".green());

    Ok(())
}

/// Format wei as ETH for display.
fn format_wei_as_eth(wei: U256) -> String {
    let eth = wei.to_string().parse::<f64>().unwrap_or(0.0) / 1e18;
    format!("{eth:.4} ETH")
}

/// Print funding status to the console.
///
/// Displays a formatted table of wallet balances with ✓ or ✗ indicators.
pub fn print_funding_status(result: &FundingCheckResult, cgt_symbol: Option<&str>) {
    ::log::info!("Checking wallet balances...");

    for status in &result.statuses {
        let eth_ok = status.has_sufficient_eth();
        let eth_icon = if eth_ok { "✓".green() } else { "✗".red() };

        let current_eth = format_wei_as_eth(status.current_eth);
        let required_eth = format_wei_as_eth(status.requirement.required_eth);

        let mut line = format!(
            "  {}: {} (required: {}) {}",
            status.requirement.name.cyan(),
            current_eth,
            required_eth,
            eth_icon
        );

        // Add token info if applicable
        if let (Some(required), Some(current)) =
            (status.requirement.required_token, status.current_token)
        {
            let token_ok = status.has_sufficient_token();
            let token_icon = if token_ok { "✓".green() } else { "✗".red() };
            let symbol = cgt_symbol.unwrap_or("CGT");

            line.push_str(&format!(
                ", {} {} (required: {} {}) {}",
                current, symbol, required, symbol, token_icon
            ));
        }

        ::log::info!("{}", line);
    }
}

/// Check funding status for chain wallets.
///
/// # Arguments
///
/// * `wallets` - The chain wallets to check.
/// * `rpc_url` - RPC endpoint URL.
/// * `cgt_address` - Optional CGT contract address (for custom base token chains).
///
/// # Returns
///
/// A `FundingCheckResult` with the status of each wallet.
///
/// # Errors
///
/// Returns an error if balance checking fails.
pub async fn check_chain_funding(
    wallets: &ChainWallets,
    rpc_url: &str,
    cgt_address: Option<Address>,
) -> Result<FundingCheckResult> {
    let cast = CastCli::new();
    let mut statuses = Vec::new();

    // Check deployer wallet (1 ETH)
    let deployer_req = FundingRequirement::new(
        wallets.deployer.address,
        "chain deployer",
        U256::from(DEFAULT_DEPLOYER_ETH),
    );

    let deployer_eth = check_eth_balance(&cast, wallets.deployer.address, rpc_url)
        .await
        .wrap_err("Failed to check chain deployer ETH balance")?;

    statuses.push(FundingStatus {
        requirement: deployer_req,
        current_eth: deployer_eth.balance,
        current_token: None,
    });

    // Check governor wallet (1 ETH + 5 CGT if applicable)
    let mut governor_req = FundingRequirement::new(
        wallets.governor.address,
        "chain governor",
        U256::from(DEFAULT_GOVERNOR_ETH),
    );

    let governor_eth = check_eth_balance(&cast, wallets.governor.address, rpc_url)
        .await
        .wrap_err("Failed to check chain governor ETH balance")?;

    let governor_token = if let Some(token) = cgt_address {
        governor_req = governor_req.with_token(U256::from(DEFAULT_GOVERNOR_CGT));
        let balance = check_token_balance(&cast, token, wallets.governor.address, rpc_url)
            .await
            .wrap_err("Failed to check chain governor CGT balance")?;
        Some(balance.balance)
    } else {
        None
    };

    statuses.push(FundingStatus {
        requirement: governor_req,
        current_eth: governor_eth.balance,
        current_token: governor_token,
    });

    // Check operator wallet (5 ETH)
    let operator_req = FundingRequirement::new(
        wallets.operator.address,
        "operator",
        U256::from(DEFAULT_OPERATOR_ETH),
    );

    let operator_eth = check_eth_balance(&cast, wallets.operator.address, rpc_url)
        .await
        .wrap_err("Failed to check operator ETH balance")?;

    statuses.push(FundingStatus {
        requirement: operator_req,
        current_eth: operator_eth.balance,
        current_token: None,
    });

    // Check prove operator wallet (5 ETH)
    let prove_req = FundingRequirement::new(
        wallets.prove_operator.address,
        "prove operator",
        U256::from(DEFAULT_OPERATOR_ETH),
    );

    let prove_eth = check_eth_balance(&cast, wallets.prove_operator.address, rpc_url)
        .await
        .wrap_err("Failed to check prove operator ETH balance")?;

    statuses.push(FundingStatus {
        requirement: prove_req,
        current_eth: prove_eth.balance,
        current_token: None,
    });

    // Check execute operator wallet (5 ETH)
    let execute_req = FundingRequirement::new(
        wallets.execute_operator.address,
        "execute operator",
        U256::from(DEFAULT_OPERATOR_ETH),
    );

    let execute_eth = check_eth_balance(&cast, wallets.execute_operator.address, rpc_url)
        .await
        .wrap_err("Failed to check execute operator ETH balance")?;

    statuses.push(FundingStatus {
        requirement: execute_req,
        current_eth: execute_eth.balance,
        current_token: None,
    });

    let all_funded = statuses.iter().all(|s| s.is_funded());

    Ok(FundingCheckResult {
        statuses,
        all_funded,
    })
}

/// Fund chain wallets from a funder wallet.
///
/// This function performs pre-flight validation and then funds any
/// chain wallets that are below their required balance.
///
/// # Arguments
///
/// * `wallets` - The chain wallets to fund.
/// * `funder` - The funder wallet configuration.
/// * `rpc_url` - RPC endpoint URL.
///
/// # Returns
///
/// `Ok(())` if all wallets are successfully funded.
///
/// # Errors
///
/// Returns an error if:
/// - Funder wallet has insufficient balance
/// - Any transfer fails
///
/// # Example
///
/// ```rust,ignore
/// let funder = FunderConfig {
///     private_key: SecretString::new("0x...".to_string()),
///     cgt_address: None,
/// };
///
/// fund_chain_wallets(&wallets, &funder, "http://localhost:8545").await?;
/// ```
pub async fn fund_chain_wallets(
    wallets: &ChainWallets,
    funder: &FunderConfig,
    rpc_url: &str,
) -> Result<()> {
    let cast = CastCli::new();

    // Get funder address from private key
    let funder_wallet = crate::ecosystem::wallets::Wallet::from_private_key(&funder.private_key)
        .wrap_err("Failed to derive funder address from private key")?;

    let funder_address = funder_wallet.address;

    ::log::info!(
        "Checking funder wallet balance: {}",
        format!("{funder_address}").bright_yellow()
    );

    // Check current funding status
    let funding_status = check_chain_funding(wallets, rpc_url, funder.cgt_address).await?;

    if funding_status.all_funded {
        ::log::info!("All chain wallets are already funded");
        return Ok(());
    }

    // Calculate total ETH and CGT needed
    let mut total_eth_needed = U256::ZERO;
    let mut total_cgt_needed = U256::ZERO;

    for status in &funding_status.statuses {
        total_eth_needed += status.eth_deficit();
        if let Some(deficit) = status.token_deficit() {
            total_cgt_needed += deficit;
        }
    }

    // Check funder has enough ETH
    let funder_eth = check_eth_balance(&cast, funder_address, rpc_url)
        .await
        .wrap_err("Failed to check funder ETH balance")?;

    if funder_eth.balance < total_eth_needed {
        let needed_str = format_wei_as_eth(total_eth_needed);
        let available_str = format_wei_as_eth(funder_eth.balance);
        return Err(eyre::eyre!(
            "Funder wallet has insufficient ETH for chain wallets\n\n\
             Details:\n  \
             Wallet: {}\n  \
             Required: {}\n  \
             Available: {}\n\n\
             Resolution:\n  \
             1. Fund the funder wallet with at least {} more ETH\n  \
             2. Re-run the deployment command",
            funder_address,
            needed_str,
            available_str,
            format_wei_as_eth(total_eth_needed - funder_eth.balance)
        ));
    }

    // Check funder has enough CGT (if applicable)
    if let Some(cgt_address) = funder.cgt_address {
        if total_cgt_needed > U256::ZERO {
            let funder_cgt = check_token_balance(&cast, cgt_address, funder_address, rpc_url)
                .await
                .wrap_err("Failed to check funder CGT balance")?;

            if funder_cgt.balance < total_cgt_needed {
                return Err(eyre::eyre!(
                    "Funder wallet has insufficient CGT for chain wallets\n\n\
                     Details:\n  \
                     Wallet: {}\n  \
                     Required: {} tokens\n  \
                     Available: {} tokens\n\n\
                     Resolution:\n  \
                     1. Fund the funder wallet with more CGT tokens\n  \
                     2. Re-run the deployment command",
                    funder_address,
                    total_cgt_needed,
                    funder_cgt.balance
                ));
            }
        }
    }

    // Fund each wallet that needs it
    let private_key = funder.private_key.expose_secret();

    for status in &funding_status.statuses {
        if !status.is_funded() {
            // Fund ETH if needed
            if !status.has_sufficient_eth() {
                let deficit = status.eth_deficit();
                ::log::info!(
                    "Funding {} ({}) with {}",
                    status.requirement.name.cyan(),
                    format!("{}", status.requirement.address).bright_yellow(),
                    format_wei_as_eth(deficit)
                );

                let result = transfer_eth(
                    &cast,
                    funder_address,
                    status.requirement.address,
                    deficit,
                    private_key,
                    rpc_url,
                )
                .await
                .wrap_err_with(|| format!("Failed to fund {} with ETH", status.requirement.name))?;

                if !result.success {
                    return Err(eyre::eyre!(
                        "ETH transfer to {} failed: {}",
                        status.requirement.name,
                        result.output
                    ));
                }

                ::log::info!(
                    "  {} Funded {} with {}",
                    "✓".green(),
                    status.requirement.name,
                    format_wei_as_eth(deficit)
                );
            }

            // Fund CGT if needed
            if let Some(cgt_address) = funder.cgt_address {
                if let Some(deficit) = status.token_deficit() {
                    if deficit > U256::ZERO {
                        ::log::info!(
                            "Funding {} ({}) with {} CGT",
                            status.requirement.name.cyan(),
                            format!("{}", status.requirement.address).bright_yellow(),
                            deficit
                        );

                        let result = transfer_token(
                            &cast,
                            cgt_address,
                            funder_address,
                            status.requirement.address,
                            deficit,
                            private_key,
                            rpc_url,
                        )
                        .await
                        .wrap_err_with(|| {
                            format!("Failed to fund {} with CGT", status.requirement.name)
                        })?;

                        if !result.success {
                            return Err(eyre::eyre!(
                                "CGT transfer to {} failed: {}",
                                status.requirement.name,
                                result.output
                            ));
                        }

                        ::log::info!(
                            "  {} Funded {} with {} CGT",
                            "✓".green(),
                            status.requirement.name,
                            deficit
                        );
                    }
                }
            }
        }
    }

    ::log::info!("{} All chain wallets funded", "✓".green());

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_wei_as_eth() {
        let wei = U256::from(1_500_000_000_000_000_000u128);
        assert_eq!(format_wei_as_eth(wei), "1.5000 ETH");
    }

    #[test]
    fn test_funding_check_result_underfunded() {
        let req = FundingRequirement::new(Address::ZERO, "test", U256::from(DEFAULT_DEPLOYER_ETH));

        let status = FundingStatus {
            requirement: req,
            current_eth: U256::from(500_000_000_000_000_000u128),
            current_token: None,
        };

        let result = FundingCheckResult {
            statuses: vec![status],
            all_funded: false,
        };

        assert_eq!(result.underfunded_wallets().len(), 1);
    }
}
