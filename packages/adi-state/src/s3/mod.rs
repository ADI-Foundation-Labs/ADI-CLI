//! S3 synchronization support for state management.
//!
//! This module provides utilities for synchronizing ecosystem state
//! with S3-compatible storage services.
//!
//! # Features
//!
//! - Create and extract gzip-compressed tar archives
//! - Upload/download state archives to/from S3
//! - Support for S3-compatible services (MinIO, LocalStack)
//! - Automatic key prefix detection from IAM identity
//!
//! # Key Prefix Auto-Detection
//!
//! The S3 client automatically determines the key prefix from the AWS IAM identity:
//! - IAM User `alice` → prefix `alice/`
//! - Assumed Role `deploy-role` → prefix `deploy-role/`
//! - Root account → prefix `root/`
//!
//! This enables multi-tenant bucket usage with automatic isolation.
//!
//! # Example
//!
//! ```rust,ignore
//! use adi_state::s3::{S3Client, S3Config, create_tar_gz, extract_tar_gz};
//! use std::path::Path;
//!
//! # async fn example() -> adi_state::Result<()> {
//! // Create archive from ecosystem directory
//! let archive = create_tar_gz(Path::new("/path/to/ecosystem")).await?;
//!
//! // Upload to S3 (key prefix auto-detected from IAM identity)
//! let config = S3Config {
//!     bucket: "my-bucket".to_string(),
//!     region: "us-east-1".to_string(),
//!     endpoint_url: None,
//!     access_key_id: "AKIA...".to_string(),
//!     secret_access_key: "...".to_string(),
//! };
//! let client = S3Client::new(config).await?;
//! // If IAM user is "alice", this uploads to "alice/my_ecosystem.tar.gz"
//! client.upload("my_ecosystem.tar.gz", archive).await?;
//! # Ok(())
//! # }
//! ```

mod archive;
mod client;

pub use archive::{create_tar_gz, extract_tar_gz};
pub use client::{S3Client, S3Config};
