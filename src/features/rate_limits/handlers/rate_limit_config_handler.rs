use std::sync::Arc;

use axum::{
    extract::{Path, State},
    Json,
};
use validator::Validate;

use crate::core::error::{AppError, Result};
use crate::features::auth::guards::RequireSuperAdmin;
use crate::features::rate_limits::dtos::{RateLimitConfigResponseDto, UpdateRateLimitConfigDto};
use crate::features::rate_limits::services::RateLimitConfigService;
use crate::shared::types::ApiResponse;

/// List all rate limit configurations
#[utoipa::path(
    get,
    path = "/api/admin/rate-limits",
    responses(
        (status = 200, description = "List of rate limit configurations", body = ApiResponse<Vec<RateLimitConfigResponseDto>>),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - Super admin access required")
    ),
    tag = "rate-limits",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn list_rate_limit_configs(
    RequireSuperAdmin(_user): RequireSuperAdmin,
    State(service): State<Arc<RateLimitConfigService>>,
) -> Result<Json<ApiResponse<Vec<RateLimitConfigResponseDto>>>> {
    let configs = service.list_all().await?;
    let response: Vec<RateLimitConfigResponseDto> = configs.into_iter().map(|c| c.into()).collect();

    Ok(Json(ApiResponse::success(Some(response), None, None)))
}

/// Get a specific rate limit configuration by key
#[utoipa::path(
    get,
    path = "/api/admin/rate-limits/{key}",
    params(
        ("key" = String, Path, description = "Configuration key")
    ),
    responses(
        (status = 200, description = "Rate limit configuration", body = ApiResponse<RateLimitConfigResponseDto>),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - Super admin access required"),
        (status = 404, description = "Configuration not found")
    ),
    tag = "rate-limits",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn get_rate_limit_config(
    RequireSuperAdmin(_user): RequireSuperAdmin,
    State(service): State<Arc<RateLimitConfigService>>,
    Path(key): Path<String>,
) -> Result<Json<ApiResponse<RateLimitConfigResponseDto>>> {
    let config = service.get_config(&key).await?;
    let response: RateLimitConfigResponseDto = config.into();

    Ok(Json(ApiResponse::success(Some(response), None, None)))
}

/// Update a rate limit configuration value
#[utoipa::path(
    put,
    path = "/api/admin/rate-limits/{key}",
    params(
        ("key" = String, Path, description = "Configuration key")
    ),
    request_body = UpdateRateLimitConfigDto,
    responses(
        (status = 200, description = "Updated rate limit configuration", body = ApiResponse<RateLimitConfigResponseDto>),
        (status = 400, description = "Validation error"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - Super admin access required"),
        (status = 404, description = "Configuration not found")
    ),
    tag = "rate-limits",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn update_rate_limit_config(
    RequireSuperAdmin(user): RequireSuperAdmin,
    State(service): State<Arc<RateLimitConfigService>>,
    Path(key): Path<String>,
    Json(dto): Json<UpdateRateLimitConfigDto>,
) -> Result<Json<ApiResponse<RateLimitConfigResponseDto>>> {
    dto.validate()
        .map_err(|e| AppError::Validation(format!("Invalid request: {}", e)))?;

    let config = service
        .update_config(&key, dto.value, &user.account_id)
        .await?;
    let response: RateLimitConfigResponseDto = config.into();

    tracing::info!(
        "Rate limit config '{}' updated to {} by {}",
        key,
        dto.value,
        user.account_id
    );

    Ok(Json(ApiResponse::success(Some(response), None, None)))
}
