//! Docker registry authentication.
//!
//! Reads credentials from Docker's credential store (via `~/.docker/config.json`
//! and configured credential helpers) for authenticating with private registries.

use bollard::auth::DockerCredentials;
use docker_credential::{CredentialRetrievalError, DockerCredential};

/// Default Docker registry (Docker Hub).
const DEFAULT_REGISTRY: &str = "https://index.docker.io/v1/";

/// Extract registry hostname from a Docker image URI.
///
/// Image URIs can be in several formats:
/// - `nginx` -> Docker Hub (default registry)
/// - `nginx:latest` -> Docker Hub with tag
/// - `library/nginx` -> Docker Hub with namespace
/// - `registry.example.com/image` -> Custom registry
/// - `registry.example.com:5000/image` -> Custom registry with port
/// - `registry.example.com/namespace/image:tag` -> Custom registry with namespace and tag
///
/// # Arguments
///
/// * `image_uri` - The full image URI (e.g., `harbor.example.com/project/image:tag`)
///
/// # Returns
///
/// The registry hostname or default Docker Hub registry.
fn extract_registry(image_uri: &str) -> &str {
    // If first path segment contains '.', it's a registry hostname
    // (ports like :5000 would come after the hostname which contains '.')
    if let Some(first) = image_uri.split('/').next() {
        if first.contains('.') {
            return first;
        }
    }

    // Default to Docker Hub
    DEFAULT_REGISTRY
}

/// Get Docker credentials for a registry.
///
/// Reads credentials from Docker's credential store, which may use:
/// - Direct credentials in `~/.docker/config.json`
/// - Credential helpers (e.g., `docker-credential-desktop`, `docker-credential-osxkeychain`)
///
/// # Arguments
///
/// * `image_uri` - The full image URI to pull
///
/// # Returns
///
/// `Some(DockerCredentials)` if credentials are found, `None` otherwise.
/// Returns `None` gracefully for public registries or missing credentials.
pub(crate) fn get_credentials_for_image(image_uri: &str) -> Option<DockerCredentials> {
    let registry = extract_registry(image_uri);

    match docker_credential::get_credential(registry) {
        Ok(credential) => Some(convert_credential(credential)),
        Err(CredentialRetrievalError::NoCredentialConfigured) => None,
        Err(CredentialRetrievalError::ConfigNotFound) => None,
        Err(e) => {
            log::debug!("Failed to retrieve credentials for {}: {:?}", registry, e);
            None
        }
    }
}

/// Convert docker_credential's DockerCredential to bollard's DockerCredentials.
fn convert_credential(credential: DockerCredential) -> DockerCredentials {
    match credential {
        DockerCredential::UsernamePassword(username, password) => DockerCredentials {
            username: Some(username),
            password: Some(password),
            ..Default::default()
        },
        DockerCredential::IdentityToken(token) => DockerCredentials {
            identitytoken: Some(token),
            ..Default::default()
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_registry_custom() {
        assert_eq!(
            extract_registry("harbor.example.com/project/image:tag"),
            "harbor.example.com"
        );
        assert_eq!(extract_registry("gcr.io/project/image"), "gcr.io");
        assert_eq!(
            extract_registry("registry.example.com:5000/image"),
            "registry.example.com:5000"
        );
    }

    #[test]
    fn test_extract_registry_dockerhub() {
        assert_eq!(extract_registry("nginx"), DEFAULT_REGISTRY);
        assert_eq!(extract_registry("nginx:latest"), DEFAULT_REGISTRY);
        assert_eq!(extract_registry("library/nginx"), DEFAULT_REGISTRY);
    }
}
