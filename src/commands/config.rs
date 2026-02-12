//! Config command implementation.

use crate::{
    context::Context,
    error::{Result, WrapErr},
    ui,
};

/// Execute the config command.
pub async fn run(context: &Context) -> Result<()> {
    let config = context.config();
    let yaml = serde_yaml::to_string(config).wrap_err("Failed to serialize config to YAML")?;

    ui::intro("ADI CLI")?;
    ui::note("Configuration", yaml.trim_end())?;
    ui::outro("")?;

    Ok(())
}
