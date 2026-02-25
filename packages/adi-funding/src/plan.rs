//! Funding plan calculation and validation.

use crate::balance::{get_token_balance, get_token_decimals, get_token_symbol, get_wallet_balance};
use crate::config::{FundingConfig, FundingTarget, FundingTargetStatus, WalletRole, WalletSource};
use crate::error::{FundingError, Result};
use crate::provider::FundingProvider;
use crate::transfer::{estimate_eth_transfer_gas, estimate_token_transfer_gas, Transfer};
use adi_types::Wallets;
use alloy_primitives::{Address, U256};

/// A complete funding plan ready for execution.
#[derive(Clone, Debug)]
pub struct FundingPlan {
    /// Funder wallet address.
    pub funder: Address,
    /// Funder's current ETH balance.
    pub funder_eth_balance: U256,
    /// Funder's current token balance (if applicable).
    pub funder_token_balance: Option<U256>,
    /// List of required transfers.
    pub transfers: Vec<Transfer>,
    /// Total ETH required (transfers + gas).
    pub total_eth_required: U256,
    /// Total token required.
    pub total_token_required: U256,
    /// Estimated total gas cost.
    pub total_gas_cost: U256,
    /// Current gas price used for estimation.
    pub gas_price: u128,
}

impl FundingPlan {
    /// Check if the funder has sufficient balance.
    pub fn is_valid(&self) -> bool {
        self.funder_eth_balance >= self.total_eth_required
            && self
                .funder_token_balance
                .is_none_or(|bal| bal >= self.total_token_required)
    }

    /// Get number of transfers.
    pub fn transfer_count(&self) -> usize {
        self.transfers.len()
    }

    /// Get ETH transfers only.
    pub fn eth_transfers(&self) -> impl Iterator<Item = &Transfer> {
        self.transfers.iter().filter(|t| t.is_eth())
    }

    /// Get token transfers only.
    pub fn token_transfers(&self) -> impl Iterator<Item = &Transfer> {
        self.transfers.iter().filter(|t| !t.is_eth())
    }

    /// Get total ETH to transfer (excluding gas).
    pub fn total_eth_transfers(&self) -> U256 {
        self.total_eth_required - self.total_gas_cost
    }
}

/// Builder for creating funding plans.
pub struct FundingPlanBuilder<'a> {
    provider: &'a FundingProvider,
    config: &'a FundingConfig,
    funder: Address,
    targets: Vec<FundingTarget>,
}

impl<'a> FundingPlanBuilder<'a> {
    /// Create a new plan builder.
    pub fn new(provider: &'a FundingProvider, config: &'a FundingConfig, funder: Address) -> Self {
        Self {
            provider,
            config,
            funder,
            targets: Vec::new(),
        }
    }

    /// Add funding targets from Wallets struct using default amounts.
    ///
    /// This automatically adds all wallets present in the Wallets struct
    /// with their corresponding default funding amounts.
    pub fn with_wallets(mut self, wallets: &Wallets, source: WalletSource) -> Self {
        let amounts = &self.config.default_amounts;

        // Add deployer if present
        if let Some(w) = &wallets.deployer {
            self.targets.push(FundingTarget::new(
                WalletRole::Deployer,
                source,
                w.address,
                amounts.deployer_eth,
            ));
        }

        // Add governor if present (token amount added in build after fetching decimals)
        if let Some(w) = &wallets.governor {
            self.targets.push(FundingTarget::new(
                WalletRole::Governor,
                source,
                w.address,
                amounts.governor_eth,
            ));
        }

        // Add operator if present
        if let Some(w) = &wallets.operator {
            self.targets.push(FundingTarget::new(
                WalletRole::Operator,
                source,
                w.address,
                amounts.operator_eth,
            ));
        }

        // Add blob operator if present
        if let Some(w) = &wallets.blob_operator {
            self.targets.push(FundingTarget::new(
                WalletRole::BlobOperator,
                source,
                w.address,
                amounts.blob_operator_eth,
            ));
        }

        // Add prove operator if present
        if let Some(w) = &wallets.prove_operator {
            self.targets.push(FundingTarget::new(
                WalletRole::ProveOperator,
                source,
                w.address,
                amounts.prove_operator_eth,
            ));
        }

        // Add execute operator if present
        if let Some(w) = &wallets.execute_operator {
            self.targets.push(FundingTarget::new(
                WalletRole::ExecuteOperator,
                source,
                w.address,
                amounts.execute_operator_eth,
            ));
        }

        // Add fee account if present and amount > 0
        if let Some(w) = &wallets.fee_account {
            if !amounts.fee_account_eth.is_zero() {
                self.targets.push(FundingTarget::new(
                    WalletRole::FeeAccount,
                    source,
                    w.address,
                    amounts.fee_account_eth,
                ));
            }
        }

        // Add token multiplier setter if present and amount > 0
        if let Some(w) = &wallets.token_multiplier_setter {
            if !amounts.token_multiplier_setter_eth.is_zero() {
                self.targets.push(FundingTarget::new(
                    WalletRole::TokenMultiplierSetter,
                    source,
                    w.address,
                    amounts.token_multiplier_setter_eth,
                ));
            }
        }

        self
    }

