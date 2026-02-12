//! Version command implementation.

use crate::{error::Result, ui};

include!(concat!(env!("OUT_DIR"), "/built.rs"));

/// Execute the version command.
pub async fn run() -> Result<()> {
    let package_version = PKG_VERSION;

    let git_commit = GIT_COMMIT_HASH.unwrap_or("unknown");
    let git_commit_short = git_commit.get(..8).unwrap_or(git_commit);

    let info = format!(
        "Version: {}\nCommit:  {}",
        package_version, git_commit_short
    );

    ui::intro("ADI CLI")?;
    ui::note("Build Info", info)?;
    ui::outro("")?;

    Ok(())
}
