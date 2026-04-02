//! HashiCorp Vault HTTP client for KV v2 secret operations.

use eyre::Result;

/// Vault HTTP client.
pub struct VaultClient {
    #[allow(dead_code)]
    base_url: String,
    #[allow(dead_code)]
    http: reqwest::Client,
}

impl VaultClient {
    /// Create a new Vault client with the given base URL.
    pub fn new(_base_url: &str) -> Result<Self> {
        todo!()
    }

    /// Check Vault connectivity via `/v1/sys/health`.
    pub async fn health_check(&self) -> Result<()> {
        todo!()
    }

    /// Write a secret to the given full API path.
    ///
    /// Sends `POST {base_url}{path}` with body `{"data": data}`.
    pub async fn write_secret(
        &self,
        _token: &str,
        _path: &str,
        _data: &serde_json::Value,
    ) -> Result<()> {
        todo!()
    }
}