    /// Add funding targets for ecosystem wallets only.
    ///
    /// Ecosystem deployment requires funding only:
    /// - `deployer` - for contract deployment (ETH)
    /// - `governor` - for governance operations (ETH + CGT)
    ///
    /// Other wallet roles in the Wallets struct are ignored.
    pub fn with_ecosystem_wallets(mut self, wallets: &Wallets) -> Self {
        let amounts = &self.config.default_amounts;

        if let Some(w) = &wallets.deployer {
            self.targets.push(FundingTarget::new(
                WalletRole::Deployer,
                WalletSource::Ecosystem,
                w.address,
                amounts.deployer_eth,
            ));
        }

        if let Some(w) = &wallets.governor {
            self.targets.push(FundingTarget::new(
                WalletRole::Governor,
                WalletSource::Ecosystem,
                w.address,
                amounts.governor_eth,
            ));
        }

        self
    }

    /// Add funding targets for chain wallets only.
    ///
    /// Chain deployment requires funding only:
    /// - `governor` - for chain governance (ETH + CGT)
    /// - `operator` - for chain operations (ETH)
    /// - `prove_operator` - for proof submission (ETH)
    /// - `execute_operator` - for transaction execution (ETH)
    ///
    /// Other wallet roles (deployer, blob_operator, fee_account,
    /// token_multiplier_setter) are NOT funded.
    pub fn with_chain_wallets(mut self, wallets: &Wallets) -> Self {
        let amounts = &self.config.default_amounts;

        if let Some(w) = &wallets.governor {
            self.targets.push(FundingTarget::new(
                WalletRole::Governor,
                WalletSource::Chain,
                w.address,
                amounts.governor_eth,
            ));
        }

        if let Some(w) = &wallets.operator {
            self.targets.push(FundingTarget::new(
                WalletRole::Operator,
                WalletSource::Chain,
                w.address,
                amounts.operator_eth,
            ));
        }

        if let Some(w) = &wallets.prove_operator {
            self.targets.push(FundingTarget::new(
                WalletRole::ProveOperator,
                WalletSource::Chain,
                w.address,
                amounts.prove_operator_eth,
            ));
        }

        if let Some(w) = &wallets.execute_operator {
            self.targets.push(FundingTarget::new(
                WalletRole::ExecuteOperator,
                WalletSource::Chain,
                w.address,
                amounts.execute_operator_eth,
            ));
        }

        self
    }

    /// Add a custom funding target.
    pub fn with_target(mut self, target: FundingTarget) -> Self {
        self.targets.push(target);
        self
    }

    /// Add multiple custom funding targets.
    pub fn with_targets(mut self, targets: impl IntoIterator<Item = FundingTarget>) -> Self {
        self.targets.extend(targets);
        self
    }

