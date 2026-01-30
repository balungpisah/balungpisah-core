use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

/// Request DTO for user registration
#[derive(Debug, Serialize, Deserialize, Validate, ToSchema)]
pub struct RegisterRequestDto {
    #[validate(email(message = "Invalid email format"))]
    pub email: String,

    #[validate(length(min = 8, message = "Password must be at least 8 characters"))]
    pub password: String,

    #[validate(length(min = 1, max = 50, message = "Username must be 1-50 characters"))]
    pub username: Option<String>,
}

/// Request DTO for user login
#[derive(Debug, Serialize, Deserialize, Validate, ToSchema)]
pub struct LoginRequestDto {
    #[validate(email(message = "Invalid email format"))]
    pub email: String,

    #[validate(length(min = 1, message = "Password is required"))]
    pub password: String,
}

/// Request DTO for token refresh
#[derive(Debug, Serialize, Deserialize, Validate, ToSchema)]
pub struct RefreshTokenRequestDto {
    #[validate(length(min = 1, message = "Refresh token is required"))]
    pub refresh_token: String,
}

/// Response DTO for token refresh (same structure as auth response but without user info)
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct RefreshTokenResponseDto {
    /// New JWT access token
    pub access_token: String,
    /// Token type (always "Bearer")
    pub token_type: String,
    /// Token expiry time in seconds
    pub expires_in: i64,
    /// New refresh token (if rotated)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,
}

/// Response DTO for authentication (register/login)
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AuthResponseDto {
    /// JWT access token (Logto OIDC token)
    pub access_token: String,
    /// Token type (always "Bearer")
    pub token_type: String,
    /// Token expiry time in seconds
    pub expires_in: i64,
    /// Refresh token for obtaining new access tokens (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,
    /// Authenticated user info
    pub user: AuthUserDto,
}

/// User info included in auth response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AuthUserDto {
    /// Logto user ID
    pub id: String,
    /// Username (optional)
    pub username: Option<String>,
    /// Display name (optional)
    pub name: Option<String>,
    /// Email address (optional, may be null if not verified)
    pub email: Option<String>,
    /// Whether email is verified
    pub email_verified: bool,
    /// Avatar URL (optional)
    pub avatar: Option<String>,
}
