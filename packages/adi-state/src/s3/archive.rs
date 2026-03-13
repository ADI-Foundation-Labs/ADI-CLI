//! Tar/gzip archive utilities for state packaging.
//!
//! Provides functions to create and extract gzip-compressed tar archives
//! for ecosystem state synchronization with S3.

use crate::error::{Result, StateError};
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use std::io::Write;
use std::path::{Path, PathBuf};
use tar::{Archive, Builder};

/// Directories to exclude from archives.
const EXCLUDED_DIRS: &[&str] = &["logs"];

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

/// Recursively append directory contents to tar, excluding specified directories.
fn append_dir_filtered<W: Write>(
    builder: &mut Builder<W>,
    source_dir: &Path,
    current_dir: &Path,
) -> Result<()> {
    let entries = std::fs::read_dir(current_dir).map_err(|e| StateError::ArchiveCreateFailed {
        path: current_dir.to_path_buf(),
        reason: e.to_string(),
    })?;

    for entry in entries {
        let entry = entry.map_err(|e| StateError::ArchiveCreateFailed {
            path: current_dir.to_path_buf(),
            reason: e.to_string(),
        })?;

        let path = entry.path();
        let file_name = entry.file_name();
        let name_str = file_name.to_string_lossy();

        // Skip excluded directories
        if path.is_dir() && EXCLUDED_DIRS.contains(&name_str.as_ref()) {
            continue;
        }

        let relative =
            path.strip_prefix(source_dir)
                .map_err(|e| StateError::ArchiveCreateFailed {
                    path: path.clone(),
                    reason: e.to_string(),
                })?;

        if path.is_file() {
            builder
                .append_path_with_name(&path, relative)
                .map_err(|e| StateError::ArchiveCreateFailed {
                    path: path.clone(),
                    reason: e.to_string(),
                })?;
        } else if path.is_dir() {
            builder
                .append_dir(relative, &path)
                .map_err(|e| StateError::ArchiveCreateFailed {
                    path: path.clone(),
                    reason: e.to_string(),
                })?;
            // Recurse into subdirectory
            append_dir_filtered(builder, source_dir, &path)?;
        }
    }

    Ok(())
}

/// Synchronous implementation of tar.gz creation.
fn create_tar_gz_sync(source_dir: &Path) -> Result<Vec<u8>> {
    let mut archive_data = Vec::new();

    {
        let encoder = GzEncoder::new(&mut archive_data, Compression::default());
        let mut builder = Builder::new(encoder);

        append_dir_filtered(&mut builder, source_dir, source_dir)?;

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
#[allow(clippy::unwrap_used)]
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

    #[tokio::test]
    async fn test_logs_directory_excluded() {
        let source_dir = TempDir::new().unwrap();
        let source_path = source_dir.path();

        // Create test files including logs directory
        fs::write(source_path.join("config.yaml"), "config content").unwrap();
        fs::create_dir(source_path.join("logs")).unwrap();
        fs::write(source_path.join("logs/app.log"), "log content").unwrap();
        fs::create_dir(source_path.join("chains")).unwrap();
        fs::write(source_path.join("chains/chain.yaml"), "chain content").unwrap();

        // Create and extract archive
        let archive_data = create_tar_gz(source_path).await.unwrap();
        let target_dir = TempDir::new().unwrap();
        extract_tar_gz(&archive_data, target_dir.path())
            .await
            .unwrap();

        // Verify config and chains are present
        assert!(target_dir.path().join("config.yaml").exists());
        assert!(target_dir.path().join("chains/chain.yaml").exists());

        // Verify logs directory is excluded
        assert!(!target_dir.path().join("logs").exists());
        assert!(!target_dir.path().join("logs/app.log").exists());
    }
}
