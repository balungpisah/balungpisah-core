use std::sync::Arc;

use axum::{
    extract::{Path, State},
    Json,
};

use crate::core::error::Result;
use crate::core::extractor::AppJson;
use crate::features::auth::model::AuthenticatedUser;
use crate::features::reports::dtos::{
    ReportDetailResponseDto, ReportResponseDto, UpdateReportStatusDto,
};
use crate::features::reports::services::ReportService;
use crate::shared::types::ApiResponse;

/// State for report handlers
#[derive(Clone)]
pub struct ReportState {
    pub report_service: Arc<ReportService>,
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
    let mut dtos = Vec::with_capacity(reports.len());
    for r in reports {
        let id = r.id;
        let mut dto: ReportResponseDto = r.into();
        dto.categories = state.report_service.get_categories_with_names(id).await?;
        dto.tags = state
            .report_service
            .get_tags(id)
            .await?
            .into_iter()
            .map(|t| t.into())
            .collect();
        dto.location_display_name = state.report_service.get_location_display_name(id).await?;
        dtos.push(dto);
    }
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

    let mut report_dto: ReportResponseDto = report.into();
    report_dto.categories = state.report_service.get_categories_with_names(id).await?;
    report_dto.tags = state
        .report_service
        .get_tags(id)
        .await?
        .into_iter()
        .map(|t| t.into())
        .collect();

    let location = state.report_service.get_location_with_regions(id).await?;
    let dto = ReportDetailResponseDto {
        report: report_dto,
        location,
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
