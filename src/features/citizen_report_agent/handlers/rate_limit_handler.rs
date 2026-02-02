use std::sync::Arc;

use axum::{extract::State, Json};

use crate::core::error::Result;
use crate::features::auth::model::AuthenticatedUser;
use crate::features::rate_limits::dtos::UserRateLimitStatusDto;
use crate::features::rate_limits::services::RateLimitService;
use crate::shared::types::ApiResponse;

/// Get current user's rate limit status
#[utoipa::path(
    get,
    path = "/api/citizen-report-agent/rate-limit",
    responses(
        (status = 200, description = "User's rate limit status", body = ApiResponse<UserRateLimitStatusDto>),
        (status = 401, description = "Unauthorized")
    ),
    tag = "citizen-report-agent",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn get_user_rate_limit(
    user: AuthenticatedUser,
    State(service): State<Arc<RateLimitService>>,
) -> Result<Json<ApiResponse<UserRateLimitStatusDto>>> {
    let status = service.get_user_status(&user.account_id).await?;
    Ok(Json(ApiResponse::success(Some(status), None, None)))
}
