use crate::core::error::{AppError, Result};
use crate::core::extractor::AppJson;
use crate::features::auth::dto::MeResponseDto;
use crate::features::auth::dtos::{
    AuthResponseDto, LoginRequestDto, RefreshTokenRequestDto, RefreshTokenResponseDto,
    RegisterRequestDto,
};
use crate::features::auth::model::AuthenticatedUser;
use crate::features::auth::services::AuthService;
use crate::shared::types::ApiResponse;
use axum::{extract::State, http::StatusCode, Json};
use std::sync::Arc;
use validator::Validate;

/// Register a new user
#[utoipa::path(
    post,
    path = "/api/auth/register",
    request_body = RegisterRequestDto,
    responses(
        (status = 201, description = "User registered successfully", body = ApiResponse<AuthResponseDto>),
        (status = 400, description = "Validation error"),
        (status = 409, description = "Email already registered")
    ),
    tag = "auth"
)]
pub async fn register(
    State(service): State<Arc<AuthService>>,
    AppJson(dto): AppJson<RegisterRequestDto>,
) -> Result<(StatusCode, Json<ApiResponse<AuthResponseDto>>)> {
    dto.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let auth_response = service.register(dto).await?;
    Ok((
        StatusCode::CREATED,
        Json(ApiResponse::success(Some(auth_response), None, None)),
    ))
}

/// Login with email and password
#[utoipa::path(
    post,
    path = "/api/auth/login",
    request_body = LoginRequestDto,
    responses(
        (status = 200, description = "Login successful", body = ApiResponse<AuthResponseDto>),
        (status = 400, description = "Validation error"),
        (status = 401, description = "Invalid credentials"),
        (status = 403, description = "Account suspended"),
        (status = 404, description = "User not found")
    ),
    tag = "auth"
)]
pub async fn login(
    State(service): State<Arc<AuthService>>,
    AppJson(dto): AppJson<LoginRequestDto>,
) -> Result<Json<ApiResponse<AuthResponseDto>>> {
    dto.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let auth_response = service.login(dto).await?;
    Ok(Json(ApiResponse::success(Some(auth_response), None, None)))
}

/// Get current authenticated user info
#[utoipa::path(
    get,
    path = "/api/auth/me",
    responses(
        (status = 200, description = "Current user retrieved successfully", body = ApiResponse<MeResponseDto>),
        (status = 401, description = "Unauthorized")
    ),
    tag = "auth",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn get_me(
    user: AuthenticatedUser,
    State(service): State<Arc<AuthService>>,
) -> Result<Json<ApiResponse<MeResponseDto>>> {
    let user_data = service.get_current_user(user).await?;
    Ok(Json(ApiResponse::success(Some(user_data), None, None)))
}

/// Refresh access token using refresh token
#[utoipa::path(
    post,
    path = "/api/auth/refresh",
    request_body = RefreshTokenRequestDto,
    responses(
        (status = 200, description = "Token refreshed successfully", body = ApiResponse<RefreshTokenResponseDto>),
        (status = 401, description = "Invalid or expired refresh token")
    ),
    tag = "auth"
)]
pub async fn refresh_token(
    State(service): State<Arc<AuthService>>,
    AppJson(dto): AppJson<RefreshTokenRequestDto>,
) -> Result<Json<ApiResponse<RefreshTokenResponseDto>>> {
    dto.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let response = service.refresh_token(dto).await?;
    Ok(Json(ApiResponse::success(Some(response), None, None)))
}
