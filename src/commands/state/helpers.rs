//! Shared helpers for state S3 operations.

use secrecy::ExposeSecret;

use crate::error::Result;

/// Get tenant ID from config.
pub fn get_tenant_id(config: &crate::config::S3Config) -> Result<String> {
    config
        .tenant_id
        .clone()
        .ok_or_else(|| eyre::eyre!("S3 tenant_id not configured. Set s3.tenant_id in config"))
}

/// Get access key ID from config or environment.
pub fn get_access_key_id(config: &crate::config::S3Config) -> Result<String> {
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
pub fn get_secret_access_key(config: &crate::config::S3Config) -> Result<String> {
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
