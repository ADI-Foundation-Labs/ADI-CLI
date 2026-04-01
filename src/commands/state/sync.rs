//! Manual S3 sync command.
//!
//! Syncs the ecosystem state directory to S3 as a tar.gz archive.

use adi_state::s3::{create_tar_gz, S3Client, S3Config};
use clap::Args;
use serde::{Deserialize, Serialize};

use crate::commands::helpers::resolve_ecosystem_name;
use crate::context::Context;
use crate::error::{Result, WrapErr};
use crate::ui;

/// Arguments for the sync subcommand.
#[derive(Clone, Args, Debug, Serialize, Deserialize)]
pub struct SyncArgs {
    /// Ecosystem name to sync.
    #[arg(long, help = "Ecosystem name (falls back to config if not provided)")]
    pub ecosystem_name: Option<String>,
}

/// Execute the sync command.
pub async fn run(args: &SyncArgs, context: &Context) -> Result<()> {
    ui::intro("ADI State Sync")?;

    // Resolve ecosystem name
    let ecosystem_name = resolve_ecosystem_name(args.ecosystem_name.as_ref(), context.config())?;
    let ecosystem_path = context.config().state_dir.join(&ecosystem_name);

    // Verify ecosystem exists
    if !ecosystem_path.exists() {
        return Err(eyre::eyre!(
            "Ecosystem '{}' not found at {}",
            ecosystem_name,
            ecosystem_path.display()
        ));
    }

    // Get S3 config and validate
    let s3_config = &context.config().s3;
    if !s3_config.enabled {
        return Err(eyre::eyre!(
            "S3 sync is not enabled. Set s3.enabled: true in config"
        ));
    }

    let bucket = s3_config
        .bucket
        .as_ref()
        .ok_or_else(|| eyre::eyre!("S3 bucket not configured. Set s3.bucket in config"))?;

    // Get credentials
    let tenant_id = super::helpers::get_tenant_id(s3_config)?;
    let access_key_id = super::helpers::get_access_key_id(s3_config)?;
    let secret_access_key = super::helpers::get_secret_access_key(s3_config)?;

    // Create S3 client
    let client_config = S3Config {
        bucket: bucket.clone(),
        region: s3_config
            .region
            .clone()
            .unwrap_or_else(|| "us-east-1".to_string()),
        endpoint_url: s3_config.endpoint_url.as_ref().map(|u| u.to_string()),
        tenant_id,
        access_key_id,
        secret_access_key,
    };

    ui::info("Connecting to S3...")?;
    let s3_client = S3Client::new(client_config)
        .await
        .wrap_err("Failed to create S3 client")?;

    let archive_key = format!("{}{}.tar.gz", s3_client.key_prefix(), ecosystem_name);

    ui::note(
        "Sync Configuration",
        format!(
            "Ecosystem: {}\nBucket: {}\nKey: {}",
            ui::green(&ecosystem_name),
            ui::green(bucket),
            ui::green(&archive_key)
        ),
    )?;

    // Create archive with spinner
    let spinner = cliclack::spinner();
    spinner.start("Creating archive...");
    let archive_data = create_tar_gz(&ecosystem_path).await.wrap_err(format!(
        "Failed to create archive from {}",
        ecosystem_path.display()
    ))?;

    let archive_size_mb = crate::ui::bytes_to_mb(archive_data.len());
    spinner.stop(format!("Archive created: {:.2} MB", archive_size_mb));

    // Upload to S3 with spinner
    let spinner = cliclack::spinner();
    spinner.start(format!("Uploading to S3 ({archive_size_mb:.2} MB)..."));
    let key = format!("{}.tar.gz", ecosystem_name);
    s3_client
        .upload(&key, archive_data)
        .await
        .wrap_err("Failed to upload to S3")?;
    spinner.stop(format!(
        "Uploaded: s3://{}/{}",
        ui::green(bucket),
        ui::green(&archive_key)
    ));

    ui::success("State synced to S3 successfully")?;
    ui::outro(format!("s3://{}/{}", bucket, archive_key))?;

    Ok(())
}
