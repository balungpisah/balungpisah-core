use axum::{extract::Path, extract::Query, extract::State, Json};
use std::sync::Arc;
use uuid::Uuid;
use validator::Validate;

use crate::core::error::{AppError, Result};
use crate::core::extractor::AppJson;
use crate::features::auth::guards::RequireSuperAdmin;
use crate::features::prompts::dtos::{
    CreatePromptDto, PromptQueryParams, PromptResponseDto, UpdatePromptDto,
};
use crate::features::prompts::services::PromptService;
use crate::shared::types::{ApiResponse, Meta};

/// Create a new prompt template (super admin only)
#[utoipa::path(
    post,
    path = "/api/admin/prompts",
    request_body = CreatePromptDto,
    responses(
        (status = 201, description = "Prompt created successfully", body = ApiResponse<PromptResponseDto>),
        (status = 400, description = "Validation error"),
        (status = 403, description = "Forbidden - super admin only")
    ),
    tag = "prompts",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn create_prompt(
    RequireSuperAdmin(_user): RequireSuperAdmin,
    State(service): State<Arc<PromptService>>,
    AppJson(dto): AppJson<CreatePromptDto>,
) -> Result<Json<ApiResponse<PromptResponseDto>>> {
    dto.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let prompt = service.create(dto).await?;
    Ok(Json(ApiResponse::success(Some(prompt), None, None)))
}

/// Get a prompt by ID (super admin only)
#[utoipa::path(
    get,
    path = "/api/admin/prompts/{id}",
    params(
        ("id" = Uuid, Path, description = "Prompt ID")
    ),
    responses(
        (status = 200, description = "Prompt retrieved successfully", body = ApiResponse<PromptResponseDto>),
        (status = 404, description = "Prompt not found"),
        (status = 403, description = "Forbidden - super admin only")
    ),
    tag = "prompts",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn get_prompt(
    RequireSuperAdmin(_user): RequireSuperAdmin,
    State(service): State<Arc<PromptService>>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<PromptResponseDto>>> {
    let prompt = service.get_by_id(id).await?;
    Ok(Json(ApiResponse::success(Some(prompt), None, None)))
}

/// List all prompts with pagination and filters (super admin only)
#[utoipa::path(
    get,
    path = "/api/admin/prompts",
    params(PromptQueryParams),
    responses(
        (status = 200, description = "Prompts retrieved successfully", body = ApiResponse<Vec<PromptResponseDto>>),
        (status = 403, description = "Forbidden - super admin only")
    ),
    tag = "prompts",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn list_prompts(
    RequireSuperAdmin(_user): RequireSuperAdmin,
    State(service): State<Arc<PromptService>>,
    Query(params): Query<PromptQueryParams>,
) -> Result<Json<ApiResponse<Vec<PromptResponseDto>>>> {
    let (prompts, total) = service.list(&params).await?;
    Ok(Json(ApiResponse::success(
        Some(prompts),
        None,
        Some(Meta { total }),
    )))
}

/// Update a prompt (super admin only)
#[utoipa::path(
    put,
    path = "/api/admin/prompts/{id}",
    params(
        ("id" = Uuid, Path, description = "Prompt ID")
    ),
    request_body = UpdatePromptDto,
    responses(
        (status = 200, description = "Prompt updated successfully", body = ApiResponse<PromptResponseDto>),
        (status = 400, description = "Validation error"),
        (status = 404, description = "Prompt not found"),
        (status = 403, description = "Forbidden - super admin only")
    ),
    tag = "prompts",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn update_prompt(
    RequireSuperAdmin(_user): RequireSuperAdmin,
    State(service): State<Arc<PromptService>>,
    Path(id): Path<Uuid>,
    AppJson(dto): AppJson<UpdatePromptDto>,
) -> Result<Json<ApiResponse<PromptResponseDto>>> {
    dto.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let prompt = service.update(id, dto).await?;
    Ok(Json(ApiResponse::success(Some(prompt), None, None)))
}

/// Restore a soft-deleted prompt (super admin only)
#[utoipa::path(
    post,
    path = "/api/admin/prompts/{id}/restore",
    params(
        ("id" = Uuid, Path, description = "Prompt ID")
    ),
    responses(
        (status = 200, description = "Prompt restored successfully", body = ApiResponse<PromptResponseDto>),
        (status = 400, description = "Prompt is already active"),
        (status = 404, description = "Prompt not found"),
        (status = 409, description = "Active prompt with same key already exists"),
        (status = 403, description = "Forbidden - super admin only")
    ),
    tag = "prompts",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn restore_prompt(
    RequireSuperAdmin(_user): RequireSuperAdmin,
    State(service): State<Arc<PromptService>>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<PromptResponseDto>>> {
    let prompt = service.restore(id).await?;
    Ok(Json(ApiResponse::success(Some(prompt), None, None)))
}

/// Delete a prompt (soft delete, super admin only)
#[utoipa::path(
    delete,
    path = "/api/admin/prompts/{id}",
    params(
        ("id" = Uuid, Path, description = "Prompt ID")
    ),
    responses(
        (status = 200, description = "Prompt deleted successfully"),
        (status = 404, description = "Prompt not found"),
        (status = 403, description = "Forbidden - super admin only")
    ),
    tag = "prompts",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn delete_prompt(
    RequireSuperAdmin(_user): RequireSuperAdmin,
    State(service): State<Arc<PromptService>>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<()>>> {
    service.delete(id).await?;
    Ok(Json(ApiResponse::success(None, None, None)))
}
