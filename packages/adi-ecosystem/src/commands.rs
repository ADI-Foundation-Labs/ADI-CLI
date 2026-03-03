//! Command argument builders for ecosystem operations.
//!
//! This module builds command-line arguments for zkstack CLI commands.
//! It does NOT know how the commands are executed (Docker, local, etc.).

use crate::config::{ChainConfig, EcosystemConfig};

/// Path to zksync-era in toolkit container.
pub const ERA_CONTRACTS_PATH: &str = "/deps/zksync-era";

/// Build arguments for `zkstack ecosystem create` command.
///
/// Returns arguments to pass to zkstack CLI (not including "zkstack" itself).
///
/// # Arguments
///
/// * `config` - Ecosystem configuration.
///
/// # Returns
///
/// Vector of command arguments.
///
/// # Example
///
/// ```rust
/// use adi_ecosystem::{EcosystemConfig, build_ecosystem_create_args};
///
/// let config = EcosystemConfig::default();
/// let args = build_ecosystem_create_args(&config);
///
/// assert!(args.contains(&"ecosystem".to_string()));
/// assert!(args.contains(&"create".to_string()));
/// ```
#[must_use]
pub fn build_ecosystem_create_args(config: &EcosystemConfig) -> Vec<String> {
    vec![
        "ecosystem".to_string(),
        "create".to_string(),
        "--zksync-os".to_string(),
        "-v".to_string(),
        "--ecosystem-name".to_string(),
        config.name.clone(),
        "--l1-network".to_string(),
        config.l1_network.to_string(),
        "--link-to-code".to_string(),
        ERA_CONTRACTS_PATH.to_string(),
        "--chain-name".to_string(),
        config.chain_name.clone(),
        "--chain-id".to_string(),
        config.chain_id.to_string(),
        "--prover-mode".to_string(),
        config.prover_mode.to_string(),
        "--wallet-creation".to_string(),
        "random".to_string(),
        "--l1-batch-commit-data-generator-mode".to_string(),
        "rollup".to_string(),
        "--base-token-address".to_string(),
        config.base_token_address.to_string(),
        "--base-token-price-nominator".to_string(),
        config.base_token_price_nominator.to_string(),
        "--base-token-price-denominator".to_string(),
        config.base_token_price_denominator.to_string(),
        "--evm-emulator".to_string(),
        config.evm_emulator.to_string(),
        "--start-containers".to_string(),
        "false".to_string(),
    ]
}

/// Build arguments for `zkstack chain create` command.
///
/// Returns arguments to pass to zkstack CLI (not including "zkstack" itself).
/// This is used to add a new chain to an existing ecosystem.
///
/// # Arguments
///
/// * `config` - Chain configuration.
///
/// # Returns
///
/// Vector of command arguments.
///
/// # Example
///
/// ```rust
/// use adi_ecosystem::{ChainConfig, ProverMode, build_chain_create_args};
///
/// let config = ChainConfig::builder()
///     .name("my_chain")
///     .chain_id(123)
///     .prover_mode(ProverMode::NoProofs)
///     .build();
/// let args = build_chain_create_args(&config);
///
/// assert!(args.contains(&"chain".to_string()));
/// assert!(args.contains(&"create".to_string()));
/// ```
#[must_use]
pub fn build_chain_create_args(config: &ChainConfig) -> Vec<String> {
    vec![
        "chain".to_string(),
        "create".to_string(),
        "--zksync-os".to_string(),
        "-v".to_string(),
        "--chain-name".to_string(),
        config.name.clone(),
        "--chain-id".to_string(),
        config.chain_id.to_string(),
        "--prover-mode".to_string(),
        config.prover_mode.to_string(),
        "--wallet-creation".to_string(),
        "random".to_string(),
        "--l1-batch-commit-data-generator-mode".to_string(),
        "rollup".to_string(),
        "--base-token-address".to_string(),
        config.base_token_address.to_string(),
        "--base-token-price-nominator".to_string(),
        config.base_token_price_nominator.to_string(),
        "--base-token-price-denominator".to_string(),
        config.base_token_price_denominator.to_string(),
        "--evm-emulator".to_string(),
        config.evm_emulator.to_string(),
        "--set-as-default".to_string(),
        "false".to_string(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{L1Network, ProverMode};
    use adi_types::ETH_TOKEN_ADDRESS;

    #[test]
    fn test_build_ecosystem_create_args() {
        let config = EcosystemConfig {
            name: "test_ecosystem".to_string(),
            l1_network: L1Network::Sepolia,
            chain_name: "test_chain".to_string(),
            chain_id: 123,
            prover_mode: ProverMode::NoProofs,
            base_token_address: ETH_TOKEN_ADDRESS,
            base_token_price_nominator: 1,
            base_token_price_denominator: 1,
            evm_emulator: false,
            l3: false,
            rpc_url: None,
        };

        let args = build_ecosystem_create_args(&config);

        assert!(args.contains(&"ecosystem".to_string()));
        assert!(args.contains(&"create".to_string()));
        assert!(args.contains(&"--ecosystem-name".to_string()));
        assert!(args.contains(&"test_ecosystem".to_string()));
        assert!(args.contains(&"--l1-network".to_string()));
        assert!(args.contains(&"sepolia".to_string()));
        assert!(args.contains(&"--chain-name".to_string()));
        assert!(args.contains(&"test_chain".to_string()));
        assert!(args.contains(&"--chain-id".to_string()));
        assert!(args.contains(&"123".to_string()));
        assert!(args.contains(&"--prover-mode".to_string()));
        assert!(args.contains(&"no-proofs".to_string()));
        assert!(args.contains(&"--start-containers".to_string()));
        assert!(args.contains(&"false".to_string()));
    }

    #[test]
    fn test_build_chain_create_args() {
        let config = ChainConfig {
            name: "new_chain".to_string(),
            chain_id: 456,
            prover_mode: ProverMode::Gpu,
            base_token_address: ETH_TOKEN_ADDRESS,
            base_token_price_nominator: 1,
            base_token_price_denominator: 1,
            evm_emulator: true,
        };

        let args = build_chain_create_args(&config);

        assert!(args.contains(&"chain".to_string()));
        assert!(args.contains(&"create".to_string()));
        assert!(args.contains(&"--chain-name".to_string()));
        assert!(args.contains(&"new_chain".to_string()));
        assert!(args.contains(&"--chain-id".to_string()));
        assert!(args.contains(&"456".to_string()));
        assert!(args.contains(&"--prover-mode".to_string()));
        assert!(args.contains(&"gpu".to_string()));
        assert!(args.contains(&"--evm-emulator".to_string()));
        assert!(args.contains(&"true".to_string()));
        assert!(args.contains(&"--wallet-creation".to_string()));
        assert!(args.contains(&"random".to_string()));
    }
}