    /// Build the funding plan by checking balances and calculating transfers.
    ///
    /// This performs the following:
    /// 1. Gets the current gas price
    /// 2. Checks funder's ETH and token balances
    /// 3. For each target, checks current balance and calculates needed amount
    /// 4. Estimates gas for each transfer
    /// 5. Validates funder has sufficient funds
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - RPC requests fail
    /// - Funder has insufficient balance
    /// - No funding is required (all wallets already funded)
    pub async fn build(mut self) -> Result<FundingPlan> {
        if self.targets.is_empty() {
            return Err(FundingError::NoFundingRequired);
        }

        let gas_price = self.provider.get_gas_price().await?;
        let adjusted_gas_price = (gas_price * u128::from(self.config.gas_price_multiplier)) / 100;

        // Get funder balances
        let funder_eth = self.provider.get_eth_balance(self.funder).await?;
        let funder_token = match self.config.token_address {
            Some(token) => Some(get_token_balance(self.provider, token, self.funder).await?),
            None => None,
        };

        // Get token symbol and decimals if token is configured
        let (token_symbol, token_decimals) = match self.config.token_address {
            Some(token) => {
                let symbol = if let Some(sym) = &self.config.token_symbol {
                    sym.clone()
                } else {
                    get_token_symbol(self.provider, token)
                        .await
                        .unwrap_or_else(|_| "TOKEN".to_string())
                };
                let decimals = get_token_decimals(self.provider, token).await?;
                (symbol, Some(decimals))
            }
            None => (String::new(), None),
        };

        // Add token amount to governor targets if token is configured
        if let Some(decimals) = token_decimals {
            let cgt_amount = self.config.default_amounts.governor_cgt_amount(decimals);
            for target in &mut self.targets {
                if target.role == WalletRole::Governor && target.token_amount.is_none() {
                    target.token_amount = Some(cgt_amount);
                }
            }
        }

        let mut transfers = Vec::new();
        let mut total_eth_transfer = U256::ZERO;
        let mut total_token_transfer = U256::ZERO;
        let mut total_gas = U256::ZERO;

        // Calculate required transfers for each target
        for target in &self.targets {
            let current_balance =
                get_wallet_balance(self.provider, target.address, self.config.token_address)
                    .await?;

            // ETH funding needed?
            if current_balance.eth_balance < target.eth_amount {
                let eth_needed = target.eth_amount - current_balance.eth_balance;
                let gas_estimate = estimate_eth_transfer_gas(
                    self.provider,
                    self.funder,
                    target.address,
                    eth_needed,
                )
                .await?;

                transfers.push(Transfer::eth(
                    target.role,
                    self.funder,
                    target.address,
                    eth_needed,
                    gas_estimate,
                ));

                total_eth_transfer += eth_needed;
                total_gas += U256::from(gas_estimate) * U256::from(adjusted_gas_price);
            }

            // Token funding needed?
            if let (Some(token_amount), Some(token_addr)) =
                (target.token_amount, self.config.token_address)
            {
                let current_token = current_balance.token_balance.unwrap_or(U256::ZERO);
                if current_token < token_amount {
                    let token_needed = token_amount - current_token;
                    let gas_estimate = estimate_token_transfer_gas(
                        self.provider,
                        self.funder,
                        target.address,
                        token_addr,
                        token_needed,
                    )
                    .await?;

                    transfers.push(Transfer::token(
                        target.role,
                        self.funder,
                        target.address,
                        token_addr,
                        token_needed,
                        token_symbol.clone(),
                        gas_estimate,
                    ));

                    total_token_transfer += token_needed;
                    total_gas += U256::from(gas_estimate) * U256::from(adjusted_gas_price);
                }
            }
        }

        if transfers.is_empty() {
            return Err(FundingError::NoFundingRequired);
        }

        let total_eth_required = total_eth_transfer + total_gas;

        // Validate funder has enough ETH
        if funder_eth < total_eth_required {
            return Err(FundingError::InsufficientEthBalance {
                have: funder_eth,
                need: total_eth_required,
                gas_estimate: total_gas,
            });
        }

        // Validate funder has enough tokens
        if let Some(funder_tok) = funder_token {
            if self.config.token_address.is_some() && funder_tok < total_token_transfer {
                return Err(FundingError::InsufficientTokenBalance {
                    symbol: token_symbol,
                    have: funder_tok,
                    need: total_token_transfer,
                });
            }
        }

        Ok(FundingPlan {
            funder: self.funder,
            funder_eth_balance: funder_eth,
            funder_token_balance: funder_token,
            transfers,
            total_eth_required,
            total_token_required: total_token_transfer,
            total_gas_cost: total_gas,
            gas_price: adjusted_gas_price,
        })
    }
}

