//! S3 restore command.
//!
//! Downloads and extracts ecosystem state from S3 to local filesystem.

use adi_state::s3::{extract_tar_gz, S3Client, S3Config};
use clap::Args;
use secrecy::ExposeSecret;
use serde::{Deserialize, Serialize};

use crate::commands::helpers::resolve_ecosystem_name;
use crate::context::Context;
use crate::error::{Result, WrapErr};
use crate::ui;

/// Arguments for the restore subcommand.
#[derive(Clone, Args, Debug, Serialize, Deserialize)]
pub struct RestoreArgs {
    /// Ecosystem name to restore.
    #[arg(long, help = "Ecosystem name (falls back to config if not provided)")]
    pub ecosystem_name: Option<String>,

    /// Force overwrite if local state exists.
    #[arg(long, short = 'f', help = "Overwrite existing local state")]
    pub force: bool,
}

/// Execute the restore command.
pub async fn run(args: &RestoreArgs, context: &Context) -> Result<()> {
    ui::intro("ADI State Restore")?;

    // Resolve ecosystem name
    let ecosystem_name = resolve_ecosystem_name(args.ecosystem_name.as_ref(), context.config())?;
    let ecosystem_path = context.config().state_dir.join(&ecosystem_name);

    // Check if local state exists
    if ecosystem_path.exists() && !args.force {
        ui::warning(format!(
            "Local state exists at {}",
            ui::yellow(ecosystem_path.display())
        ))?;

        let confirmed = ui::confirm("Overwrite existing local state?")
            .initial_value(false)
            .interact()
            .wrap_err("Failed to get confirmation")?;

        if !confirmed {
            ui::outro_cancel("Restore cancelled")?;
            return Ok(());
        }
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
    let tenant_id = get_tenant_id(s3_config)?;
    let access_key_id = get_access_key_id(s3_config)?;
    let secret_access_key = get_secret_access_key(s3_config)?;

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
        "Restore Configuration",
        format!(
            "Ecosystem: {}\nBucket: {}\nKey: {}\nTarget: {}",
            ui::green(&ecosystem_name),
            ui::green(bucket),
            ui::green(&archive_key),
            ui::green(ecosystem_path.display())
        ),
    )?;

    // Download from S3 with spinner
    let spinner = cliclack::spinner();
    spinner.start(format!(
        "Downloading from s3://{}/{}...",
        bucket, archive_key
    ));
    let key = format!("{}.tar.gz", ecosystem_name);
    let archive_data = s3_client
        .download(&key)
        .await
        .wrap_err("Failed to download from S3")?;

    let archive_size_mb = ui::bytes_to_mb(archive_data.len());
    spinner.stop(format!("Downloaded: {:.2} MB", archive_size_mb));

    // Remove existing directory if force
    if ecosystem_path.exists() {
        let spinner = cliclack::spinner();
        spinner.start("Removing existing state...");
        tokio::fs::remove_dir_all(&ecosystem_path)
            .await
            .wrap_err("Failed to remove existing state")?;
        spinner.stop("Existing state removed");
    }

    // Extract archive with spinner
    let spinner = cliclack::spinner();
    spinner.start("Extracting archive...");
    extract_tar_gz(&archive_data, &ecosystem_path)
        .await
        .wrap_err("Failed to extract archive")?;
    spinner.stop(format!(
        "Extracted to: {}",
        ui::green(ecosystem_path.display())
    ));

    ui::success("State restored from S3 successfully")?;
    ui::outro(format!("{}", ecosystem_path.display()))?;

    Ok(())
}

/// Get tenant ID from config.
fn get_tenant_id(config: &crate::config::S3Config) -> Result<String> {
    config
        .tenant_id
        .clone()
        .ok_or_else(|| eyre::eyre!("S3 tenant_id not configured. Set s3.tenant_id in config"))
}

/// Get access key ID from config or environment.
fn get_access_key_id(config: &crate::config::S3Config) -> Result<String> {
    // Check config first (highest priority after env)
    if let Some(ref key) = config.access_key_id {
        return Ok(key.expose_secret().to_string());
    }

    // Check AWS_ACCESS_KEY_ID environment variable
    if let Ok(key) = std::env::var("AWS_ACCESS_KEY_ID") {
        return Ok(key);
    }

    Err(eyre::eyre!(
        "S3 access key not configured. Set s3.access_key_id in config or AWS_ACCESS_KEY_ID env var"
    ))
}

/// Get secret access key from config or environment.
fn get_secret_access_key(config: &crate::config::S3Config) -> Result<String> {
    // Check config first (highest priority after env)
    if let Some(ref key) = config.secret_access_key {
        return Ok(key.expose_secret().to_string());
    }

    // Check AWS_SECRET_ACCESS_KEY environment variable
    if let Ok(key) = std::env::var("AWS_SECRET_ACCESS_KEY") {
        return Ok(key);
    }

    Err(eyre::eyre!(
        "S3 secret key not configured. Set s3.secret_access_key in config or AWS_SECRET_ACCESS_KEY env var"
    ))
}
