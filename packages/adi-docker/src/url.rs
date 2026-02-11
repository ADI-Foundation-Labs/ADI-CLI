//! URL utilities for Docker container networking.
//!
//! Provides helpers for transforming URLs when passing them to Docker containers,
//! particularly handling the macOS Docker Desktop networking quirk where containers
//! cannot reach `localhost` on the host machine.

use adi_types::Logger;
use url::Url;

/// Docker host internal hostname used by Docker Desktop on macOS.
const DOCKER_HOST_INTERNAL: &str = "host.docker.internal";

/// Transform a URL for use inside a Docker container.
///
/// On macOS, Docker containers cannot reach `localhost` or `127.0.0.1` on the
/// host machine. This function transforms such URLs to use `host.docker.internal`
/// instead, which Docker Desktop resolves to the host machine.
///
/// On other platforms (Linux), the URL is returned unchanged since Docker's
/// host networking mode works as expected.
///
/// # Arguments
///
/// * `url` - The URL to transform.
/// * `logger` - Logger for debug output.
///
/// # Returns
///
/// The transformed URL (on macOS with localhost) or the original URL.
///
/// # Example
///
/// ```rust,no_run
/// use adi_docker::transform_url_for_container;
/// use adi_types::NoopLogger;
///
/// let logger = NoopLogger;
/// let url = "http://localhost:8545";
/// let transformed = transform_url_for_container(url, &logger);
/// // On macOS: "http://host.docker.internal:8545/"
/// // On Linux: "http://localhost:8545"
/// ```
pub fn transform_url_for_container(url: &str, logger: &dyn Logger) -> String {
    // Only transform on macOS
    if std::env::consts::OS != "macos" {
        return url.to_string();
    }

    // Parse the URL
    let Ok(mut parsed) = Url::parse(url) else {
        return url.to_string();
    };

    // Get host
    let Some(host) = parsed.host_str() else {
        return url.to_string();
    };

    // Check if host is localhost variant
    let lower = host.to_lowercase();
    if lower != "localhost" && lower != "127.0.0.1" {
        return url.to_string();
    }

    // Replace with host.docker.internal
    if parsed.set_host(Some(DOCKER_HOST_INTERNAL)).is_err() {
        return url.to_string();
    }

    logger.debug(&format!(
        "Transformed localhost URL for Docker container: {} -> {}",
        url, parsed
    ));

    parsed.to_string()
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use adi_types::NoopLogger;

    #[test]
    fn test_transform_preserves_remote_urls() {
        let logger = NoopLogger;
        let url = "https://sepolia.infura.io/v3/key";
        assert_eq!(transform_url_for_container(url, &logger), url);
    }

    #[test]
    fn test_transform_handles_invalid_url() {
        let logger = NoopLogger;
        let url = "not-a-valid-url";
        assert_eq!(transform_url_for_container(url, &logger), url);
    }

    #[test]
    fn test_transform_preserves_port_in_parsing() {
        // Verify URL parsing preserves port
        let url = "http://localhost:8545";
        let parsed = Url::parse(url).unwrap();
        assert_eq!(parsed.port(), Some(8545));
    }

    #[test]
    fn test_transform_preserves_path() {
        // Verify URL parsing preserves path
        let url = "http://localhost:8545/api/v1";
        let parsed = Url::parse(url).unwrap();
        assert_eq!(parsed.path(), "/api/v1");
    }
}