/// Build funding target statuses for display.
///
/// Returns all targets with their current balances and funding status,
/// suitable for displaying a funding plan summary before execution.
/// This matches the pattern used by `AnvilFunder::get_funding_targets()`.
pub async fn build_funding_target_statuses(
    provider: &FundingProvider,
    config: &FundingConfig,
    ecosystem_wallets: &Wallets,
    chain_wallets: &Wallets,
) -> Result<Vec<FundingTargetStatus>> {
    let amounts = &config.default_amounts;
    let mut targets: Vec<FundingTarget> = Vec::new();

    // Add ecosystem wallets
    if let Some(w) = &ecosystem_wallets.deployer {
        targets.push(FundingTarget::new(
            WalletRole::Deployer,
            WalletSource::Ecosystem,
            w.address,
            amounts.deployer_eth,
        ));
    }
    if let Some(w) = &ecosystem_wallets.governor {
        targets.push(FundingTarget::new(
            WalletRole::Governor,
            WalletSource::Ecosystem,
            w.address,
            amounts.governor_eth,
        ));
    }

    // Add chain wallets
    if let Some(w) = &chain_wallets.governor {
        targets.push(FundingTarget::new(
            WalletRole::Governor,
            WalletSource::Chain,
            w.address,
            amounts.governor_eth,
        ));
    }
    if let Some(w) = &chain_wallets.operator {
        targets.push(FundingTarget::new(
            WalletRole::Operator,
            WalletSource::Chain,
            w.address,
            amounts.operator_eth,
        ));
    }
    if let Some(w) = &chain_wallets.prove_operator {
        targets.push(FundingTarget::new(
            WalletRole::ProveOperator,
            WalletSource::Chain,
            w.address,
            amounts.prove_operator_eth,
        ));
    }
    if let Some(w) = &chain_wallets.execute_operator {
        targets.push(FundingTarget::new(
            WalletRole::ExecuteOperator,
            WalletSource::Chain,
            w.address,
            amounts.execute_operator_eth,
        ));
    }

    // Get token decimals for CGT amount calculation
    let token_decimals = match config.token_address {
        Some(token) => Some(get_token_decimals(provider, token).await?),
        None => None,
    };

    // Add token amounts to governor targets
    if let Some(decimals) = token_decimals {
        let cgt_amount = amounts.governor_cgt_amount(decimals);
        for target in &mut targets {
            if target.role == WalletRole::Governor && target.token_amount.is_none() {
                target.token_amount = Some(cgt_amount);
            }
        }
    }

    // Query balances and build statuses
    let mut statuses = Vec::with_capacity(targets.len());
    for target in &targets {
        let balance = get_wallet_balance(provider, target.address, config.token_address).await?;

        let needs_eth = balance.eth_balance < target.eth_amount;
        let needs_token = target
            .token_amount
            .map(|req| balance.token_balance.unwrap_or(U256::ZERO) < req)
            .unwrap_or(false);

        statuses.push(FundingTargetStatus {
            role: target.role,
            source: target.source,
            address: target.address,
            required_eth: target.eth_amount,
            required_token: target.token_amount,
            current_eth: balance.eth_balance,
            current_token: balance.token_balance,
            needs_eth_funding: needs_eth,
            needs_token_funding: needs_token,
        });
    }

    Ok(statuses)
}
