use crate::core::error::{AppError, Result};
use crate::features::logto::token_manager::LogtoTokenManager;
use crate::features::users::dtos::{ExtendedProfileDto, UserProfileResponseDto};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Logto user from Management API
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogtoUser {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub primary_email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub primary_phone: Option<String>,
    #[serde(default)]
    pub profile: LogtoUserProfile,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<i64>,
}

/// OIDC standard profile fields
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogtoUserProfile {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub given_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub family_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub middle_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nickname: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preferred_username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub website: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gender: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub birthdate: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub zoneinfo: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locale: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address: Option<LogtoAddress>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogtoAddress {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub formatted: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub street_address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locality: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub postal_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,
}

/// Request to update basic user fields
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateUserRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
}

/// Request to update user profile
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateUserProfileRequest {
    pub profile: LogtoUserProfileUpdate,
}

/// Updateable profile fields
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogtoUserProfileUpdate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub given_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub family_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nickname: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub birthdate: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locale: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub zoneinfo: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gender: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub website: Option<String>,
}

/// Client for Logto User Management API
pub struct LogtoUserProfileClient {
    token_manager: Arc<LogtoTokenManager>,
    http_client: reqwest::Client,
}

impl LogtoUserProfileClient {
    pub fn new(token_manager: Arc<LogtoTokenManager>) -> Self {
        Self {
            token_manager,
            http_client: reqwest::Client::new(),
        }
    }

    /// Get user by ID
    pub async fn get_user(&self, user_id: &str) -> Result<LogtoUser> {
        let token_response = self
            .token_manager
            .get_access_token()
            .await
            .map_err(|e| AppError::Internal(format!("Failed to get access token: {}", e)))?;

        let url = format!(
            "{}/api/users/{}",
            self.token_manager.api_base_url(),
            user_id
        );

        tracing::debug!("Fetching user from Logto: {}", url);

        let response = self
            .http_client
            .get(&url)
            .bearer_auth(&token_response.access_token)
            .send()
            .await
            .map_err(|e| {
                tracing::error!("Failed to fetch user: {}", e);
                AppError::Internal(format!("Failed to fetch user from Logto: {}", e))
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            tracing::error!("Logto API error: HTTP {} - {}", status, body);
            return Err(AppError::Internal(format!(
                "Logto API error: HTTP {} - {}",
                status, body
            )));
        }

        let user = response.json::<LogtoUser>().await.map_err(|e| {
            tracing::error!("Failed to parse user response: {}", e);
            AppError::Internal(format!("Failed to parse user response: {}", e))
        })?;

        Ok(user)
    }

    /// Update basic user fields (name, avatar, username)
    pub async fn update_user(
        &self,
        user_id: &str,
        request: UpdateUserRequest,
    ) -> Result<LogtoUser> {
        let token_response = self
            .token_manager
            .get_access_token()
            .await
            .map_err(|e| AppError::Internal(format!("Failed to get access token: {}", e)))?;

        let url = format!(
            "{}/api/users/{}",
            self.token_manager.api_base_url(),
            user_id
        );

        tracing::debug!("Updating user in Logto: {}", url);

        let response = self
            .http_client
            .patch(&url)
            .bearer_auth(&token_response.access_token)
            .json(&request)
            .send()
            .await
            .map_err(|e| {
                tracing::error!("Failed to update user: {}", e);
                AppError::Internal(format!("Failed to update user in Logto: {}", e))
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            tracing::error!("Logto API error: HTTP {} - {}", status, body);
            return Err(AppError::Internal(format!(
                "Logto API error: HTTP {} - {}",
                status, body
            )));
        }

        let user = response.json::<LogtoUser>().await.map_err(|e| {
            tracing::error!("Failed to parse user response: {}", e);
            AppError::Internal(format!("Failed to parse user response: {}", e))
        })?;

        tracing::info!("Successfully updated user: {}", user_id);

        Ok(user)
    }

    /// Update user profile (extended OIDC fields)
    pub async fn update_user_profile(
        &self,
        user_id: &str,
        request: UpdateUserProfileRequest,
    ) -> Result<LogtoUserProfile> {
        let token_response = self
            .token_manager
            .get_access_token()
            .await
            .map_err(|e| AppError::Internal(format!("Failed to get access token: {}", e)))?;

        let url = format!(
            "{}/api/users/{}/profile",
            self.token_manager.api_base_url(),
            user_id
        );

        tracing::debug!("Updating user profile in Logto: {}", url);

        let response = self
            .http_client
            .patch(&url)
            .bearer_auth(&token_response.access_token)
            .json(&request)
            .send()
            .await
            .map_err(|e| {
                tracing::error!("Failed to update user profile: {}", e);
                AppError::Internal(format!("Failed to update user profile in Logto: {}", e))
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            tracing::error!("Logto API error: HTTP {} - {}", status, body);
            return Err(AppError::Internal(format!(
                "Logto API error: HTTP {} - {}",
                status, body
            )));
        }

        let profile = response.json::<LogtoUserProfile>().await.map_err(|e| {
            tracing::error!("Failed to parse profile response: {}", e);
            AppError::Internal(format!("Failed to parse profile response: {}", e))
        })?;

        tracing::info!("Successfully updated user profile: {}", user_id);

        Ok(profile)
    }
}

// Conversion from LogtoUserProfile to ExtendedProfileDto
impl From<LogtoUserProfile> for ExtendedProfileDto {
    fn from(profile: LogtoUserProfile) -> Self {
        Self {
            given_name: profile.given_name,
            family_name: profile.family_name,
            nickname: profile.nickname,
            birthdate: profile.birthdate,
            locale: profile.locale,
            zoneinfo: profile.zoneinfo,
            gender: profile.gender,
            website: profile.website,
        }
    }
}

// Conversion from LogtoUser to UserProfileResponseDto (roles will be added separately)
impl From<LogtoUser> for UserProfileResponseDto {
    fn from(user: LogtoUser) -> Self {
        Self {
            id: user.id,
            username: user.username,
            name: user.name,
            avatar: user.avatar,
            primary_email: user.primary_email,
            primary_phone: user.primary_phone,
            profile: user.profile.into(),
            created_at: user.created_at,
            updated_at: user.updated_at,
            roles: Vec::new(), // Will be populated from JWT
        }
    }
}
