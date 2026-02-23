//! Tar/gzip archive utilities for state packaging.
//!
//! Provides functions to create and extract gzip-compressed tar archives
//! for ecosystem state synchronization with S3.

use crate::error::{Result, StateError};
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use std::path::{Path, PathBuf};
use tar::{Archive, Builder};

/// Create a gzipped tar archive from a directory.
///
/// # Arguments
///
/// * `source_dir` - Directory to archive
///
/// # Returns
///
/// Compressed archive data as bytes
///
/// # Errors
///
/// Returns `StateError::ArchiveCreateFailed` if archiving fails.
pub async fn create_tar_gz(source_dir: &Path) -> Result<Vec<u8>> {
    let source_dir = source_dir.to_path_buf();

    tokio::task::spawn_blocking(move || create_tar_gz_sync(&source_dir))
        .await
        .map_err(|e| StateError::ArchiveCreateFailed {
            path: PathBuf::new(),
            reason: format!("Task join error: {e}"),
        })?
}

/// Synchronous implementation of tar.gz creation.
fn create_tar_gz_sync(source_dir: &Path) -> Result<Vec<u8>> {
    let mut archive_data = Vec::new();

    {
        let encoder = GzEncoder::new(&mut archive_data, Compression::default());
        let mut builder = Builder::new(encoder);

        builder
            .append_dir_all(".", source_dir)
            .map_err(|e| StateError::ArchiveCreateFailed {
                path: source_dir.to_path_buf(),
                reason: e.to_string(),
            })?;

        let encoder = builder
            .into_inner()
            .map_err(|e| StateError::ArchiveCreateFailed {
                path: source_dir.to_path_buf(),
                reason: e.to_string(),
            })?;

        encoder
            .finish()
            .map_err(|e| StateError::ArchiveCreateFailed {
                path: source_dir.to_path_buf(),
                reason: e.to_string(),
            })?;
    }

    Ok(archive_data)
}

/// Extract a gzipped tar archive to a directory.
///
/// # Arguments
///
/// * `archive_data` - Compressed archive data
/// * `target_dir` - Directory to extract to
///
/// # Errors
///
/// Returns `StateError::ArchiveExtractFailed` if extraction fails.
pub async fn extract_tar_gz(archive_data: &[u8], target_dir: &Path) -> Result<()> {
    let archive_data = archive_data.to_vec();
    let target_dir = target_dir.to_path_buf();

    // Ensure target directory exists
    tokio::fs::create_dir_all(&target_dir)
        .await
        .map_err(|e| StateError::WriteFailed {
            path: target_dir.clone(),
            source: e,
        })?;

    tokio::task::spawn_blocking(move || extract_tar_gz_sync(&archive_data, &target_dir))
        .await
        .map_err(|e| StateError::ArchiveExtractFailed {
            path: PathBuf::new(),
            reason: format!("Task join error: {e}"),
        })?
}

/// Synchronous implementation of tar.gz extraction.
fn extract_tar_gz_sync(archive_data: &[u8], target_dir: &Path) -> Result<()> {
    let decoder = GzDecoder::new(archive_data);
    let mut archive = Archive::new(decoder);

    archive
        .unpack(target_dir)
        .map_err(|e| StateError::ArchiveExtractFailed {
            path: target_dir.to_path_buf(),
            reason: e.to_string(),
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_create_and_extract_tar_gz() {
        // Create a temp directory with some files
        let source_dir = TempDir::new().unwrap();
        let source_path = source_dir.path();

        // Create test files
        fs::write(source_path.join("test.txt"), "hello world").unwrap();
        fs::create_dir(source_path.join("subdir")).unwrap();
        fs::write(source_path.join("subdir/nested.txt"), "nested content").unwrap();

        // Create archive
        let archive_data = create_tar_gz(source_path).await.unwrap();
        assert!(!archive_data.is_empty());

        // Extract to new directory
        let target_dir = TempDir::new().unwrap();
        extract_tar_gz(&archive_data, target_dir.path())
            .await
            .unwrap();

        // Verify extracted files
        let content = fs::read_to_string(target_dir.path().join("test.txt")).unwrap();
        assert_eq!(content, "hello world");

        let nested = fs::read_to_string(target_dir.path().join("subdir/nested.txt")).unwrap();
        assert_eq!(nested, "nested content");
    }
}
