//! S3 client wrapper for state synchronization.
//!
//! Provides a simplified interface for uploading and downloading
//! ecosystem state archives to/from S3-compatible storage.
//!
//! Key prefix is determined from the `tenant_id` configuration field,
//! enabling multi-tenant bucket usage with clear folder separation.

use crate::error::{Result, StateError};
use s3::bucket::Bucket;
use s3::creds::Credentials;
use s3::region::Region;

/// HTTP 404 Not Found status code.
const HTTP_NOT_FOUND: u16 = 404;

/// Configuration for S3 synchronization.
#[derive(Clone, Debug)]
pub struct S3Config {
    /// S3 bucket name.
    pub bucket: String,
    /// AWS region.
    pub region: String,
    /// Optional custom endpoint (for MinIO, LocalStack, etc.).
    pub endpoint_url: Option<String>,
    /// Tenant identifier for S3 key prefix.
    pub tenant_id: String,
    /// AWS access key ID.
    pub access_key_id: String,
    /// AWS secret access key.
    pub secret_access_key: String,
}

/// S3 client for state uploads and downloads.
pub struct S3Client {
    bucket: Box<Bucket>,
    key_prefix: String,
}

impl S3Client {
    /// Create a new S3 client from configuration.
    ///
    /// The key prefix is set to `{tenant_id}/` which provides
    /// clear folder separation for multi-tenant bucket usage.
    ///
    /// # Arguments
    ///
    /// * `config` - S3 configuration with bucket, region, credentials, and tenant_id
    ///
    /// # Errors
    ///
    /// Returns error if client initialization fails.
    pub async fn new(config: S3Config) -> Result<Self> {
        let credentials = Credentials::new(
            Some(&config.access_key_id),
            Some(&config.secret_access_key),
            None, // security token
            None, // session token
            None, // profile
        )
        .map_err(|e| StateError::S3UploadFailed {
            key: "credentials".to_string(),
            reason: e.to_string(),
        })?;

        // Use tenant_id as key prefix for multi-tenant bucket usage
        let key_prefix = format!("{}/", config.tenant_id);

        // Determine region: custom endpoint or standard AWS region
        let region = if let Some(endpoint) = &config.endpoint_url {
            Region::Custom {
                region: config.region.clone(),
                endpoint: endpoint.clone(),
            }
        } else {
            config.region.parse().unwrap_or(Region::UsEast1)
        };

        // Create bucket with path-style addressing for S3-compatible services
        let mut bucket = Bucket::new(&config.bucket, region, credentials).map_err(|e| {
            StateError::S3UploadFailed {
                key: "bucket".to_string(),
                reason: e.to_string(),
            }
        })?;

        // Enable path-style for custom endpoints (MinIO, LocalStack, etc.)
        if config.endpoint_url.is_some() {
            bucket = bucket.with_path_style();
        }

        Ok(Self { bucket, key_prefix })
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

        self.bucket
            .put_object(&full_key, &data)
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

        let response =
            self.bucket
                .get_object(&full_key)
                .await
                .map_err(|e| StateError::S3DownloadFailed {
                    key: full_key,
                    reason: e.to_string(),
                })?;

        Ok(response.to_vec())
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

        let (_, status) =
            self.bucket
                .head_object(&full_key)
                .await
                .map_err(|e| StateError::S3DownloadFailed {
                    key: full_key,
                    reason: e.to_string(),
                })?;

        Ok(status != HTTP_NOT_FOUND)
    }
}
