//! HashiCorp Vault HTTP client for KV v2 secret operations.
//!
//! Provides a minimal client for health-checking a Vault instance and
//! writing secrets to the KV v2 engine.
//!
//! # Example
//!
//! ```rust,ignore
//! use adi_vault::VaultClient;
//!
//! # async fn example() -> eyre::Result<()> {
//! let client = VaultClient::new("https://vault.example.com")?;
//! client.health_check().await?;
//! client.write_secret("hvs.token", "/v1/secret/data/myapp", &serde_json::json!({"key": "val"})).await?;
//! # Ok(())
//! # }
//! ```

use eyre::{eyre, Result};
use reqwest::StatusCode;
use serde_json::json;

/// Vault HTTP client.
pub struct VaultClient {
    base_url: String,
    http: reqwest::Client,
}

impl VaultClient {
    /// Create a new Vault client with the given base URL.
    ///
    /// The trailing slash is stripped from `base_url` if present.
    pub fn new(base_url: &str) -> Result<Self> {
        let http = reqwest::Client::new();
        Ok(Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            http,
        })
    }

    /// Check Vault connectivity via `GET /v1/sys/health`.
    ///
    /// Returns `Ok(())` if Vault responds with a success status.
    pub async fn health_check(&self) -> Result<()> {
        let url = format!("{}/v1/sys/health", self.base_url);

        let response = self
            .http
            .get(&url)
            .send()
            .await
            .map_err(|e| eyre!("Failed to connect to Vault: {e}"))?;

        if !response.status().is_success() {
            return Err(eyre!("Vault health check failed ({})", response.status()));
        }

        Ok(())
    }

    /// Write a secret to the given full API path.
    ///
    /// Sends `POST {base_url}{path}` with body `{"data": data}` and the
    /// provided token in the `X-Vault-Token` header.
    ///
    /// Returns the response body on success.
    pub async fn write_secret(
        &self,
        token: &str,
        path: &str,
        data: &serde_json::Value,
    ) -> Result<String> {
        let url = format!("{}{path}", self.base_url);
        let body = json!({ "data": data });

        let response = self
            .http
            .post(&url)
            .header("X-Vault-Token", token)
            .json(&body)
            .send()
            .await
            .map_err(|e| eyre!("Failed to connect to Vault: {e}"))?;

        let status = response.status();
        let body_text = response.text().await.unwrap_or_default();

        if status.is_success() {
            return Ok(body_text);
        }

        match status {
            StatusCode::FORBIDDEN => Err(eyre!(
                "Vault authentication failed. Token may be expired or lack permissions."
            )),
            StatusCode::NOT_FOUND => Err(eyre!(
                "Vault path not found: {path}. Verify the path is correct."
            )),
            _ => Err(eyre!("Vault API error ({status}): {body_text}")),
        }
    }
}
