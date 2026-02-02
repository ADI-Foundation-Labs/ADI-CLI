//! Verification functions for ecosystem operations.
//!
//! This module verifies that ecosystem operations completed successfully
//! by checking for expected files and configurations.

use crate::config::EcosystemConfig;
use crate::error::{EcosystemError, Result};
use std::path::Path;

/// Verify ecosystem was created successfully.
///
/// Checks for the presence of required files that should exist
/// after a successful `zkstack ecosystem create` command.
///
/// # Arguments
///
/// * `state_dir` - The state directory where ecosystem was created.
/// * `config` - The ecosystem configuration used for creation.
///
/// # Errors
///
/// Returns `EcosystemError::MissingFile` if any required file is missing.
///
/// # Example
///
/// ```rust,no_run
/// use adi_ecosystem::{EcosystemConfig, verify_ecosystem_created};
/// use std::path::Path;
///
/// let config = EcosystemConfig::default();
/// let state_dir = Path::new("/home/user/.adi_cli/state");
///
/// verify_ecosystem_created(state_dir, &config)?;
/// # Ok::<(), adi_ecosystem::EcosystemError>(())
/// ```
pub fn verify_ecosystem_created(state_dir: &Path, config: &EcosystemConfig) -> Result<()> {
    let ecosystem_dir = state_dir.join(&config.name);

    let required_files = [
        ecosystem_dir.join("ZkStack.yaml"),
        ecosystem_dir.join("configs").join("wallets.yaml"),
        ecosystem_dir
            .join("chains")
            .join(&config.chain_name)
            .join("configs")
            .join("wallets.yaml"),
    ];

    for path in &required_files {
        if !path.exists() {
            log::error!("Required file is missing: {}", path.display());
            return Err(EcosystemError::MissingFile(path.clone()));
        }
        log::debug!("Verified file exists: {}", path.display());
    }

    log::debug!("All required ecosystem files verified");
    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    fn create_test_dir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("adi_test_{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn test_verify_missing_files() {
        let state_dir = create_test_dir();
        let config = EcosystemConfig::default();

        let result = verify_ecosystem_created(&state_dir, &config);
        assert!(result.is_err());

        // Cleanup
        let _ = fs::remove_dir_all(&state_dir);
    }

    #[test]
    fn test_verify_success() {
        let state_dir = create_test_dir();
        let config = EcosystemConfig {
            name: "test_eco".to_string(),
            chain_name: "test_chain".to_string(),
            ..Default::default()
        };

        // Create required files
        let ecosystem_dir = state_dir.join(&config.name);
        fs::create_dir_all(ecosystem_dir.join("configs")).unwrap();
        fs::create_dir_all(
            ecosystem_dir
                .join("chains")
                .join(&config.chain_name)
                .join("configs"),
        )
        .unwrap();

        fs::write(ecosystem_dir.join("ZkStack.yaml"), "test").unwrap();
        fs::write(ecosystem_dir.join("configs").join("wallets.yaml"), "test").unwrap();
        fs::write(
            ecosystem_dir
                .join("chains")
                .join(&config.chain_name)
                .join("configs")
                .join("wallets.yaml"),
            "test",
        )
        .unwrap();

        let result = verify_ecosystem_created(&state_dir, &config);
        assert!(result.is_ok());

        // Cleanup
        let _ = fs::remove_dir_all(&state_dir);
    }
}
