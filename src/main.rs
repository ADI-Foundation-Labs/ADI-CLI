use std::io::Write;
use std::path::PathBuf;

use chrono::Local;
use clap::{
    builder::styling::{AnsiColor as Ansi, Styles},
    Parser,
};
use env_logger::fmt::style::{AnsiColor, Style};
use serde::{Deserialize, Serialize};

mod commands;
mod config;
mod context;
mod error;

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

    #[command(subcommand)]
    cmd: commands::Commands,
}

/// Initialize the logger with colored output.
///
/// Uses env_logger with custom formatting to match the CLI style.
/// Default log level is `info`, or `debug` if debug mode is enabled.
/// Controllable via `RUST_LOG` environment variable (takes precedence).
fn init_logger(debug: bool) {
    let default_level = if debug { "debug" } else { "info" };
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(default_level))
        .format(|buf, record| {
            let level_style = match record.level() {
                ::log::Level::Error => Style::new().fg_color(Some(AnsiColor::Red.into())).bold(),
                ::log::Level::Warn => Style::new().fg_color(Some(AnsiColor::Yellow.into())).bold(),
                ::log::Level::Info => Style::new().fg_color(Some(AnsiColor::Cyan.into())),
                ::log::Level::Debug => Style::new().fg_color(Some(AnsiColor::BrightBlack.into())),
                ::log::Level::Trace => Style::new().fg_color(Some(AnsiColor::Magenta.into())),
            };
            let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");

            // Apply color to entire message for Warn/Error, only to tag for others
            let color_full_message =
                matches!(record.level(), ::log::Level::Error | ::log::Level::Warn);

            if color_full_message {
                writeln!(
                    buf,
                    "[{timestamp}] {style}[{level}]: {args}\x1b[0m",
                    timestamp = timestamp,
                    style = level_style,
                    level = record.level(),
                    args = record.args()
                )
            } else {
                writeln!(
                    buf,
                    "[{timestamp}] {style}[{level}]\x1b[0m: {args}",
                    timestamp = timestamp,
                    style = level_style,
                    level = record.level(),
                    args = record.args()
                )
            }
        })
        .init();
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opts: Opts = Opts::parse();
    let cfg = config::Config::new(opts.config.as_deref())?;

    // CLI flag takes precedence over config file
    let debug_enabled = opts.debug || cfg.debug;
    init_logger(debug_enabled);

    log::debug!("Debug mode enabled");

    let ctx = context::Context::new_from_options(&opts)?;
    if let Some(err) = opts.cmd.run(&ctx).await.err() {
        ::log::error!("{:?}", err);
        std::process::exit(1);
    }

    Ok(())
}
