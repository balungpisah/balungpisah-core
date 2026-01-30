use crate::core::config::AuthTokenConfig;
use crate::core::error::{AppError, Result};
use crate::features::logto::token_manager::LogtoTokenManager;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Response from Logto subject token creation
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubjectTokenResponse {
    pub subject_token: String,
    pub expires_in: u64,
}

/// Response from Logto token exchange
#[derive(Debug, Deserialize)]
pub struct TokenExchangeResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: i64,
    #[serde(default)]
    pub _scope: String,
    pub refresh_token: Option<String>,
}

/// Request body for subject token creation
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CreateSubjectTokenRequest {
    user_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    context: Option<serde_json::Value>,
}

/// Service for creating Logto OIDC tokens via subject token exchange
pub struct TokenService {
    config: AuthTokenConfig,
    token_manager: Arc<LogtoTokenManager>,
    http_client: reqwest::Client,
}

impl TokenService {
    pub fn new(config: AuthTokenConfig, token_manager: Arc<LogtoTokenManager>) -> Self {
        Self {
            config,
            token_manager,
            http_client: reqwest::Client::new(),
        }
    }

    /// Create a Logto access token for the given user via subject token exchange
    ///
    /// Flow:
    /// 1. Create subject token via Management API
    /// 2. Exchange subject token for access token via OIDC token endpoint
    ///
    /// Returns (access_token, expires_in_seconds, refresh_token)
    pub async fn create_token(
        &self,
        user_id: &str,
        context: Option<serde_json::Value>,
    ) -> Result<TokenExchangeResponse> {
        // Step 1: Create subject token
        let subject_token = self.create_subject_token(user_id, context).await?;

        // Step 2: Exchange subject token for access token
        let token_response = self.exchange_subject_token(&subject_token).await?;

        Ok(token_response)
    }

    /// Create a subject token for the user via Logto Management API
    async fn create_subject_token(
        &self,
        user_id: &str,
        context: Option<serde_json::Value>,
    ) -> Result<String> {
        let m2m_token = self.token_manager.get_access_token().await.map_err(|e| {
            AppError::ExternalServiceError(format!("Failed to get M2M token: {}", e))
        })?;

        let url = format!("{}/api/subject-tokens", self.token_manager.api_base_url());

        let request_body = CreateSubjectTokenRequest {
            user_id: user_id.to_string(),
            context,
        };

        tracing::debug!("Creating subject token for user: {}", user_id);

        let response = self
            .http_client
            .post(&url)
            .bearer_auth(&m2m_token.access_token)
            .json(&request_body)
            .send()
            .await
            .map_err(|e| {
                tracing::error!("Failed to create subject token: {}", e);
                AppError::ExternalServiceError(format!("Failed to create subject token: {}", e))
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            tracing::error!(
                "Logto API error creating subject token: HTTP {} - {}",
                status,
                body
            );
            return Err(AppError::ExternalServiceError(format!(
                "Failed to create subject token: HTTP {}",
                status
            )));
        }

        let subject_token_response: SubjectTokenResponse = response.json().await.map_err(|e| {
            tracing::error!("Failed to parse subject token response: {}", e);
            AppError::ExternalServiceError(format!("Failed to parse subject token response: {}", e))
        })?;

        tracing::debug!(
            "Subject token created, expires in {} seconds",
            subject_token_response.expires_in
        );

        Ok(subject_token_response.subject_token)
    }

    /// Exchange subject token for access token via OIDC token endpoint
    async fn exchange_subject_token(&self, subject_token: &str) -> Result<TokenExchangeResponse> {
        // Build Basic auth header
        let credentials = format!(
            "{}:{}",
            self.config.token_exchange_app_id, self.config.token_exchange_app_secret
        );
        let auth_header = format!("Basic {}", BASE64.encode(credentials.as_bytes()));

        // Build form body for token exchange
        let form_body = [
            (
                "grant_type",
                "urn:ietf:params:oauth:grant-type:token-exchange",
            ),
            ("subject_token", subject_token),
            (
                "subject_token_type",
                "urn:ietf:params:oauth:token-type:access_token",
            ),
            ("resource", &self.config.api_resource),
            ("scope", &self.config.token_scopes),
        ];

        tracing::debug!(
            "Exchanging subject token for access token with scopes: {}",
            &self.config.token_scopes
        );

        let response = self
            .http_client
            .post(&self.config.oidc_token_url)
            .header("Authorization", auth_header)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .form(&form_body)
            .send()
            .await
            .map_err(|e| {
                tracing::error!("Failed to exchange subject token: {}", e);
                AppError::ExternalServiceError(format!("Failed to exchange subject token: {}", e))
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            tracing::error!("Token exchange failed: HTTP {} - {}", status, body);
            return Err(AppError::ExternalServiceError(format!(
                "Token exchange failed: HTTP {} - {}",
                status, body
            )));
        }

        let token_response: TokenExchangeResponse = response.json().await.map_err(|e| {
            tracing::error!("Failed to parse token exchange response: {}", e);
            AppError::ExternalServiceError(format!(
                "Failed to parse token exchange response: {}",
                e
            ))
        })?;

        tracing::info!(
            "Token exchange successful, expires in {} seconds, refresh_token present: {}",
            token_response.expires_in,
            token_response.refresh_token.is_some()
        );

        Ok(token_response)
    }

    /// Refresh an access token using a refresh token
    ///
    /// Uses the standard OAuth2 refresh_token grant type
    pub async fn refresh_token(&self, refresh_token: &str) -> Result<TokenExchangeResponse> {
        // Build Basic auth header
        let credentials = format!(
            "{}:{}",
            self.config.token_exchange_app_id, self.config.token_exchange_app_secret
        );
        let auth_header = format!("Basic {}", BASE64.encode(credentials.as_bytes()));

        // Build form body for refresh token
        let form_body = [
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
            ("resource", &self.config.api_resource),
            ("scope", &self.config.token_scopes),
        ];

        tracing::debug!("Refreshing access token");

        let response = self
            .http_client
            .post(&self.config.oidc_token_url)
            .header("Authorization", auth_header)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .form(&form_body)
            .send()
            .await
            .map_err(|e| {
                tracing::error!("Failed to refresh token: {}", e);
                AppError::ExternalServiceError(format!("Failed to refresh token: {}", e))
            })?;

        let status = response.status();

        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            tracing::error!("Token refresh failed: HTTP {} - {}", status, body);

            // Handle specific error cases
            if status.as_u16() == 400 || status.as_u16() == 401 {
                // Invalid or expired refresh token
                return Err(AppError::Unauthorized(
                    "Invalid or expired refresh token".to_string(),
                ));
            }

            return Err(AppError::ExternalServiceError(format!(
                "Token refresh failed: HTTP {}",
                status
            )));
        }

        let token_response: TokenExchangeResponse = response.json().await.map_err(|e| {
            tracing::error!("Failed to parse refresh token response: {}", e);
            AppError::ExternalServiceError(format!("Failed to parse refresh token response: {}", e))
        })?;

        tracing::info!(
            "Token refresh successful, expires in {} seconds",
            token_response.expires_in
        );

        Ok(token_response)
    }
}
