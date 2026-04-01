//! URL utilities for RPC connections.
//!
//! Provides helpers for normalizing URLs between Docker container and host contexts.

/// Check if an RPC URL points to localhost.
///
/// Returns `true` for URLs containing:
/// - `localhost`
/// - `127.0.0.1`
/// - `host.docker.internal` (Docker's localhost equivalent)
///
/// # Example
///
/// ```rust
/// use adi_types::is_localhost_rpc;
///
/// assert!(is_localhost_rpc("http://localhost:8545"));
/// assert!(is_localhost_rpc("http://127.0.0.1:8545"));
/// assert!(is_localhost_rpc("http://host.docker.internal:8545"));
/// assert!(!is_localhost_rpc("https://sepolia.infura.io/v3/key"));
/// ```
pub fn is_localhost_rpc(rpc_url: &str) -> bool {
    let lower = rpc_url.to_lowercase();
    lower.contains("localhost")
        || lower.contains("127.0.0.1")
        || lower.contains("host.docker.internal")
}

/// Normalize RPC URL for host-side connections.
///
/// Converts `host.docker.internal` to `localhost` since the former
/// is only resolvable from inside Docker containers.
///
/// # Example
///
/// ```rust
/// use adi_types::normalize_rpc_url;
///
/// assert_eq!(
///     normalize_rpc_url("http://host.docker.internal:8545"),
///     "http://localhost:8545"
/// );
/// assert_eq!(
///     normalize_rpc_url("http://localhost:8545"),
///     "http://localhost:8545"
/// );
/// ```
pub fn normalize_rpc_url(rpc_url: &str) -> String {
    let lower = rpc_url.to_lowercase();
    if lower.contains("host.docker.internal") {
        lower.replace("host.docker.internal", "localhost")
    } else {
        rpc_url.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_localhost_rpc_localhost() {
        assert!(is_localhost_rpc("http://localhost:8545"));
        assert!(is_localhost_rpc("http://LOCALHOST:8545"));
        assert!(is_localhost_rpc("https://localhost:8545"));
    }

    #[test]
    fn test_is_localhost_rpc_127() {
        assert!(is_localhost_rpc("http://127.0.0.1:8545"));
        assert!(is_localhost_rpc("https://127.0.0.1:8545"));
    }

    #[test]
    fn test_is_localhost_rpc_docker_internal() {
        assert!(is_localhost_rpc("http://host.docker.internal:8545"));
        assert!(is_localhost_rpc("http://HOST.DOCKER.INTERNAL:8545"));
    }

    #[test]
    fn test_is_localhost_rpc_remote() {
        assert!(!is_localhost_rpc("https://sepolia.infura.io/v3/key"));
        assert!(!is_localhost_rpc("https://mainnet.infura.io/v3/key"));
        assert!(!is_localhost_rpc("https://rpc.zksync.io"));
    }

    #[test]
    fn test_normalize_rpc_url_docker_internal() {
        assert_eq!(
            normalize_rpc_url("http://host.docker.internal:8545"),
            "http://localhost:8545"
        );
        assert_eq!(
            normalize_rpc_url("http://HOST.DOCKER.INTERNAL:8545"),
            "http://localhost:8545"
        );
        // Mixed case is also handled
        assert_eq!(
            normalize_rpc_url("http://Host.Docker.Internal:8545"),
            "http://localhost:8545"
        );
    }

    #[test]
    fn test_normalize_rpc_url_passthrough() {
        assert_eq!(
            normalize_rpc_url("http://localhost:8545"),
            "http://localhost:8545"
        );
        assert_eq!(
            normalize_rpc_url("https://sepolia.infura.io/v3/key"),
            "https://sepolia.infura.io/v3/key"
        );
    }
}
