//! Contract registry for verification.
//!
//! Maps contract types to their source file paths within zksync-era contracts.

mod builders;
mod mappings;
mod target;
mod types;

pub use mappings::ContractRegistry;
pub use target::VerificationTarget;
pub use types::{
    ChainAdminVerificationInfo, ContractType, ProxyVerificationInfo, VerifierVerificationInfo,
};
