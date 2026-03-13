//! Shell completion generation command.

use std::io;

use clap::{Args, CommandFactory, ValueEnum};
use clap_complete::{generate, Shell};
use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::Opts;

/// Supported shell types for completion generation.
#[derive(Clone, Copy, Debug, ValueEnum, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ShellType {
    /// Bash shell
    Bash,
    /// Zsh shell
    Zsh,
    /// Fish shell
    Fish,
    /// PowerShell
    #[value(name = "powershell")]
    PowerShell,
}

impl From<ShellType> for Shell {
    fn from(shell: ShellType) -> Self {
        match shell {
            ShellType::Bash => Shell::Bash,
            ShellType::Zsh => Shell::Zsh,
            ShellType::Fish => Shell::Fish,
            ShellType::PowerShell => Shell::PowerShell,
        }
    }
}

/// Arguments for the `completions` command.
#[derive(Clone, Args, Debug, Serialize, Deserialize)]
#[command(after_help = "\
Installation:
  Bash:
    mkdir -p ~/.local/share/bash-completion/completions
    adi completions bash > ~/.local/share/bash-completion/completions/adi

  Zsh (Oh My Zsh):
    mkdir -p ~/.oh-my-zsh/completions
    adi completions zsh > ~/.oh-my-zsh/completions/_adi

  Fish:
    mkdir -p ~/.config/fish/completions
    adi completions fish > ~/.config/fish/completions/adi.fish

  PowerShell:
    adi completions powershell >> $PROFILE
")]
pub struct CompletionsArgs {
    /// Shell to generate completions for
    #[arg(value_enum)]
    pub shell: ShellType,
}

/// Generate shell completion scripts.
pub async fn run(args: &CompletionsArgs) -> Result<()> {
    let mut cmd = Opts::command();
    let shell: Shell = args.shell.into();
    generate(shell, &mut cmd, "adi", &mut io::stdout());
    Ok(())
}
