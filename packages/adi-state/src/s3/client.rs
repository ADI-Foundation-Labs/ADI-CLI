//! S3 client wrapper for state synchronization.
//!
//! Provides a simplified interface for uploading and downloading
//! ecosystem state archives to/from S3-compatible storage.
//!
//! Key prefix is automatically determined from IAM identity (username or role name).

use crate::error::{Result, StateError};
use aws_config::BehaviorVersion;
use aws_sdk_s3::config::{Credentials, Region};
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::Client;
use aws_sdk_sts::Client as StsClient;

/// Configuration for S3 synchronization.
#[derive(Clone, Debug)]
pub struct S3Config {
    /// S3 bucket name.
    pub bucket: String,
    /// AWS region.
    pub region: String,
    /// Optional custom endpoint (for MinIO, LocalStack, etc.).
    pub endpoint_url: Option<String>,
    /// AWS access key ID.
    pub access_key_id: String,
    /// AWS secret access key.
    pub secret_access_key: String,
}

/// S3 client for state uploads and downloads.
pub struct S3Client {
    client: Client,
    bucket: String,
    key_prefix: String,
}

impl S3Client {
    /// Create a new S3 client from configuration.
    ///
    /// The key prefix is automatically determined from the IAM identity
    /// (username for IAM users, role name for assumed roles).
    ///
    /// # Arguments
    ///
    /// * `config` - S3 configuration with bucket, region, and credentials
    ///
    /// # Errors
    ///
    /// Returns error if client initialization fails.
    pub async fn new(config: S3Config) -> Result<Self> {
        let credentials = Credentials::new(
            &config.access_key_id,
            &config.secret_access_key,
            None, // session token
            None, // expiry
            "adi-cli",
        );

        // Load base SDK config with proper runtime support
        let sdk_config = aws_config::defaults(BehaviorVersion::latest())
            .region(Region::new(config.region.clone()))
            .credentials_provider(credentials)
            .load()
            .await;

        // Auto-detect IAM identity for key prefix
        let key_prefix = get_iam_identity(&sdk_config)
            .await
            .map(|name| format!("{}/", name))
            .unwrap_or_default();

        // Build S3 client config from SDK config
        let mut s3_config_builder =
            aws_sdk_s3::config::Builder::from(&sdk_config).region(Region::new(config.region));

        // Custom endpoint for S3-compatible services
        if let Some(endpoint) = &config.endpoint_url {
            s3_config_builder = s3_config_builder
                .endpoint_url(endpoint)
                .force_path_style(true);
        }

        let client = Client::from_conf(s3_config_builder.build());

        Ok(Self {
            client,
            bucket: config.bucket,
            key_prefix,
        })
    }

    /// Get the full S3 key with prefix.
    fn full_key(&self, key: &str) -> String {
        format!("{}{}", self.key_prefix, key)
    }

    /// Get the current key prefix (for debugging/logging).
    #[must_use]
    pub fn key_prefix(&self) -> &str {
        &self.key_prefix
    }

    /// Upload data to S3.
    ///
    /// # Arguments
    ///
    /// * `key` - Object key (will be prefixed)
    /// * `data` - Data to upload
    ///
    /// # Errors
    ///
    /// Returns `StateError::S3UploadFailed` if upload fails.
    pub async fn upload(&self, key: &str, data: Vec<u8>) -> Result<()> {
        let full_key = self.full_key(key);

        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(&full_key)
            .body(ByteStream::from(data))
            .send()
            .await
            .map_err(|e| StateError::S3UploadFailed {
                key: full_key,
                reason: e.to_string(),
            })?;

        Ok(())
    }

