//! Cleanup utilities for post-container execution.

use adi_types::Logger;
use std::path::Path;

/// Pattern for files to keep in .tmp directory (root level only).
const KEEP_PATTERN: &str = ".md";

/// Clean up the .tmp directory after container execution.
///
/// Keeps only `*.md` files in the root of the .tmp directory.
/// Removes all other files and directories.
///
/// # Arguments
///
/// * `tmp_dir` - Path to the .tmp directory (e.g., `~/.adi_cli/state/<ecosystem>/.tmp`)
/// * `logger` - Logger for debug/warning output
///
/// # Notes
///
/// - This function never fails - all errors are logged as warnings
/// - Symlinks are skipped (not followed or deleted) for safety
/// - Permission errors are logged but do not prevent other cleanups
pub fn cleanup_tmp_dir(tmp_dir: &Path, logger: &dyn Logger) {
    logger.debug(&format!("Cleaning up tmp directory: {}", tmp_dir.display()));

    let entries = match std::fs::read_dir(tmp_dir) {
        Ok(entries) => entries,
        Err(e) => {
            logger.warning(&format!(
                "Failed to read tmp directory {}: {}",
                tmp_dir.display(),
                e
            ));
            return;
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let file_name = entry.file_name();
        let name = file_name.to_string_lossy();

        // Skip symlinks for safety
        if path.is_symlink() {
            logger.debug(&format!("Skipping symlink: {}", path.display()));
            continue;
        }

        // Keep *.md files in root
        if path.is_file() && name.ends_with(KEEP_PATTERN) {
            logger.debug(&format!("Keeping file: {}", path.display()));
            continue;
        }

        // Remove everything else
        let result = if path.is_dir() {
            std::fs::remove_dir_all(&path)
        } else {
            std::fs::remove_file(&path)
        };

        match result {
            Ok(()) => logger.debug(&format!("Removed: {}", path.display())),
            Err(e) => logger.warning(&format!("Failed to remove {}: {}", path.display(), e)),
        }
    }

    logger.debug("Tmp directory cleanup completed");
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use adi_types::NoopLogger;
    use std::fs::{self, File};
    use std::io::Write;
    use tempfile::TempDir;

    fn setup_test_tmp() -> TempDir {
        let dir = TempDir::new().expect("Failed to create temp dir");

        // Create files to keep (*.md)
        let mut file = File::create(dir.path().join("report.md")).expect("create file");
        writeln!(file, "# Report").expect("write");

        let mut file = File::create(dir.path().join("output.md")).expect("create file");
        writeln!(file, "# Output").expect("write");

        // Create files to remove
        File::create(dir.path().join("garbage.txt")).expect("create file");
        File::create(dir.path().join("report-123.toml")).expect("create file");
        File::create(dir.path().join(".cache")).expect("create file");

        // Create directories to remove
        fs::create_dir_all(dir.path().join("node_modules/some_pkg")).expect("create dir");
        fs::create_dir_all(dir.path().join("cache")).expect("create dir");

        dir
    }

    #[test]
    fn test_cleanup_keeps_md_files() {
        let logger = NoopLogger;
        let dir = setup_test_tmp();
        cleanup_tmp_dir(dir.path(), &logger);

        assert!(dir.path().join("report.md").exists());
        assert!(dir.path().join("output.md").exists());
    }

    #[test]
    fn test_cleanup_removes_other_files() {
        let logger = NoopLogger;
        let dir = setup_test_tmp();
        cleanup_tmp_dir(dir.path(), &logger);

        assert!(!dir.path().join("garbage.txt").exists());
        assert!(!dir.path().join("report-123.toml").exists());
        assert!(!dir.path().join(".cache").exists());
    }

    #[test]
    fn test_cleanup_removes_directories() {
        let logger = NoopLogger;
        let dir = setup_test_tmp();
        cleanup_tmp_dir(dir.path(), &logger);

        assert!(!dir.path().join("node_modules").exists());
        assert!(!dir.path().join("cache").exists());
    }

    #[test]
    fn test_cleanup_handles_nonexistent_dir() {
        let logger = NoopLogger;
        let path = Path::new("/nonexistent/path/that/does/not/exist");
        // Should not panic
        cleanup_tmp_dir(path, &logger);
    }

    #[test]
    fn test_cleanup_handles_empty_dir() {
        let logger = NoopLogger;
        let dir = TempDir::new().expect("Failed to create temp dir");
        // Should not panic
        cleanup_tmp_dir(dir.path(), &logger);
    }
}
