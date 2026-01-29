use crate::core::config::LogtoM2MConfig;
use serde::Deserialize;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Response from Logto token endpoint
#[derive(Debug, Clone, Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub expires_in: u64,
    #[serde(rename = "token_type")]
    pub _token_type: String,
    #[serde(default)]
    _scope: String,
}

/// Cached token with expiration tracking
struct TokenCache {
    token: TokenResponse,
    fetched_at: Instant,
}

/// Manages Logto M2M access tokens with caching
pub struct LogtoTokenManager {
    config: LogtoM2MConfig,
    client: reqwest::Client,
    cache: Arc<RwLock<Option<TokenCache>>>,
    /// Refresh token this many seconds before expiration
    refresh_margin: Duration,
}

impl LogtoTokenManager {
    pub fn new(config: LogtoM2MConfig) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
            cache: Arc::new(RwLock::new(None)),
            refresh_margin: Duration::from_secs(60),
        }
    }

    /// Get a valid access token, fetching a new one if necessary
    pub async fn get_access_token(&self) -> Result<TokenResponse, TokenError> {
        // Try to get from cache first
        {
            let cache = self.cache.read().await;
            if let Some(ref cached) = *cache {
                let elapsed = cached.fetched_at.elapsed();
                let expires_in = Duration::from_secs(cached.token.expires_in);

                // Return cached token if not expired (with margin)
                if elapsed + self.refresh_margin < expires_in {
                    tracing::debug!(
                        "Using cached Logto M2M token (expires in {} seconds)",
                        (expires_in - elapsed).as_secs()
                    );
                    return Ok(cached.token.clone());
                }
            }
        }

        // Cache miss or near expiration - fetch new token
        self.fetch_token().await
    }

    /// Fetch a new token from Logto
    async fn fetch_token(&self) -> Result<TokenResponse, TokenError> {
        tracing::debug!(
            "Fetching new Logto M2M token from {}",
            self.config.token_url
        );

        let response = self
            .client
            .post(&self.config.token_url)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .form(&[
                ("grant_type", "client_credentials"),
                ("client_id", &self.config.client_id),
                ("client_secret", &self.config.client_secret),
                ("scope", &self.config.scope),
                ("resource", &self.config.resource),
            ])
            .send()
            .await
            .map_err(|e| TokenError::FetchError(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(TokenError::FetchError(format!(
                "Token request failed: HTTP {} - {}",
                status, body
            )));
        }

        let token_response: TokenResponse = response
            .json()
            .await
            .map_err(|e| TokenError::ParseError(e.to_string()))?;

        tracing::info!(
            "Fetched new Logto M2M token, expires in {} seconds",
            token_response.expires_in
        );

        // Update cache
        let mut cache = self.cache.write().await;
        *cache = Some(TokenCache {
            token: token_response.clone(),
            fetched_at: Instant::now(),
        });

        Ok(token_response)
    }

    /// Get the API base URL for Logto Management API
    pub fn api_base_url(&self) -> &str {
        &self.config.api_base_url
    }
}

#[derive(Debug, thiserror::Error)]
pub enum TokenError {
    #[error("Failed to fetch token: {0}")]
    FetchError(String),

    #[error("Failed to parse token response: {0}")]
    ParseError(String),
}
