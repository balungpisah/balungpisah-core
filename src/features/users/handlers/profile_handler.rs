use crate::core::error::{AppError, Result};
use crate::core::extractor::AppJson;
use crate::features::auth::model::AuthenticatedUser;
use crate::features::users::dtos::{
    ExtendedProfileDto, UpdateBasicProfileDto, UpdateExtendedProfileDto, UserProfileResponseDto,
};
use crate::features::users::services::UserProfileService;
use crate::shared::types::ApiResponse;
use axum::{extract::State, Json};
use std::sync::Arc;
use validator::Validate;

#[utoipa::path(
    get,
    path = "/api/users/me",
    responses(
        (status = 200, description = "Profile retrieved successfully", body = ApiResponse<UserProfileResponseDto>),
        (status = 401, description = "Unauthorized")
    ),
    tag = "users",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn get_profile(
    user: AuthenticatedUser,
    State(service): State<Arc<UserProfileService>>,
) -> Result<Json<ApiResponse<UserProfileResponseDto>>> {
    let profile = service.get_profile(&user).await?;
    Ok(Json(ApiResponse::success(Some(profile), None, None)))
}

#[utoipa::path(
    patch,
    path = "/api/users/me",
    request_body = UpdateBasicProfileDto,
    responses(
        (status = 200, description = "Profile updated successfully", body = ApiResponse<UserProfileResponseDto>),
        (status = 400, description = "Validation error"),
        (status = 401, description = "Unauthorized"),
        (status = 409, description = "Username already taken")
    ),
    tag = "users",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn update_basic_profile(
    user: AuthenticatedUser,
    State(service): State<Arc<UserProfileService>>,
    AppJson(dto): AppJson<UpdateBasicProfileDto>,
) -> Result<Json<ApiResponse<UserProfileResponseDto>>> {
    dto.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let profile = service.update_basic_profile(&user, dto).await?;
    Ok(Json(ApiResponse::success(
        Some(profile),
        Some("Profile updated successfully".to_string()),
        None,
    )))
}

#[utoipa::path(
    patch,
    path = "/api/users/me/profile",
    request_body = UpdateExtendedProfileDto,
    responses(
        (status = 200, description = "Extended profile updated successfully", body = ApiResponse<ExtendedProfileDto>),
        (status = 400, description = "Validation error"),
        (status = 401, description = "Unauthorized")
    ),
    tag = "users",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn update_extended_profile(
    user: AuthenticatedUser,
    State(service): State<Arc<UserProfileService>>,
    AppJson(dto): AppJson<UpdateExtendedProfileDto>,
) -> Result<Json<ApiResponse<ExtendedProfileDto>>> {
    dto.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let profile = service.update_extended_profile(&user, dto).await?;
    Ok(Json(ApiResponse::success(
        Some(profile),
        Some("Extended profile updated successfully".to_string()),
        None,
    )))
}
