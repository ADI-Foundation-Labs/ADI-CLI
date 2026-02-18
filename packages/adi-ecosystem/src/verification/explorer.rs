//! Block explorer API client for verification status checks.
//!
//! Supports Etherscan-compatible and Blockscout APIs.

use super::error::VerificationError;
use super::types::{ExplorerType, VerificationStatus};
use adi_types::Logger;
use alloy_primitives::Address;
use serde::Deserialize;
use std::sync::Arc;
use std::time::Duration;
use url::Url;

/// Default delay between API requests to avoid rate limiting.
const DEFAULT_REQUEST_DELAY_MS: u64 = 200;

/// Maximum retries for rate-limited requests.
const MAX_RETRIES: u32 = 3;

/// Explorer API client configuration.
#[derive(Debug, Clone)]
pub struct ExplorerConfig {
    /// Explorer type.
    pub explorer_type: ExplorerType,
    /// API URL.
    pub api_url: Url,
    /// API key (optional for some explorers).
    pub api_key: Option<String>,
    /// Chain ID.
    pub chain_id: u64,
}

impl ExplorerConfig {
    /// Create a new explorer configuration.
    pub fn new(
        explorer_type: ExplorerType,
        api_url: Url,
        api_key: Option<String>,
        chain_id: u64,
    ) -> Self {
        Self {
            explorer_type,
            api_url,
            api_key,
            chain_id,
        }
    }

    /// Get default API URL for known explorers.
    ///
    /// For Etherscan, returns the unified V2 API URL which works for all chains.
    /// The chain is specified via the `chainid` query parameter at request time.
    pub fn default_api_url(explorer_type: ExplorerType, _chain_id: u64) -> Option<Url> {
        match explorer_type {
            // Etherscan V2 API - unified URL for all chains
            ExplorerType::Etherscan => Url::parse("https://api.etherscan.io/v2/api").ok(),
            // Blockscout and custom explorers require explicit URL
            ExplorerType::Blockscout | ExplorerType::Custom => None,
        }
    }
}

/// Explorer API client for checking verification status.
pub struct ExplorerClient {
    config: ExplorerConfig,
    http_client: reqwest::Client,
    logger: Arc<dyn Logger>,
}

impl ExplorerClient {
    /// Create a new explorer client.
    pub fn new(config: ExplorerConfig, logger: Arc<dyn Logger>) -> Self {
        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_default();

        Self {
            config,
            http_client,
            logger,
        }
    }

    /// Get the explorer config.
    pub fn config(&self) -> &ExplorerConfig {
        &self.config
    }

    /// Check if a contract is verified on the explorer.
    pub async fn check_verification_status(
        &self,
        address: Address,
    ) -> Result<VerificationStatus, VerificationError> {
        self.logger.debug(&format!(
            "Checking verification status for {} on {}",
            address, self.config.explorer_type
        ));

        // Build the API URL
        let mut url = self.config.api_url.clone();
        {
            let mut query = url.query_pairs_mut();
            // Chain ID required for Etherscan V2 API
            query.append_pair("chainid", &self.config.chain_id.to_string());
            query.append_pair("module", "contract");
            query.append_pair("action", "getabi");
            query.append_pair("address", &format!("{:?}", address));
            if let Some(ref api_key) = self.config.api_key {
                query.append_pair("apikey", api_key);
            }
        }

        // Execute with retry logic
        let mut last_error = None;
        for attempt in 0..MAX_RETRIES {
            if attempt > 0 {
                // Exponential backoff: 200ms, 400ms, 800ms
                let delay = DEFAULT_REQUEST_DELAY_MS * (1 << attempt);
                tokio::time::sleep(Duration::from_millis(delay)).await;
            }

            match self.execute_status_check(&url).await {
                Ok(status) => return Ok(status),
                Err(VerificationError::RateLimited) => {
                    self.logger.warning(&format!(
                        "Rate limited, retrying ({}/{})",
                        attempt + 1,
                        MAX_RETRIES
                    ));
                    last_error = Some(VerificationError::RateLimited);
                }
                Err(e) => return Err(e),
            }
        }

        Err(last_error.unwrap_or(VerificationError::RateLimited))
    }

    /// Execute the status check HTTP request.
    async fn execute_status_check(
        &self,
        url: &Url,
    ) -> Result<VerificationStatus, VerificationError> {
        self.logger.debug(&format!("Request URL: {}", url));
        let response = self.http_client.get(url.as_str()).send().await?;

        // Check for rate limiting
        if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
            return Err(VerificationError::RateLimited);
        }

        if !response.status().is_success() {
            return Err(VerificationError::ExplorerApi(format!(
                "HTTP {} from explorer",
                response.status()
            )));
        }