    /// Download data from S3.
    ///
    /// # Arguments
    ///
    /// * `key` - Object key (will be prefixed)
    ///
    /// # Returns
    ///
    /// Downloaded data as bytes.
    ///
    /// # Errors
    ///
    /// Returns `StateError::S3DownloadFailed` if download fails.
    pub async fn download(&self, key: &str) -> Result<Vec<u8>> {
        let full_key = self.full_key(key);

        let response = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(&full_key)
            .send()
            .await
            .map_err(|e| StateError::S3DownloadFailed {
                key: full_key.clone(),
                reason: e.to_string(),
            })?;

        let data = response
            .body
            .collect()
            .await
            .map_err(|e| StateError::S3DownloadFailed {
                key: full_key,
                reason: e.to_string(),
            })?;

        Ok(data.into_bytes().to_vec())
    }

    /// Check if an object exists in S3.
    ///
    /// # Arguments
    ///
    /// * `key` - Object key (will be prefixed)
    ///
    /// # Returns
    ///
    /// `true` if object exists, `false` otherwise.
    pub async fn exists(&self, key: &str) -> Result<bool> {
        let full_key = self.full_key(key);

        match self
            .client
            .head_object()
            .bucket(&self.bucket)
            .key(&full_key)
            .send()
            .await
        {
            Ok(_) => Ok(true),
            Err(e) => {
                // Check if it's a "not found" error
                let service_error = e.into_service_error();
                if service_error.is_not_found() {
                    Ok(false)
                } else {
                    Err(StateError::S3DownloadFailed {
                        key: full_key,
                        reason: service_error.to_string(),
                    })
                }
            }
        }
    }
}

/// Get IAM identity (username or role name) from STS GetCallerIdentity.
///
/// Returns `None` if identity cannot be determined.
async fn get_iam_identity(sdk_config: &aws_config::SdkConfig) -> Option<String> {
    let sts = StsClient::new(sdk_config);

    let identity = sts.get_caller_identity().send().await.ok()?;
    let arn = identity.arn()?;

    parse_identity_from_arn(arn)
}

/// Parse identity name from AWS ARN.
///
/// Supported ARN formats:
/// - IAM User: `arn:aws:iam::123456789012:user/alice` → `alice`
/// - IAM Role: `arn:aws:sts::123456789012:assumed-role/role-name/session` → `role-name`
/// - Root: `arn:aws:iam::123456789012:root` → `root`
fn parse_identity_from_arn(arn: &str) -> Option<String> {
    let parts: Vec<&str> = arn.split(':').collect();
    let resource = parts.get(5)?;

    if resource.starts_with("user/") {
        // IAM User: user/alice or user/admins/alice → alice (last part)
        resource.rsplit('/').next().map(String::from)
    } else if resource.starts_with("assumed-role/") {
        // Assumed Role: assumed-role/role-name/session-name → role-name
        let role_parts: Vec<&str> = resource.split('/').collect();
        role_parts.get(1).map(|s| (*s).to_string())
    } else if *resource == "root" {
        Some("root".to_string())
    } else {
        // Fallback: use last part after /
        resource.rsplit('/').next().map(String::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_iam_user_arn() {
        let arn = "arn:aws:iam::123456789012:user/alice";
        assert_eq!(parse_identity_from_arn(arn), Some("alice".to_string()));
    }

    #[test]
    fn test_parse_iam_user_with_path() {
        let arn = "arn:aws:iam::123456789012:user/admins/alice";
        assert_eq!(parse_identity_from_arn(arn), Some("alice".to_string()));
    }

    #[test]
    fn test_parse_assumed_role_arn() {
        let arn = "arn:aws:sts::123456789012:assumed-role/deploy-role/session-name";
        assert_eq!(
            parse_identity_from_arn(arn),
            Some("deploy-role".to_string())
        );
    }

    #[test]
    fn test_parse_root_arn() {
        let arn = "arn:aws:iam::123456789012:root";
        assert_eq!(parse_identity_from_arn(arn), Some("root".to_string()));
    }

    #[test]
    fn test_parse_invalid_arn() {
        assert_eq!(parse_identity_from_arn("invalid"), None);
        assert_eq!(parse_identity_from_arn(""), None);
    }
}
