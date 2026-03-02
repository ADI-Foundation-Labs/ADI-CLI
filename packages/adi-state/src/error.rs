//! Error types for state operations.

use std::path::PathBuf;
use thiserror::Error;

/// Result type alias using StateError.
pub type Result<T> = std::result::Result<T, StateError>;

/// Errors that can occur during state operations.
#[derive(Error, Debug)]
pub enum StateError {
    /// State directory does not exist.
    #[error("State directory not found: {}", .0.display())]
    StateDirectoryNotFound(PathBuf),

    /// Failed to read from state backend.
    #[error("Failed to read state file '{}': {source}", .path.display())]
    ReadFailed {
        /// Path that failed to read.
        path: PathBuf,
        /// Underlying error.
        #[source]
        source: std::io::Error,
    },

    /// Failed to write to state backend.
    #[error("Failed to write state file '{}': {source}", .path.display())]
    WriteFailed {
        /// Path that failed to write.
        path: PathBuf,
        /// Underlying error.
        #[source]
        source: std::io::Error,
    },

    /// Failed to parse YAML content.
    #[error("Failed to parse YAML from '{}': {source}", .path.display())]
    YamlParseFailed {
        /// Path to file that failed parsing.
        path: PathBuf,
        /// Underlying YAML error.
        #[source]
        source: serde_yaml::Error,
    },

    /// Failed to serialize to YAML.
    #[error("Failed to serialize YAML for '{}': {source}", .path.display())]
    YamlSerializeFailed {
        /// Path where serialization was intended.
        path: PathBuf,
        /// Underlying YAML error.
        #[source]
        source: serde_yaml::Error,
    },

    /// Requested resource was not found.
    #[error("State file not found: {}", .0.display())]
    NotFound(PathBuf),

    /// State file already exists (cannot create).
    #[error("State file already exists: {}", .0.display())]
    AlreadyExists(PathBuf),

    /// Failed to delete state file.
    #[error("Failed to delete state file '{}': {source}", .path.display())]
    DeleteFailed {
        /// Path that failed to delete.
        path: PathBuf,
        /// Underlying error.
        #[source]
        source: std::io::Error,
    },

    /// Chain not found in ecosystem.
    #[error("Chain '{name}' not found in ecosystem")]
    ChainNotFound {
        /// Chain name that was not found.
        name: String,
    },

    /// S3 upload failed.
    #[error("Failed to upload to S3 key '{key}': {reason}")]
    S3UploadFailed {
        /// S3 object key that failed to upload.
        key: String,
        /// Error description.
        reason: String,
    },

    /// S3 download failed.
    #[error("Failed to download from S3 key '{key}': {reason}")]
    S3DownloadFailed {
        /// S3 object key that failed to download.
        key: String,
        /// Error description.
        reason: String,
    },

    /// Archive creation failed.
    #[error("Failed to create archive from '{}': {reason}", .path.display())]
    ArchiveCreateFailed {
        /// Path that failed to archive.
        path: PathBuf,
        /// Error description.
        reason: String,
    },

    /// Archive extraction failed.
    #[error("Failed to extract archive to '{}': {reason}", .path.display())]
    ArchiveExtractFailed {
        /// Path where extraction failed.
        path: PathBuf,
        /// Error description.
        reason: String,
    },
}
