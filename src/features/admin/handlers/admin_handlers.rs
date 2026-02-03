use std::sync::Arc;

use axum::{
    extract::{Query, State},
    Json,
};

use crate::core::error::Result;
use crate::features::admin::dtos::*;
use crate::features::admin::services::AdminService;
use crate::features::auth::guards::RequireSuperAdmin;
use crate::shared::types::{ApiResponse, Meta, PaginationQuery};

/// List all expectations (paginated)
#[utoipa::path(
    get,
    path = "/api/admin/expectations",
    params(PaginationQuery),
    responses(
        (status = 200, description = "List of expectations", body = ApiResponse<Vec<AdminExpectationDto>>),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - Super admin access required")
    ),
    tag = "admin",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn list_expectations(
    RequireSuperAdmin(_user): RequireSuperAdmin,
    State(service): State<Arc<AdminService>>,
    Query(params): Query<PaginationQuery>,
) -> Result<Json<ApiResponse<Vec<AdminExpectationDto>>>> {
    let (items, total) = service
        .list_expectations(params.offset(), params.limit())
        .await?;

    Ok(Json(ApiResponse::success(
        Some(items),
        None,
        Some(Meta { total }),
    )))
}

/// List all reports with attachments (paginated)
#[utoipa::path(
    get,
    path = "/api/admin/reports",
    params(PaginationQuery),
    responses(
        (status = 200, description = "List of reports with attachments", body = ApiResponse<Vec<AdminReportDto>>),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - Super admin access required")
    ),
    tag = "admin",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn list_reports(
    RequireSuperAdmin(_user): RequireSuperAdmin,
    State(service): State<Arc<AdminService>>,
    Query(params): Query<PaginationQuery>,
) -> Result<Json<ApiResponse<Vec<AdminReportDto>>>> {
    let (items, total) = service
        .list_reports(params.offset(), params.limit())
        .await?;

    Ok(Json(ApiResponse::success(
        Some(items),
        None,
        Some(Meta { total }),
    )))
}

/// List all contributors (paginated)
#[utoipa::path(
    get,
    path = "/api/admin/contributors",
    params(PaginationQuery),
    responses(
        (status = 200, description = "List of contributors", body = ApiResponse<Vec<AdminContributorDto>>),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - Super admin access required")
    ),
    tag = "admin",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn list_contributors(
    RequireSuperAdmin(_user): RequireSuperAdmin,
    State(service): State<Arc<AdminService>>,
    Query(params): Query<PaginationQuery>,
) -> Result<Json<ApiResponse<Vec<AdminContributorDto>>>> {
    let (items, total) = service
        .list_contributors(params.offset(), params.limit())
        .await?;

    Ok(Json(ApiResponse::success(
        Some(items),
        None,
        Some(Meta { total }),
    )))
}

/// List all tickets (paginated)
#[utoipa::path(
    get,
    path = "/api/admin/tickets",
    params(PaginationQuery),
    responses(
        (status = 200, description = "List of tickets", body = ApiResponse<Vec<AdminTicketDto>>),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - Super admin access required")
    ),
    tag = "admin",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn list_tickets(
    RequireSuperAdmin(_user): RequireSuperAdmin,
    State(service): State<Arc<AdminService>>,
    Query(params): Query<PaginationQuery>,
) -> Result<Json<ApiResponse<Vec<AdminTicketDto>>>> {
    let (items, total) = service
        .list_tickets(params.offset(), params.limit())
        .await?;

    Ok(Json(ApiResponse::success(
        Some(items),
        None,
        Some(Meta { total }),
    )))
}
