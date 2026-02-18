//! Smart contract verification on block explorers.
//!
//! This module provides functionality for verifying deployed smart contracts
//! on block explorers like Etherscan and Blockscout.
//!
//! # Overview
//!
//! The verification module handles:
//! - Checking existing verification status on explorers
//! - Building verification targets from contract state
//! - Contract type to source file mapping
//!
//! # Example
//!
//! ```rust,no_run
//! use adi_ecosystem::verification::{
//!     ContractRegistry, ExplorerClient, ExplorerConfig, ExplorerType,
//! };
//! use adi_types::NoopLogger;
//! use std::sync::Arc;
//! use url::Url;
//!
//! // Create explorer client
//! let config = ExplorerConfig::new(
//!     ExplorerType::Etherscan,
//!     Url::parse("https://api.etherscan.io/api").unwrap(),
//!     Some("YOUR_API_KEY".to_string()),
//!     1, // mainnet
//! );
//! let client = ExplorerClient::new(config, Arc::new(NoopLogger));
//! ```

mod error;
mod explorer;
mod registry;
mod types;

// Re-export public types
pub use error::VerificationError;
pub use explorer::{ExplorerClient, ExplorerConfig};
pub use registry::{ContractRegistry, ContractType, VerificationTarget};
pub use types::{
    ContractVerificationStatus, ExplorerType, VerificationOutcome, VerificationResult,
    VerificationStatus, VerificationStatusSummary, VerificationSummary,
};
