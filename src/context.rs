//! Context for command execution.
//!
//! The Context struct carries configuration through command execution.

use adi_toolkit::ToolkitConfig;

use crate::config::Config;
use crate::error::{Result, WrapErr};

/// Execution context for CLI commands.
///
/// Provides access to configuration and shared resources.
#[derive(Clone)]
pub struct Context {
    cfg: Config,
    /// CLI-provided image tag override (highest priority).
    image_tag_override: Option<String>,
}

impl Context {
    /// Create a new context from CLI options.
    ///
    /// # Errors
    ///
    /// Returns an error if configuration loading fails.
    pub fn new_from_options(options: &super::Opts) -> Result<Self> {
        let cfg = Config::new(options.config.as_deref()).wrap_err("Failed to load config")?;
        Ok(Self {
            cfg,
            image_tag_override: options.image_tag.clone(),
        })
    }

    /// Get a reference to the configuration.
    pub fn config(&self) -> &Config {
        &self.cfg
    }

    /// Build ToolkitConfig with overrides applied.
    ///
    /// Priority: CLI flag > env var (via config) > config file > default
    pub fn toolkit_config(&self) -> ToolkitConfig {
        let mut config = ToolkitConfig::default();

        // Apply image tag override (CLI flag > config file/env var)
        let tag = self
            .image_tag_override
            .clone()
            .or_else(|| self.cfg.toolkit.image_tag.clone());

        if let Some(tag) = tag {
            config = config.with_tag_override(tag);
        }

        config
    }
}
