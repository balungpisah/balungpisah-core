use chrono::{DateTime, Utc};
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::features::reports::models::{ReportSeverity, ReportStatus};
use crate::features::tickets::models::TicketStatus;

// =============================================================================
// EXPECTATION DTOs
// =============================================================================

/// Admin view of expectation
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct AdminExpectationDto {
    pub id: Uuid,
    pub name: Option<String>,
    pub email: Option<String>,
    pub expectation: String,
    pub created_at: DateTime<Utc>,
}

// =============================================================================
// REPORT DTOs
// =============================================================================

/// Admin view of report with attachments
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct AdminReportDto {
    pub id: Uuid,
    pub reference_number: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub status: ReportStatus,
    pub user_id: Option<String>,
    pub platform: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    // Related data
    pub categories: Vec<AdminReportCategoryDto>,
    pub location: Option<AdminReportLocationDto>,
    pub attachments: Vec<AdminReportAttachmentDto>,
}

/// Category info for admin report
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct AdminReportCategoryDto {
    pub category_id: Uuid,
    pub category_name: String,
    pub category_slug: String,
    pub severity: ReportSeverity,
}

/// Location info for admin report
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct AdminReportLocationDto {
    pub raw_input: String,
    pub display_name: Option<String>,
    pub lat: Option<f64>,
    pub lon: Option<f64>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub province_name: Option<String>,
    pub regency_name: Option<String>,
}

/// Attachment info for admin report
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct AdminReportAttachmentDto {
    pub file_id: Uuid,
    pub original_filename: String,
    pub content_type: String,
    pub file_size: i64,
    pub url: String,
}

// =============================================================================
// CONTRIBUTOR DTOs
// =============================================================================

/// Admin view of contributor
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct AdminContributorDto {
    pub id: Uuid,
    pub submission_type: String,
    // Personal fields
    pub name: Option<String>,
    pub email: Option<String>,
    pub whatsapp: Option<String>,
    pub city: Option<String>,
    pub role: Option<String>,
    pub skills: Option<String>,
    pub bio: Option<String>,
    pub portfolio_url: Option<String>,
    pub aspiration: Option<String>,
    // Organization fields
    pub organization_name: Option<String>,
    pub organization_type: Option<String>,
    pub contact_name: Option<String>,
    pub contact_position: Option<String>,
    pub contact_whatsapp: Option<String>,
    pub contact_email: Option<String>,
    pub contribution_offer: Option<String>,
    // Common
    pub agreed: bool,
    pub created_at: DateTime<Utc>,
}

// =============================================================================
// TICKET DTOs
// =============================================================================

/// Admin view of ticket
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct AdminTicketDto {
    pub id: Uuid,
    pub reference_number: String,
    pub user_id: String,
    pub platform: String,
    pub status: TicketStatus,
    pub confidence_score: f64,
    pub completeness_score: Option<f64>,
    pub retry_count: i32,
    pub error_message: Option<String>,
    pub report_id: Option<Uuid>,
    pub submitted_at: DateTime<Utc>,
    pub processed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}
