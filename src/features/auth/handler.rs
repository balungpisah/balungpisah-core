use crate::core::error::Result;
use crate::features::auth::dto::MeResponseDto;
use crate::features::auth::guards::RequireSuperAdmin;
use crate::features::auth::service::AuthService;
use crate::shared::types::ApiResponse;
use axum::{extract::State, Json};
use std::sync::Arc;

#[utoipa::path(
    get,
    path = "/api/auth/me",
    responses(
        (status = 200, description = "Current user retrieved successfully", body = ApiResponse<MeResponseDto>),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - Super admin access required")
    ),
    tag = "auth",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn get_me(
    RequireSuperAdmin(user): RequireSuperAdmin,
    State(service): State<Arc<AuthService>>,
) -> Result<Json<ApiResponse<MeResponseDto>>> {
    let user_data = service.get_current_user(user).await?;
    Ok(Json(ApiResponse::success(Some(user_data), None, None)))
}
