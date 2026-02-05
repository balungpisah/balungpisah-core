use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    Json,
};
use uuid::Uuid;

use crate::core::error::Result;
use crate::features::admin::dtos::*;
use crate::features::admin::services::AdminService;
use crate::features::auth::guards::RequireSuperAdmin;
use crate::shared::types::{ApiResponse, Meta};

// =============================================================================
// EXPECTATION HANDLERS
// =============================================================================

/// List all expectations (paginated with filters)
#[utoipa::path(
    get,
    path = "/api/admin/expectations",
    params(ExpectationQueryParams),
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
    Query(params): Query<ExpectationQueryParams>,
) -> Result<Json<ApiResponse<Vec<AdminExpectationDto>>>> {
    let (items, total) = service.list_expectations(&params).await?;

    Ok(Json(ApiResponse::success(
        Some(items),
        None,
        Some(Meta { total }),
    )))
}

/// Get a single expectation by ID
#[utoipa::path(
    get,
    path = "/api/admin/expectations/{id}",
    params(
        ("id" = Uuid, Path, description = "Expectation ID")
    ),
    responses(
        (status = 200, description = "Expectation details", body = ApiResponse<AdminExpectationDto>),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - Super admin access required"),
        (status = 404, description = "Expectation not found")
    ),
    tag = "admin",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn get_expectation(
    RequireSuperAdmin(_user): RequireSuperAdmin,
    State(service): State<Arc<AdminService>>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<AdminExpectationDto>>> {
    let item = service.get_expectation(id).await?;
    Ok(Json(ApiResponse::success(Some(item), None, None)))
}

// =============================================================================
// REPORT HANDLERS
// =============================================================================

/// List all reports with attachments (paginated with filters)
#[utoipa::path(
    get,
    path = "/api/admin/reports",
    params(ReportQueryParams),
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
    Query(params): Query<ReportQueryParams>,
) -> Result<Json<ApiResponse<Vec<AdminReportDto>>>> {
    let (items, total) = service.list_reports(&params).await?;

    Ok(Json(ApiResponse::success(
        Some(items),
        None,
        Some(Meta { total }),
    )))
}

/// Get a single report by ID with full details
#[utoipa::path(
    get,
    path = "/api/admin/reports/{id}",
    params(
        ("id" = Uuid, Path, description = "Report ID")
    ),
    responses(
        (status = 200, description = "Report details", body = ApiResponse<AdminReportDetailDto>),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - Super admin access required"),
        (status = 404, description = "Report not found")
    ),
    tag = "admin",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn get_report(
    RequireSuperAdmin(_user): RequireSuperAdmin,
    State(service): State<Arc<AdminService>>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<AdminReportDetailDto>>> {
    let item = service.get_report(id).await?;
    Ok(Json(ApiResponse::success(Some(item), None, None)))
}

// =============================================================================
// CONTRIBUTOR HANDLERS
// =============================================================================

/// List all contributors (paginated with filters)
#[utoipa::path(
    get,
    path = "/api/admin/contributors",
    params(ContributorQueryParams),
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
    Query(params): Query<ContributorQueryParams>,
) -> Result<Json<ApiResponse<Vec<AdminContributorDto>>>> {
    let (items, total) = service.list_contributors(&params).await?;

    Ok(Json(ApiResponse::success(
        Some(items),
        None,
        Some(Meta { total }),
    )))
}

/// Get a single contributor by ID with full details
#[utoipa::path(
    get,
    path = "/api/admin/contributors/{id}",
    params(
        ("id" = Uuid, Path, description = "Contributor ID")
    ),
    responses(
        (status = 200, description = "Contributor details", body = ApiResponse<AdminContributorDetailDto>),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - Super admin access required"),
        (status = 404, description = "Contributor not found")
    ),
    tag = "admin",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn get_contributor(
    RequireSuperAdmin(_user): RequireSuperAdmin,
    State(service): State<Arc<AdminService>>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<AdminContributorDetailDto>>> {
    let item = service.get_contributor(id).await?;
    Ok(Json(ApiResponse::success(Some(item), None, None)))
}
