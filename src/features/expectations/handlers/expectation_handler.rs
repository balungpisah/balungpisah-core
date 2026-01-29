use std::sync::Arc;

use axum::{extract::State, Json};
use validator::Validate;

use crate::core::error::{AppError, Result};
use crate::core::extractor::AppJson;
use crate::features::expectations::dtos::{CreateExpectationDto, ExpectationResponseDto};
use crate::features::expectations::services::ExpectationService;
use crate::shared::types::ApiResponse;

/// Submit user expectation from landing page
///
/// This is a public endpoint (no authentication required) for collecting
/// user expectations from the "Coming Soon" landing page.
#[utoipa::path(
    post,
    path = "/api/expectations",
    request_body = CreateExpectationDto,
    responses(
        (status = 201, description = "Expectation submitted successfully", body = ApiResponse<ExpectationResponseDto>),
        (status = 400, description = "Validation error")
    ),
    tag = "expectations"
)]
pub async fn create_expectation(
    State(service): State<Arc<ExpectationService>>,
    AppJson(dto): AppJson<CreateExpectationDto>,
) -> Result<Json<ApiResponse<ExpectationResponseDto>>> {
    dto.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let expectation = service.create(dto).await?;
    Ok(Json(ApiResponse::success(
        Some(expectation),
        Some("Terima kasih! Harapan Anda sudah kami terima.".to_string()),
        None,
    )))
}
