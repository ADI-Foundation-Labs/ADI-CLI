use clap::{
    builder::styling::{AnsiColor as Ansi, Styles},
    Parser,
};
use serde::{Deserialize, Serialize};

mod commands;
mod config;
mod context;
mod error;
mod log;

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
    #[command(subcommand)]
    cmd: commands::Commands,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opts: Opts = Opts::parse();
    let ctx = context::Context::new_from_options(&opts)?;
    if let Some(err) = opts.cmd.run(&ctx).await.err() {
        eprintln!("{:?}", err);
        std::process::exit(1);
    }

    Ok(())
}
