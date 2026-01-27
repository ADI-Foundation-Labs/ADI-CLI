//! Chain wallet types.
//!
//! This module provides wallet structures for managing keypairs used in
//! chain deployment and operations.

use serde::{Deserialize, Serialize};

use crate::ecosystem::wallets::Wallet;

/// Wallets used for chain-level operations.
///
/// These wallets are used during chain deployment and ongoing operations:
/// - `deployer`: Deploys chain contracts to the settlement layer
/// - `governor`: Manages chain governance and upgrades
/// - `operator`: Submits batches to L1
/// - `prove_operator`: Submits proofs to L1
/// - `execute_operator`: Executes batches on L1
///
/// # Funding Requirements
///
/// | Role             | ETH Required | CGT Required* |
/// |------------------|--------------|---------------|
/// | deployer         | 1 ETH        | -             |
/// | governor         | 1 ETH        | 5 CGT         |
/// | operator         | 5 ETH        | -             |
/// | prove_operator   | 5 ETH        | -             |
/// | execute_operator | 5 ETH        | -             |
///
/// *CGT (Custom Gas Token) only required when chain uses custom base token.
// Note: Currently unused as commands are implemented in later phases (US1-US6)
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainWallets {
    /// The deployer wallet for contract deployment.
    pub deployer: Wallet,

    /// The governor wallet for governance operations.
    pub governor: Wallet,

    /// The operator wallet for batch submission.
    pub operator: Wallet,

    /// The prove operator wallet for proof submission.
    pub prove_operator: Wallet,

    /// The execute operator wallet for batch execution.
    pub execute_operator: Wallet,
    // TODO: Include additional wallets for database storage (not currently used by CLI):
    // - blob_operator: Wallet         // Blob operator wallet
    // - fee_account: Wallet           // Fee account wallet
    // - token_multiplier_setter: Wallet // Token multiplier setter
}

#[allow(dead_code)]
impl ChainWallets {
    /// Creates new chain wallets from individual wallets.
    pub fn new(
        deployer: Wallet,
        governor: Wallet,
        operator: Wallet,
        prove_operator: Wallet,
        execute_operator: Wallet,
    ) -> Self {
        Self {
            deployer,
            governor,
            operator,
            prove_operator,
            execute_operator,
        }
    }

    /// Returns an iterator over all wallets.
    ///
    /// Useful for funding operations that need to process all wallets.
    pub fn all(&self) -> Vec<&Wallet> {
        vec![
            &self.deployer,
            &self.governor,
            &self.operator,
            &self.prove_operator,
            &self.execute_operator,
        ]
    }

    /// Returns wallet roles and their required ETH amounts.
    ///
    /// Returns a vector of (role_name, wallet, eth_wei_required) tuples.
    pub fn funding_requirements(&self) -> Vec<(&'static str, &Wallet, u128)> {
        const ONE_ETH: u128 = 1_000_000_000_000_000_000;
        const FIVE_ETH: u128 = 5_000_000_000_000_000_000;

        vec![
            ("deployer", &self.deployer, ONE_ETH),
            ("governor", &self.governor, ONE_ETH),
            ("operator", &self.operator, FIVE_ETH),
            ("prove_operator", &self.prove_operator, FIVE_ETH),
            ("execute_operator", &self.execute_operator, FIVE_ETH),
        ]
    }
}
