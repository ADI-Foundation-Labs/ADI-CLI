//! Context for command execution.
//!
//! The Context struct carries configuration through command execution.

use crate::config::Config;
use crate::error::{Result, WrapErr};

/// Execution context for CLI commands.
///
/// Provides access to configuration and shared resources.
#[derive(Clone)]
pub struct Context {
    cfg: Config,
}

impl Context {
    /// Create a new context from CLI options.
    ///
    /// # Errors
    ///
    /// Returns an error if configuration loading fails.
    pub fn new_from_options(options: &super::Opts) -> Result<Self> {
        let cfg = Config::new(options.config.as_deref()).wrap_err("Failed to load config")?;
        Ok(Self { cfg })
    }

    /// Get a reference to the configuration.
    pub fn config(&self) -> &Config {
        &self.cfg
    }
}
