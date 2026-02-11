//! Config command implementation.

use crate::{
    context::Context,
    error::{Result, WrapErr},
};

/// Execute the config command.
pub async fn run(context: &Context) -> Result<()> {
    let config = context.config();
    let yaml = serde_yaml::to_string(config).wrap_err("Failed to serialize config to YAML")?;
    log::info!("Config: \n\n{}", yaml.trim_end());
    Ok(())
}
