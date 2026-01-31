use std::sync::Arc;

use axum::{
    extract::{Path, State},
    Json,
};

use crate::core::error::Result;
use crate::core::extractor::AppJson;
use crate::features::auth::model::AuthenticatedUser;
use crate::features::reports::dtos::{
    ClusterDetailResponseDto, ReportClusterResponseDto, ReportDetailResponseDto,
    ReportLocationResponseDto, ReportResponseDto, UpdateReportStatusDto,
};
use crate::features::reports::services::{ClusteringService, ReportService};
use crate::shared::types::ApiResponse;

/// State for report handlers
#[derive(Clone)]
pub struct ReportState {
    pub report_service: Arc<ReportService>,
    pub clustering_service: Arc<ClusteringService>,
}

/// List reports for the authenticated user
#[utoipa::path(
    get,
    path = "/api/reports",
    responses(
        (status = 200, description = "List of user's reports", body = ApiResponse<Vec<ReportResponseDto>>),
        (status = 401, description = "Unauthorized")
    ),
    security(("bearer_auth" = [])),
    tag = "reports"
)]
pub async fn list_reports(
    user: AuthenticatedUser,
    State(state): State<ReportState>,
) -> Result<Json<ApiResponse<Vec<ReportResponseDto>>>> {
    let reports = state.report_service.list_by_user(&user.sub).await?;
    let dtos: Vec<ReportResponseDto> = reports.into_iter().map(|r| r.into()).collect();
    Ok(Json(ApiResponse::success(Some(dtos), None, None)))
}

/// Get report by ID with location
#[utoipa::path(
    get,
    path = "/api/reports/{id}",
    params(
        ("id" = Uuid, Path, description = "Report ID")
    ),
    responses(
        (status = 200, description = "Report found", body = ApiResponse<ReportDetailResponseDto>),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Report not found")
    ),
    security(("bearer_auth" = [])),
    tag = "reports"
)]
pub async fn get_report(
    user: AuthenticatedUser,
    State(state): State<ReportState>,
    Path(id): Path<uuid::Uuid>,
) -> Result<Json<ApiResponse<ReportDetailResponseDto>>> {
    let report = state.report_service.get_by_id(id).await?;

    // Verify ownership by checking the ticket belongs to the user
    let reports = state.report_service.list_by_user(&user.sub).await?;
    if !reports.iter().any(|r| r.id == id) {
        return Err(crate::core::error::AppError::NotFound(format!(
            "Report {} not found",
            id
        )));
    }

    let location = state.report_service.get_location(id).await?;
    let dto = ReportDetailResponseDto {
        report: report.into(),
        location: location.map(ReportLocationResponseDto::from),
    };

    Ok(Json(ApiResponse::success(Some(dto), None, None)))
}

/// Update report status (admin only)
#[utoipa::path(
    patch,
    path = "/api/reports/{id}/status",
    params(
        ("id" = Uuid, Path, description = "Report ID")
    ),
    request_body = UpdateReportStatusDto,
    responses(
        (status = 200, description = "Status updated", body = ApiResponse<ReportResponseDto>),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Report not found")
    ),
    security(("bearer_auth" = [])),
    tag = "reports"
)]
pub async fn update_report_status(
    user: AuthenticatedUser,
    State(state): State<ReportState>,
    Path(id): Path<uuid::Uuid>,
    AppJson(dto): AppJson<UpdateReportStatusDto>,
) -> Result<Json<ApiResponse<ReportResponseDto>>> {
    // TODO: Add admin role check
    let report = state
        .report_service
        .update_status(id, &dto, &user.sub)
        .await?;
    Ok(Json(ApiResponse::success(Some(report.into()), None, None)))
}

/// List active clusters (public)
#[utoipa::path(
    get,
    path = "/api/reports/clusters",
    responses(
        (status = 200, description = "List of active clusters", body = ApiResponse<Vec<ReportClusterResponseDto>>)
    ),
    tag = "reports"
)]
pub async fn list_clusters(
    State(state): State<ReportState>,
) -> Result<Json<ApiResponse<Vec<ReportClusterResponseDto>>>> {
    let clusters = state.clustering_service.list_active().await?;
    let dtos: Vec<ReportClusterResponseDto> = clusters.into_iter().map(|c| c.into()).collect();
    Ok(Json(ApiResponse::success(Some(dtos), None, None)))
}

/// Get cluster by ID with reports (public)
#[utoipa::path(
    get,
    path = "/api/reports/clusters/{id}",
    params(
        ("id" = Uuid, Path, description = "Cluster ID")
    ),
    responses(
        (status = 200, description = "Cluster found", body = ApiResponse<ClusterDetailResponseDto>),
        (status = 404, description = "Cluster not found")
    ),
    tag = "reports"
)]
pub async fn get_cluster(
    State(state): State<ReportState>,
    Path(id): Path<uuid::Uuid>,
) -> Result<Json<ApiResponse<ClusterDetailResponseDto>>> {
    let cluster = state.clustering_service.get_by_id(id).await?;
    let reports = state.report_service.list_by_cluster(id).await?;

    let dto = ClusterDetailResponseDto {
        cluster: cluster.into(),
        reports: reports.into_iter().map(|r| r.into()).collect(),
    };

    Ok(Json(ApiResponse::success(Some(dto), None, None)))
}