        let body = response.text().await?;
        self.logger.debug(&format!("Response body: {}", body));
        self.parse_verification_response(&body)
    }

    /// Parse the verification status response.
    fn parse_verification_response(
        &self,
        body: &str,
    ) -> Result<VerificationStatus, VerificationError> {
        // Parse as JSON
        let response: EtherscanResponse =
            serde_json::from_str(body).map_err(|e| VerificationError::JsonParse(e.to_string()))?;

        // Log parsed fields for debugging
        let result_preview = if response.result.len() > 100 {
            format!("{}...", &response.result[..100])
        } else {
            response.result.clone()
        };
        self.logger.debug(&format!(
            "Parsed response: status={}, message={}, result={}",
            response.status, response.message, result_preview
        ));

        // Etherscan/Blockscout use status "1" for success (verified)
        // and status "0" with message containing "not verified" for unverified
        match response.status.as_str() {
            "1" => Ok(VerificationStatus::Verified),
            "0" => {
                let message = response.message.to_lowercase();
                if message.contains("not verified") || message.contains("notok") {
                    Ok(VerificationStatus::NotVerified)
                } else if message.contains("pending") {
                    Ok(VerificationStatus::Pending)
                } else if message.contains("rate limit") {
                    Err(VerificationError::RateLimited)
                } else {
                    Ok(VerificationStatus::Unknown(response.message))
                }
            }
            _ => Ok(VerificationStatus::Unknown(format!(
                "Unexpected status: {}",
                response.status
            ))),
        }
    }

    /// Check verification submission status by GUID.
    pub async fn check_submission_status(
        &self,
        guid: &str,
    ) -> Result<VerificationStatus, VerificationError> {
        self.logger.debug(&format!(
            "Checking submission status for GUID {} on {}",
            guid, self.config.explorer_type
        ));

        // Build the API URL
        let mut url = self.config.api_url.clone();
        {
            let mut query = url.query_pairs_mut();
            // Chain ID required for Etherscan V2 API
            query.append_pair("chainid", &self.config.chain_id.to_string());
            query.append_pair("module", "contract");
            query.append_pair("action", "checkverifystatus");
            query.append_pair("guid", guid);
            if let Some(ref api_key) = self.config.api_key {
                query.append_pair("apikey", api_key);
            }
        }

        self.logger.debug(&format!("Request URL: {}", url));
        let response = self.http_client.get(url.as_str()).send().await?;

        if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
            return Err(VerificationError::RateLimited);
        }

        let body = response.text().await?;
        self.logger
            .debug(&format!("Submission status response: {}", body));
        let parsed: EtherscanResponse =
            serde_json::from_str(&body).map_err(|e| VerificationError::JsonParse(e.to_string()))?;

        self.logger.debug(&format!(
            "Parsed submission response: status={}, message={}, result={}",
            parsed.status, parsed.message, parsed.result
        ));

        match parsed.status.as_str() {
            "1" => Ok(VerificationStatus::Verified),
            "0" => {
                let result = parsed.result.to_lowercase();
                if result.contains("pending") {
                    Ok(VerificationStatus::Pending)
                } else if result.contains("fail") {
                    Ok(VerificationStatus::Unknown(parsed.result))
                } else {
                    Ok(VerificationStatus::NotVerified)
                }
            }
            _ => Ok(VerificationStatus::Unknown(parsed.message)),
        }
    }

    /// Add a delay between requests to avoid rate limiting.
    pub async fn rate_limit_delay(&self) {
        tokio::time::sleep(Duration::from_millis(DEFAULT_REQUEST_DELAY_MS)).await;
    }
}

/// Etherscan/Blockscout API response structure.
#[derive(Debug, Deserialize)]
struct EtherscanResponse {
    status: String,
    message: String,
    #[serde(default)]
    result: String,
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use adi_types::NoopLogger;

    #[test]
    fn test_default_api_url() {
        // V2 API uses unified URL for all chains
        let url = ExplorerConfig::default_api_url(ExplorerType::Etherscan, 1);
        assert!(url.is_some());
        let url_str = url.unwrap();
        assert!(url_str.as_str().contains("api.etherscan.io/v2/api"));

        // Same V2 URL for other chains (chain specified via chainid param)
        let url = ExplorerConfig::default_api_url(ExplorerType::Etherscan, 11155111);
        assert!(url.is_some());
        assert!(url.unwrap().as_str().contains("api.etherscan.io/v2/api"));

        // Custom explorer requires explicit URL
        let url = ExplorerConfig::default_api_url(ExplorerType::Custom, 1);
        assert!(url.is_none());

        // Blockscout requires explicit URL
        let url = ExplorerConfig::default_api_url(ExplorerType::Blockscout, 1);
        assert!(url.is_none());
    }

    #[test]
    fn test_parse_verified_response() {
        let config = ExplorerConfig {
            explorer_type: ExplorerType::Etherscan,
            api_url: Url::parse("https://api.example.com").unwrap(),
            api_key: None,
            chain_id: 1,
        };
        let client = ExplorerClient::new(config, Arc::new(NoopLogger));

        let response = r#"{"status":"1","message":"OK","result":"[{\"inputs\":[]}]"}"#;
        let status = client.parse_verification_response(response).unwrap();
        assert_eq!(status, VerificationStatus::Verified);

        let response =
            r#"{"status":"0","message":"Contract source code not verified","result":""}"#;
        let status = client.parse_verification_response(response).unwrap();
        assert_eq!(status, VerificationStatus::NotVerified);
    }
}
