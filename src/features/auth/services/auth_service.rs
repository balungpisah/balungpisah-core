use crate::core::error::{AppError, Result};
use crate::features::auth::clients::{LogtoAuthClient, LogtoUserResponse};
use crate::features::auth::dto::MeResponseDto;
use crate::features::auth::dtos::{
    AuthResponseDto, AuthUserDto, LoginRequestDto, RefreshTokenRequestDto, RefreshTokenResponseDto,
    RegisterRequestDto,
};
use crate::features::auth::model::AuthenticatedUser;
use crate::features::auth::services::token_service::TokenService;
use std::sync::Arc;

/// Service for authentication operations (register, login)
pub struct AuthService {
    logto_client: Arc<LogtoAuthClient>,
    token_service: Arc<TokenService>,
}

impl AuthService {
    pub fn new(logto_client: Arc<LogtoAuthClient>, token_service: Arc<TokenService>) -> Self {
        Self {
            logto_client,
            token_service,
        }
    }

    /// Register a new user
    pub async fn register(&self, dto: RegisterRequestDto) -> Result<AuthResponseDto> {
        // Create user in Logto
        let user = self
            .logto_client
            .create_user(&dto.email, &dto.password, dto.username.as_deref())
            .await?;

        // Trigger email verification (async, non-blocking)
        let email = dto.email.clone();
        let user_id = user.id.clone();
        let logto_client = Arc::clone(&self.logto_client);
        tokio::spawn(async move {
            logto_client
                .trigger_email_verification(&user_id, &email)
                .await;
        });

        // Create Logto access token via token exchange
        let token_response = self.token_service.create_token(&user.id, None).await?;

        Ok(AuthResponseDto {
            access_token: token_response.access_token,
            token_type: token_response.token_type,
            expires_in: token_response.expires_in,
            refresh_token: token_response.refresh_token,
            user: user_to_auth_user_dto(user),
        })
    }

    /// Login with email and password
    pub async fn login(&self, dto: LoginRequestDto) -> Result<AuthResponseDto> {
        // Find user by email
        let user = self
            .logto_client
            .find_user_by_email(&dto.email)
            .await?
            .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

        // Check if user is suspended
        if user.is_suspended {
            return Err(AppError::Forbidden("Account is suspended".to_string()));
        }

        // Verify password
        let password_valid = self
            .logto_client
            .verify_password(&user.id, &dto.password)
            .await?;

        if !password_valid {
            return Err(AppError::Unauthorized("Invalid credentials".to_string()));
        }

        // Create Logto access token via token exchange
        let token_response = self.token_service.create_token(&user.id, None).await?;

        Ok(AuthResponseDto {
            access_token: token_response.access_token,
            token_type: token_response.token_type,
            expires_in: token_response.expires_in,
            refresh_token: token_response.refresh_token,
            user: user_to_auth_user_dto(user),
        })
    }

    /// Get current user info (for /me endpoint)
    pub async fn get_current_user(&self, user: AuthenticatedUser) -> Result<MeResponseDto> {
        Ok(user.into())
    }

    /// Refresh access token using refresh token
    pub async fn refresh_token(
        &self,
        dto: RefreshTokenRequestDto,
    ) -> Result<RefreshTokenResponseDto> {
        let token_response = self.token_service.refresh_token(&dto.refresh_token).await?;

        Ok(RefreshTokenResponseDto {
            access_token: token_response.access_token,
            token_type: token_response.token_type,
            expires_in: token_response.expires_in,
            refresh_token: token_response.refresh_token,
        })
    }
}

/// Convert Logto user response to auth user DTO
fn user_to_auth_user_dto(user: LogtoUserResponse) -> AuthUserDto {
    AuthUserDto {
        id: user.id,
        username: user.username,
        name: user.name,
        email: user.primary_email,
        email_verified: user.primary_email_verified,
        avatar: user.avatar,
    }
}
