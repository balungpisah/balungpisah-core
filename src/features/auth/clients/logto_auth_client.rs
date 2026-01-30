use crate::core::error::{AppError, Result};
use crate::features::logto::token_manager::LogtoTokenManager;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Logto user response from Management API
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogtoUserResponse {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub primary_email: Option<String>,
    #[serde(default)]
    pub primary_email_verified: bool,
    #[serde(default)]
    pub is_suspended: bool,
}

/// Request to create a new user in Logto
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CreateUserRequest {
    primary_email: String,
    password: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    username: Option<String>,
}

/// Request to verify user password
#[derive(Debug, Serialize)]
struct VerifyPasswordRequest {
    password: String,
}

/// Logto error response
#[derive(Debug, Deserialize)]
struct LogtoErrorResponse {
    #[serde(default)]
    message: String,
    #[serde(default)]
    code: String,
}

/// Client for Logto Management API auth operations
pub struct LogtoAuthClient {
    token_manager: Arc<LogtoTokenManager>,
    http_client: reqwest::Client,
}

impl LogtoAuthClient {
    pub fn new(token_manager: Arc<LogtoTokenManager>) -> Self {
        Self {
            token_manager,
            http_client: reqwest::Client::new(),
        }
    }

    /// Create a new user in Logto
    ///
    /// Returns Conflict error if email already exists
    pub async fn create_user(
        &self,
        email: &str,
        password: &str,
        username: Option<&str>,
    ) -> Result<LogtoUserResponse> {
        let token_response = self.token_manager.get_access_token().await.map_err(|e| {
            AppError::ExternalServiceError(format!("Failed to get M2M token: {}", e))
        })?;

        let url = format!("{}/api/users", self.token_manager.api_base_url());

        let request_body = CreateUserRequest {
            primary_email: email.to_string(),
            password: password.to_string(),
            username: username.map(String::from),
        };

        tracing::debug!("Creating user in Logto: {}", email);

        let response = self
            .http_client
            .post(&url)
            .bearer_auth(&token_response.access_token)
            .json(&request_body)
            .send()
            .await
            .map_err(|e| {
                tracing::error!("Failed to create user in Logto: {}", e);
                AppError::ExternalServiceError(format!("Failed to create user: {}", e))
            })?;

        let status = response.status();

        if status.is_success() {
            let user = response.json::<LogtoUserResponse>().await.map_err(|e| {
                tracing::error!("Failed to parse Logto user response: {}", e);
                AppError::ExternalServiceError(format!("Failed to parse user response: {}", e))
            })?;

            tracing::info!("Successfully created user: {}", user.id);
            return Ok(user);
        }

        // Handle error responses
        let body = response.text().await.unwrap_or_default();

        // Check for duplicate email (HTTP 422 with user.email.exists code)
        if status.as_u16() == 422 {
            if let Ok(error_response) = serde_json::from_str::<LogtoErrorResponse>(&body) {
                if error_response.code == "user.email.exists"
                    || error_response.message.contains("already")
                {
                    return Err(AppError::Conflict("Email already registered".to_string()));
                }
            }
            // Generic 422 error
            return Err(AppError::Validation(format!("Invalid request: {}", body)));
        }

        tracing::error!("Logto API error: HTTP {} - {}", status, body);
        Err(AppError::ExternalServiceError(format!(
            "Logto API error: HTTP {}",
            status
        )))
    }

    /// Find user by email
    ///
    /// Returns None if user not found
    pub async fn find_user_by_email(&self, email: &str) -> Result<Option<LogtoUserResponse>> {
        let token_response = self.token_manager.get_access_token().await.map_err(|e| {
            AppError::ExternalServiceError(format!("Failed to get M2M token: {}", e))
        })?;

        // Use search endpoint with email filter
        let url = format!(
            "{}/api/users?search={}",
            self.token_manager.api_base_url(),
            urlencoding::encode(email)
        );

        tracing::debug!("Searching for user by email: {}", email);

        let response = self
            .http_client
            .get(&url)
            .bearer_auth(&token_response.access_token)
            .send()
            .await
            .map_err(|e| {
                tracing::error!("Failed to search users in Logto: {}", e);
                AppError::ExternalServiceError(format!("Failed to search users: {}", e))
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            tracing::error!("Logto API error: HTTP {} - {}", status, body);
            return Err(AppError::ExternalServiceError(format!(
                "Logto API error: HTTP {}",
                status
            )));
        }

        let users = response
            .json::<Vec<LogtoUserResponse>>()
            .await
            .map_err(|e| {
                tracing::error!("Failed to parse users response: {}", e);
                AppError::ExternalServiceError(format!("Failed to parse users response: {}", e))
            })?;

        // Find exact email match (search may return partial matches)
        let user = users
            .into_iter()
            .find(|u| u.primary_email.as_deref() == Some(email));

        Ok(user)
    }

    /// Verify user password
    ///
    /// Returns true if password is correct, false otherwise
    pub async fn verify_password(&self, user_id: &str, password: &str) -> Result<bool> {
        let token_response = self.token_manager.get_access_token().await.map_err(|e| {
            AppError::ExternalServiceError(format!("Failed to get M2M token: {}", e))
        })?;

        let url = format!(
            "{}/api/users/{}/password/verify",
            self.token_manager.api_base_url(),
            user_id
        );

        let request_body = VerifyPasswordRequest {
            password: password.to_string(),
        };

        tracing::debug!("Verifying password for user: {}", user_id);

        let response = self
            .http_client
            .post(&url)
            .bearer_auth(&token_response.access_token)
            .json(&request_body)
            .send()
            .await
            .map_err(|e| {
                tracing::error!("Failed to verify password: {}", e);
                AppError::ExternalServiceError(format!("Failed to verify password: {}", e))
            })?;

        let status = response.status();

        // 204 No Content = password correct
        if status.as_u16() == 204 {
            return Ok(true);
        }

        // 422 Unprocessable Entity = password incorrect
        if status.as_u16() == 422 {
            return Ok(false);
        }

        // Other errors
        let body = response.text().await.unwrap_or_default();
        tracing::error!("Logto API error: HTTP {} - {}", status, body);
        Err(AppError::ExternalServiceError(format!(
            "Logto API error: HTTP {}",
            status
        )))
    }

    /// Trigger email verification for user (async, non-blocking)
    ///
    /// This is best-effort - errors are logged but not propagated
    pub async fn trigger_email_verification(&self, user_id: &str, email: &str) {
        if let Err(e) = self.send_verification_code(user_id, email).await {
            tracing::warn!(
                "Failed to send email verification for user {}: {}",
                user_id,
                e
            );
        }
    }

    async fn send_verification_code(&self, _user_id: &str, email: &str) -> Result<()> {
        let token_response = self.token_manager.get_access_token().await.map_err(|e| {
            AppError::ExternalServiceError(format!("Failed to get M2M token: {}", e))
        })?;

        let url = format!(
            "{}/api/verification-codes",
            self.token_manager.api_base_url()
        );

        let request_body = serde_json::json!({
            "email": email,
            "action": "verify_email"
        });

        let response = self
            .http_client
            .post(&url)
            .bearer_auth(&token_response.access_token)
            .json(&request_body)
            .send()
            .await
            .map_err(|e| {
                AppError::ExternalServiceError(format!("Failed to send verification: {}", e))
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            tracing::warn!(
                "Failed to send verification code: HTTP {} - {}",
                status,
                body
            );
        } else {
            tracing::info!("Verification email sent to: {}", email);
        }

        Ok(())
    }
}
