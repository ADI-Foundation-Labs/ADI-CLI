//! Wallet funding SDK for ADI ecosystem and chain operations.
//!
//! This crate provides wallet funding functionality:
//! - Balance checking (ETH + ERC20 tokens)
//! - Transfer planning with gas estimation
//! - Pre-flight validation
//! - Batch transfer execution with progress events
//!
//! # Overview
//!
//! The `adi-funding` crate handles funding wallets with ETH and optional
//! custom gas tokens (ERC20). It follows a plan-then-execute pattern:
//!
//! 1. Create a [`FundingPlanBuilder`] with target wallets
//! 2. Build the plan (checks balances, estimates gas, validates)
//! 3. Execute the plan with a [`FundingExecutor`]
//!
//! # Example
//!
//! ```rust,ignore
//! use adi_funding::{FundingConfig, FundingExecutor, FundingPlanBuilder};
//! use adi_types::Wallets;
//! use secrecy::SecretString;
//!
//! # async fn example() -> adi_funding::Result<()> {
//! // Load wallets from state
//! let wallets: Wallets = /* load from state */;
//! let funder_key = SecretString::from("0x...");
//!
//! // Create config
//! let config = FundingConfig::new("https://rpc.example.com");
//!
//! // Create executor
//! let executor = FundingExecutor::new(&config.rpc_url, &funder_key)?;
//!
//! // Build funding plan
//! let plan = FundingPlanBuilder::new(executor.provider(), &config, executor.funder_address())
//!     .with_wallets(&wallets)
//!     .build()
//!     .await?;
//!
//! println!("Plan: {} transfers, {} ETH required", plan.transfer_count(), plan.total_eth_required);
//!
//! // Execute plan
//! let result = executor.execute(&plan).await?;
//!
//! if result.is_success() {
//!     println!("All {} transfers successful!", result.successful);
//! }
//! # Ok(())
//! # }
//! ```
//!
//! # Progress Events
//!
//! Implement [`FundingEventHandler`] to receive progress updates:
//!
//! ```rust,ignore
//! use adi_funding::{FundingEvent, FundingEventHandler};
//!
//! struct MyHandler;
//!
//! #[async_trait::async_trait]
//! impl FundingEventHandler for MyHandler {
//!     async fn on_event(&self, event: FundingEvent) {
//!         match event {
//!             FundingEvent::TransferStarted { index, total, .. } => {
//!                 println!("Transfer {}/{}", index + 1, total);
//!             }
//!             _ => {}
//!         }
//!     }
//! }
//! ```
//!
//! # Abort on Failure
//!
//! The executor uses fail-fast behavior: if any transfer fails, execution
//! stops immediately and returns an error. This is intentional for safety
//! when dealing with funds.

#![deny(missing_docs)]
#![deny(unsafe_code)]

mod balance;
mod config;
mod error;
mod events;
mod executor;
mod plan;
mod provider;
mod signer;
mod transfer;

// Public re-exports
pub use balance::{
    get_eth_balance, get_token_balance, get_token_decimals, get_token_symbol, get_wallet_balance,
    WalletBalance,
};
pub use config::{DefaultAmounts, FundingConfig, FundingTarget, WalletRole};
pub use error::{FundingError, Result};
pub use events::{FundingEvent, FundingEventHandler, LoggingEventHandler, NoOpEventHandler};
pub use executor::{FundingExecutor, FundingResult};
pub use plan::{FundingPlan, FundingPlanBuilder};
pub use provider::FundingProvider;
pub use signer::{create_signer, signer_address};
pub use transfer::{Transfer, TransferType};
