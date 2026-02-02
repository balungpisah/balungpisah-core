use axum::{
    extract::{Path, Query, State},
    Json,
};
use std::sync::Arc;
use uuid::Uuid;

use crate::core::error::AppError;
use crate::features::dashboard::dtos::*;
use crate::features::dashboard::services::DashboardService;
use crate::shared::types::{ApiResponse, Meta};

// ============================================================================
// Summary
// ============================================================================

/// Get lightweight dashboard summary
#[utoipa::path(
    get,
    path = "/api/dashboard/summary",
    tag = "Dashboard",
    responses(
        (status = 200, description = "Dashboard summary", body = ApiResponse<DashboardSummaryDto>),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_summary(
    State(service): State<Arc<DashboardService>>,
) -> Result<Json<ApiResponse<DashboardSummaryDto>>, AppError> {
    let summary = service.get_summary().await?;
    Ok(Json(ApiResponse::success(Some(summary), None, None)))
}

// ============================================================================
// Reports List
// ============================================================================

/// List all reports with pagination
#[utoipa::path(
    get,
    path = "/api/dashboard/reports",
    tag = "Dashboard",
    params(PaginationParams),
    responses(
        (status = 200, description = "Paginated reports list", body = ApiResponse<Vec<DashboardReportDto>>),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn list_reports(
    State(service): State<Arc<DashboardService>>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<ApiResponse<Vec<DashboardReportDto>>>, AppError> {
    let (reports, total) = service.list_reports(&params).await?;
    Ok(Json(ApiResponse::success(
        Some(reports),
        None,
        Some(Meta { total }),
    )))
}

/// Get single report detail
#[utoipa::path(
    get,
    path = "/api/dashboard/reports/{id}",
    tag = "Dashboard",
    params(
        ("id" = Uuid, Path, description = "Report ID")
    ),
    responses(
        (status = 200, description = "Report detail", body = ApiResponse<DashboardReportDetailDto>),
        (status = 404, description = "Report not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_report(
    State(service): State<Arc<DashboardService>>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<DashboardReportDetailDto>>, AppError> {
    let data = service.get_report(id).await?;
    Ok(Json(ApiResponse::success(Some(data), None, None)))
}

// ============================================================================
// By Location
// ============================================================================

/// Get location overview (provinces -> regencies -> reports)
#[utoipa::path(
    get,
    path = "/api/dashboard/by-location",
    tag = "Dashboard",
    params(LocationQueryParams),
    responses(
        (status = 200, description = "Location overview with reports", body = ApiResponse<DashboardLocationOverviewDto>),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_by_location(
    State(service): State<Arc<DashboardService>>,
    Query(params): Query<LocationQueryParams>,
) -> Result<Json<ApiResponse<DashboardLocationOverviewDto>>, AppError> {
    let data = service.get_by_location(&params).await?;
    Ok(Json(ApiResponse::success(Some(data), None, None)))
}

// ============================================================================
// By Category
// ============================================================================

/// Get category overview with optional report listing
#[utoipa::path(
    get,
    path = "/api/dashboard/by-category",
    tag = "Dashboard",
    params(CategoryQueryParams),
    responses(
        (status = 200, description = "Category overview with reports", body = ApiResponse<DashboardCategoryOverviewDto>),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_by_category(
    State(service): State<Arc<DashboardService>>,
    Query(params): Query<CategoryQueryParams>,
) -> Result<Json<ApiResponse<DashboardCategoryOverviewDto>>, AppError> {
    let data = service.get_by_category(&params).await?;
    Ok(Json(ApiResponse::success(Some(data), None, None)))
}

// ============================================================================
// By Tag
// ============================================================================

/// Get tag overview with optional report listing
#[utoipa::path(
    get,
    path = "/api/dashboard/by-tag",
    tag = "Dashboard",
    params(TagQueryParams),
    responses(
        (status = 200, description = "Tag overview with reports", body = ApiResponse<DashboardTagOverviewDto>),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_by_tag(
    State(service): State<Arc<DashboardService>>,
    Query(params): Query<TagQueryParams>,
) -> Result<Json<ApiResponse<DashboardTagOverviewDto>>, AppError> {
    let data = service.get_by_tag(&params).await?;
    Ok(Json(ApiResponse::success(Some(data), None, None)))
}

// ============================================================================
// Recent Reports
// ============================================================================

/// Get recent reports (last N days)
#[utoipa::path(
    get,
    path = "/api/dashboard/recent",
    tag = "Dashboard",
    params(RecentQueryParams),
    responses(
        (status = 200, description = "Recent reports", body = ApiResponse<DashboardRecentDto>),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_recent(
    State(service): State<Arc<DashboardService>>,
    Query(params): Query<RecentQueryParams>,
) -> Result<Json<ApiResponse<DashboardRecentDto>>, AppError> {
    let data = service.get_recent(&params).await?;
    Ok(Json(ApiResponse::success(Some(data), None, None)))
}

// ============================================================================
// Map View
// ============================================================================

/// Get map markers for reports with coordinates
#[utoipa::path(
    get,
    path = "/api/dashboard/map",
    tag = "Dashboard",
    params(MapQueryParams),
    responses(
        (status = 200, description = "Map data with markers", body = ApiResponse<DashboardMapDto>),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_map(
    State(service): State<Arc<DashboardService>>,
    Query(params): Query<MapQueryParams>,
) -> Result<Json<ApiResponse<DashboardMapDto>>, AppError> {
    let data = service.get_map_data(&params).await?;
    Ok(Json(ApiResponse::success(Some(data), None, None)))
}


#[utoipa::path(
    get,
    path = "/api/dashboard/map-data",
    tag = "Dashboard",
    params(LocationQueryParams),
    responses(
        (status = 200, description = "Minimalist map points for distribution view", body = ApiResponse<DashboardMapDataDto>),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_map_data(
    State(service): State<Arc<DashboardService>>,
    Query(params): Query<LocationQueryParams>,
) -> Result<Json<ApiResponse<DashboardMapDataDto>>, AppError> {
    let data = service.get_map_data_markers(&params).await?;
    Ok(Json(ApiResponse::success(Some(data), None, None)))
}