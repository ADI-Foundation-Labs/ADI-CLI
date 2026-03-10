use std::path::PathBuf;

use clap::{
    builder::styling::{AnsiColor as Ansi, Styles},
    Parser,
};
use serde::{Deserialize, Serialize};

mod commands;
mod config;
mod config_writer;
mod context;
mod error;
mod s3_events;
mod theme;
mod ui;

const STYLES: Styles = Styles::styled()
    .header(Ansi::Green.on_default().bold())
    .usage(Ansi::Green.on_default().bold())
    .literal(Ansi::BrightCyan.on_default().bold())
    .placeholder(Ansi::BrightCyan.on_default())
    .error(Ansi::Red.on_default().bold());

#[derive(Clone, Parser, Debug, Serialize, Deserialize)]
#[clap(about)]
#[command(styles = STYLES)]
pub struct Opts {
    /// Path to config file (overrides default locations)
    #[arg(global = true, short = 'c', long = "config")]
    #[serde(default)]
    pub config: Option<PathBuf>,

    /// Enable debug logging
    #[arg(global = true, short = 'd', long)]
    #[serde(default)]
    pub debug: bool,

    /// Override toolkit Docker image tag (e.g., "v30.0.2-custom" or "latest")
    #[arg(global = true, long = "image-tag")]
    #[serde(default)]
    pub image_tag: Option<String>,

    #[command(subcommand)]
    cmd: commands::Commands,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    cliclack::set_theme(theme::AdiTheme);
    let opts: Opts = Opts::parse();

    let ctx = context::Context::new_from_options(&opts)?;

    ctx.logger().debug("Debug mode enabled");

    if let Some(err) = opts.cmd.run(&ctx).await.err() {
        let _ = ui::error(format!("{:?}", err));
        std::process::exit(1);
    }

    Ok(())
}
