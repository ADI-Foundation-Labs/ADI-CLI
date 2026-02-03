//! Path constants for state file locations.

/// Ecosystem metadata file name.
pub const ECOSYSTEM_METADATA: &str = "ZkStack.yaml";

/// Ecosystem configs directory.
pub const CONFIGS_DIR: &str = "configs";

/// Wallets config file name.
pub const WALLETS_FILE: &str = "wallets.yaml";

/// Contracts config file name (created after deployment).
pub const CONTRACTS_FILE: &str = "contracts.yaml";

/// Initial deployments config file name.
pub const INITIAL_DEPLOYMENTS_FILE: &str = "initial_deployments.yaml";

/// ERC20 deployments config file name.
pub const ERC20_DEPLOYMENTS_FILE: &str = "erc20_deployments.yaml";

/// Apps config file name.
pub const APPS_FILE: &str = "apps.yaml";

/// Chains directory name.
pub const CHAINS_DIR: &str = "chains";

/// Chain metadata file name (same as ecosystem).
pub const CHAIN_METADATA: &str = "ZkStack.yaml";

/// Relative path to ecosystem wallets from ecosystem root.
#[must_use]
pub fn ecosystem_wallets_path() -> String {
    format!("{CONFIGS_DIR}/{WALLETS_FILE}")
}

/// Relative path to ecosystem contracts from ecosystem root.
#[must_use]
pub fn ecosystem_contracts_path() -> String {
    format!("{CONFIGS_DIR}/{CONTRACTS_FILE}")
}

/// Relative path to initial deployments from ecosystem root.
#[must_use]
pub fn initial_deployments_path() -> String {
    format!("{CONFIGS_DIR}/{INITIAL_DEPLOYMENTS_FILE}")
}

/// Relative path to ERC20 deployments from ecosystem root.
#[must_use]
pub fn erc20_deployments_path() -> String {
    format!("{CONFIGS_DIR}/{ERC20_DEPLOYMENTS_FILE}")
}

/// Relative path to apps config from ecosystem root.
#[must_use]
pub fn apps_path() -> String {
    format!("{CONFIGS_DIR}/{APPS_FILE}")
}

/// Relative path to chain directory from ecosystem root.
#[must_use]
pub fn chain_dir(chain_name: &str) -> String {
    format!("{CHAINS_DIR}/{chain_name}")
}

/// Relative path to chain metadata from ecosystem root.
#[must_use]
pub fn chain_metadata_path(chain_name: &str) -> String {
    format!("{CHAINS_DIR}/{chain_name}/{CHAIN_METADATA}")
}

/// Relative path to chain wallets from ecosystem root.
#[must_use]
pub fn chain_wallets_path(chain_name: &str) -> String {
    format!("{CHAINS_DIR}/{chain_name}/{CONFIGS_DIR}/{WALLETS_FILE}")
}

/// Relative path to chain contracts from ecosystem root.
#[must_use]
pub fn chain_contracts_path(chain_name: &str) -> String {
    format!("{CHAINS_DIR}/{chain_name}/{CONFIGS_DIR}/{CONTRACTS_FILE}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ecosystem_paths() {
        assert_eq!(ecosystem_wallets_path(), "configs/wallets.yaml");
        assert_eq!(ecosystem_contracts_path(), "configs/contracts.yaml");
        assert_eq!(
            initial_deployments_path(),
            "configs/initial_deployments.yaml"
        );
        assert_eq!(erc20_deployments_path(), "configs/erc20_deployments.yaml");
        assert_eq!(apps_path(), "configs/apps.yaml");
    }

    #[test]
    fn test_chain_paths() {
        assert_eq!(chain_dir("mychain"), "chains/mychain");
        assert_eq!(
            chain_metadata_path("mychain"),
            "chains/mychain/ZkStack.yaml"
        );
        assert_eq!(
            chain_wallets_path("mychain"),
            "chains/mychain/configs/wallets.yaml"
        );
        assert_eq!(
            chain_contracts_path("mychain"),
            "chains/mychain/configs/contracts.yaml"
        );
    }
}
