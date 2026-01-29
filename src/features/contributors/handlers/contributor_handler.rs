//! Contributor registration handler

use std::sync::Arc;

use axum::{extract::State, Json};

use crate::core::error::Result;
use crate::core::extractor::AppJson;
use crate::features::contributors::dtos::{ContributorResponseDto, CreateContributorDto};
use crate::features::contributors::services::ContributorService;
use crate::shared::types::ApiResponse;

/// Register a new contributor
///
/// Public endpoint for contributor registration - stores form data.
#[utoipa::path(
    post,
    path = "/api/contributors/register",
    request_body = CreateContributorDto,
    responses(
        (status = 201, description = "Contributor registered successfully", body = ApiResponse<ContributorResponseDto>),
        (status = 400, description = "Invalid request")
    ),
    tag = "contributors"
)]
pub async fn register_contributor(
    State(service): State<Arc<ContributorService>>,
    AppJson(dto): AppJson<CreateContributorDto>,
) -> Result<Json<ApiResponse<ContributorResponseDto>>> {
    let result = service.register(dto).await?;

    Ok(Json(ApiResponse::success(
        Some(result),
        Some("Pendaftaran berhasil!".to_string()),
        None,
    )))
}
