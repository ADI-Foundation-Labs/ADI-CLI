//! Version command implementation.

use crate::error::Result;

include!(concat!(env!("OUT_DIR"), "/built.rs"));

/// Execute the version command.
pub async fn run() -> Result<()> {
    let package_name = PKG_NAME;
    let package_version = PKG_VERSION;

    let git_commit = GIT_COMMIT_HASH.unwrap_or("unknown");
    let git_commit_short = git_commit.get(..8).unwrap_or(git_commit);

    log::info!("{} {}", package_name, package_version);
    log::info!("commit: {}", git_commit_short);

    Ok(())
}
