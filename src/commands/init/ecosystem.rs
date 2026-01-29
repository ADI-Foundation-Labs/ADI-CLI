//! Ecosystem initialization command implementation.

use adi_toolkit::{DockerClient, DockerConfig, ProtocolVersion};

use super::EcosystemArgs;
use crate::context::Context;
use crate::error::{Result, WrapErr};

/// Execute the ecosystem initialization command.
///
/// This command:
/// 1. Validates the protocol version
/// 2. Connects to Docker daemon
/// 3. Ensures the toolkit image is available (pulls if needed)
/// 4. [TODO] Runs zkstack ecosystem init
pub async fn run(args: &EcosystemArgs, _context: &Context) -> Result<()> {
    log::debug!("Starting ecosystem initialization");
    log::debug!("Protocol version argument: {}", args.protocol_version);

    // 1. Parse and validate protocol version
    let version = ProtocolVersion::parse(&args.protocol_version)
        .wrap_err("Invalid protocol version")?;
    log::debug!("Parsed protocol version: {:?}", version);

    // 2. Connect to Docker daemon (SDK logs connection details)
    let client = DockerClient::new()
        .await
        .wrap_err("Failed to connect to Docker. Is Docker running?")?;

    // 3. Build image reference from config and version
    let config = DockerConfig::default();
    let image_ref = config.image_reference(&version.to_semver());

    // 4. Ensure image is available (pull_image checks existence internally)
    client
        .pull_image(&image_ref)
        .await
        .wrap_err("Failed to pull toolkit image")?;

    // 6. [TODO] Run zkstack ecosystem init in container
    log::info!("Docker setup complete. Image: {}", image_ref.full_uri());
    log::info!("[TODO] Would run: zkstack ecosystem init");

    Ok(())
}
