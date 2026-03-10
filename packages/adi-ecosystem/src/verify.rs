//! Verification functions for ecosystem operations.
//!
//! This module verifies that ecosystem operations completed successfully
//! by checking for expected files and configurations.

use crate::config::EcosystemConfig;
use crate::error::{EcosystemError, Result};
use adi_types::Logger;
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
/// * `logger` - Logger for debug/error output.
///
/// # Errors
///
/// Returns `EcosystemError::MissingFile` if any required file is missing.
///
/// # Example
///
/// ```rust,no_run
/// use adi_ecosystem::{EcosystemConfig, verify_ecosystem_created};
/// use adi_types::NoopLogger;
/// use std::path::Path;
///
/// let config = EcosystemConfig::default();
/// let state_dir = Path::new("/home/user/.adi_cli/state");
/// let logger = NoopLogger;
///
/// verify_ecosystem_created(state_dir, &config, &logger)?;
/// # Ok::<(), adi_ecosystem::EcosystemError>(())
/// ```
pub fn verify_ecosystem_created(
    state_dir: &Path,
    config: &EcosystemConfig,
    logger: &dyn Logger,
) -> Result<()> {
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
            logger.error(&format!("Required file is missing: {}", path.display()));
            return Err(EcosystemError::MissingFile(path.clone()));
        }
        logger.debug(&format!("Verified file exists: {}", path.display()));
    }

    logger.debug("All required ecosystem files verified");
    Ok(())
}

/// Verify chain was created successfully.
///
/// Checks for the presence of required files that should exist
/// after a successful `zkstack chain create` command.
///
/// # Arguments
///
/// * `state_dir` - The state directory where chain was created.
/// * `ecosystem_name` - Name of the ecosystem containing the chain.
/// * `chain_name` - Name of the created chain.
/// * `logger` - Logger for debug/error output.
///
/// # Errors
///
/// Returns `EcosystemError::MissingFile` if any required file is missing.
///
/// # Example
///
/// ```rust,no_run
/// use adi_ecosystem::verify_chain_created;
/// use adi_types::NoopLogger;
/// use std::path::Path;
///
/// let state_dir = Path::new("/home/user/.adi_cli/state");
/// let logger = NoopLogger;
///
/// verify_chain_created(state_dir, "my_ecosystem", "new_chain", &logger)?;
/// # Ok::<(), adi_ecosystem::EcosystemError>(())
/// ```
pub fn verify_chain_created(
    state_dir: &Path,
    ecosystem_name: &str,
    chain_name: &str,
    logger: &dyn Logger,
) -> Result<()> {
    let chain_dir = state_dir
        .join(ecosystem_name)
        .join("chains")
        .join(chain_name);

    let required_files = [
        chain_dir.join("ZkStack.yaml"),
        chain_dir.join("configs").join("wallets.yaml"),
    ];

    for path in &required_files {
        if !path.exists() {
            logger.error(&format!("Required file is missing: {}", path.display()));
            return Err(EcosystemError::MissingFile(path.clone()));
        }
        logger.debug(&format!("Verified file exists: {}", path.display()));
    }

    logger.debug(&format!(
        "All required chain '{}' files verified",
        chain_name
    ));
    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use adi_types::NoopLogger;
    use std::fs;
    use std::path::PathBuf;

    fn create_test_dir() -> PathBuf {
        let id = format!(
            "{}_{:?}_{:x}",
            std::process::id(),
            std::thread::current().id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let dir = std::env::temp_dir().join(format!("adi_test_{}", id));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn test_verify_missing_files() {
        let logger = NoopLogger;
        let state_dir = create_test_dir();
        let config = EcosystemConfig::default();

        let result = verify_ecosystem_created(&state_dir, &config, &logger);
        assert!(result.is_err());

        // Cleanup
        let _ = fs::remove_dir_all(&state_dir);
    }

    #[test]
    fn test_verify_success() {
        let logger = NoopLogger;
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

        let result = verify_ecosystem_created(&state_dir, &config, &logger);
        assert!(result.is_ok());

        // Cleanup
        let _ = fs::remove_dir_all(&state_dir);
    }

    #[test]
    fn test_verify_chain_missing_files() {
        let logger = NoopLogger;
        let state_dir = create_test_dir();

        let result = verify_chain_created(&state_dir, "test_eco", "new_chain", &logger);
        assert!(result.is_err());

        // Cleanup
        let _ = fs::remove_dir_all(&state_dir);
    }

    #[test]
    fn test_verify_chain_success() {
        let logger = NoopLogger;
        let state_dir = create_test_dir();
        let ecosystem_name = "test_eco";
        let chain_name = "new_chain";

        // Create required files
        let chain_dir = state_dir
            .join(ecosystem_name)
            .join("chains")
            .join(chain_name);
        fs::create_dir_all(chain_dir.join("configs")).unwrap();

        fs::write(chain_dir.join("ZkStack.yaml"), "test").unwrap();
        fs::write(chain_dir.join("configs").join("wallets.yaml"), "test").unwrap();

        let result = verify_chain_created(&state_dir, ecosystem_name, chain_name, &logger);
        assert!(result.is_ok());

        // Cleanup
        let _ = fs::remove_dir_all(&state_dir);
    }
}
