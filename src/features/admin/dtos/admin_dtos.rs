use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::features::reports::models::{ReportSeverity, ReportStatus, ReportTagType};
use crate::features::tickets::models::TicketStatus;
use crate::shared::constants::{DEFAULT_PAGE_SIZE, MAX_PAGE_SIZE};

// =============================================================================
// COMMON SORT ENUM
// =============================================================================

/// Sort direction
#[derive(Debug, Clone, Copy, Default, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum SortDirection {
    #[default]
    Desc,
    Asc,
}

impl SortDirection {
    pub fn as_sql(&self) -> &'static str {
        match self {
            SortDirection::Asc => "ASC",
            SortDirection::Desc => "DESC",
        }
    }
}

// =============================================================================
// EXPECTATION DTOs
// =============================================================================

/// Query params for listing expectations
#[derive(Debug, Clone, Deserialize, IntoParams)]
pub struct ExpectationQueryParams {
    /// Page number (1-indexed)
    #[serde(default = "default_page")]
    #[param(minimum = 1)]
    pub page: i64,
    /// Items per page
    #[serde(default = "default_page_size")]
    #[param(minimum = 1, maximum = 100)]
    pub page_size: i64,
    /// Filter by email presence
    pub has_email: Option<bool>,
    /// Filter from date (YYYY-MM-DD)
    pub from_date: Option<NaiveDate>,
    /// Filter to date (YYYY-MM-DD)
    pub to_date: Option<NaiveDate>,
    /// Search in name or expectation text
    pub search: Option<String>,
    /// Sort direction (default: desc)
    #[serde(default)]
    pub sort: SortDirection,
}

impl ExpectationQueryParams {
    pub fn offset(&self) -> i64 {
        (self.page.max(1) - 1) * self.limit()
    }
    pub fn limit(&self) -> i64 {
        self.page_size.clamp(1, MAX_PAGE_SIZE)
    }
}

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

/// Query params for listing reports
#[derive(Debug, Clone, Deserialize, IntoParams)]
pub struct ReportQueryParams {
    /// Page number (1-indexed)
    #[serde(default = "default_page")]
    #[param(minimum = 1)]
    pub page: i64,
    /// Items per page
    #[serde(default = "default_page_size")]
    #[param(minimum = 1, maximum = 100)]
    pub page_size: i64,
    /// Filter by status
    pub status: Option<ReportStatus>,
    /// Filter from date (YYYY-MM-DD)
    pub from_date: Option<NaiveDate>,
    /// Filter to date (YYYY-MM-DD)
    pub to_date: Option<NaiveDate>,
    /// Search in reference_number or title
    pub search: Option<String>,
    /// Filter by user_id
    pub user_id: Option<String>,
    /// Filter by platform
    pub platform: Option<String>,
    /// Filter reports with attachments only
    pub has_attachments: Option<bool>,
    /// Sort by field (default: created_at)
    #[serde(default)]
    pub sort_by: ReportSortBy,
    /// Sort direction (default: desc)
    #[serde(default)]
    pub sort: SortDirection,
}

/// Sort fields for reports
#[derive(Debug, Clone, Copy, Default, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ReportSortBy {
    #[default]
    CreatedAt,
    UpdatedAt,
    Status,
    ReferenceNumber,
}

impl ReportSortBy {
    pub fn as_sql(&self) -> &'static str {
        match self {
            ReportSortBy::CreatedAt => "created_at",
            ReportSortBy::UpdatedAt => "updated_at",
            ReportSortBy::Status => "status",
            ReportSortBy::ReferenceNumber => "reference_number",
        }
    }
}

impl ReportQueryParams {
    pub fn offset(&self) -> i64 {
        (self.page.max(1) - 1) * self.limit()
    }
    pub fn limit(&self) -> i64 {
        self.page_size.clamp(1, MAX_PAGE_SIZE)
    }
}

/// Admin view of report (list)
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
    pub categories: Vec<AdminReportCategoryDto>,
    pub location: Option<AdminReportLocationDto>,
    pub attachment_count: i64,
}

/// Admin view of report detail (single)
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct AdminReportDetailDto {
    pub id: Uuid,
    pub reference_number: Option<String>,
    pub ticket_id: Option<Uuid>,
    pub cluster_id: Option<Uuid>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub timeline: Option<String>,
    pub impact: Option<String>,
    pub status: ReportStatus,
    pub user_id: Option<String>,
    pub platform: Option<String>,
    pub adk_thread_id: Option<Uuid>,
    pub verified_at: Option<DateTime<Utc>>,
    pub verified_by: Option<String>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub resolved_by: Option<String>,
    pub resolution_notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub categories: Vec<AdminReportCategoryDto>,
    pub tags: Vec<ReportTagType>,
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

/// Query params for listing contributors
#[derive(Debug, Clone, Deserialize, IntoParams)]
pub struct ContributorQueryParams {
    /// Page number (1-indexed)
    #[serde(default = "default_page")]
    #[param(minimum = 1)]
    pub page: i64,
    /// Items per page
    #[serde(default = "default_page_size")]
    #[param(minimum = 1, maximum = 100)]
    pub page_size: i64,
    /// Filter by submission type (personal/organization)
    pub submission_type: Option<String>,
    /// Filter from date (YYYY-MM-DD)
    pub from_date: Option<NaiveDate>,
    /// Filter to date (YYYY-MM-DD)
    pub to_date: Option<NaiveDate>,
    /// Search in name, email, or organization_name
    pub search: Option<String>,
    /// Filter by city
    pub city: Option<String>,
    /// Sort direction (default: desc)
    #[serde(default)]
    pub sort: SortDirection,
}

impl ContributorQueryParams {
    pub fn offset(&self) -> i64 {
        (self.page.max(1) - 1) * self.limit()
    }
    pub fn limit(&self) -> i64 {
        self.page_size.clamp(1, MAX_PAGE_SIZE)
    }
}

/// Admin view of contributor (list)
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct AdminContributorDto {
    pub id: Uuid,
    pub submission_type: String,
    pub name: Option<String>,
    pub email: Option<String>,
    pub city: Option<String>,
    pub organization_name: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Admin view of contributor detail (single)
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct AdminContributorDetailDto {
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
    pub updated_at: DateTime<Utc>,
}

// =============================================================================
// TICKET DTOs
// =============================================================================

/// Query params for listing tickets
#[derive(Debug, Clone, Deserialize, IntoParams)]
pub struct TicketQueryParams {
    /// Page number (1-indexed)
    #[serde(default = "default_page")]
    #[param(minimum = 1)]
    pub page: i64,
    /// Items per page
    #[serde(default = "default_page_size")]
    #[param(minimum = 1, maximum = 100)]
    pub page_size: i64,
    /// Filter by status
    pub status: Option<TicketStatus>,
    /// Filter from date (YYYY-MM-DD)
    pub from_date: Option<NaiveDate>,
    /// Filter to date (YYYY-MM-DD)
    pub to_date: Option<NaiveDate>,
    /// Search in reference_number
    pub search: Option<String>,
    /// Filter by user_id
    pub user_id: Option<String>,
    /// Filter by platform
    pub platform: Option<String>,
    /// Filter tickets with errors only
    pub has_error: Option<bool>,
    /// Sort by field (default: created_at)
    #[serde(default)]
    pub sort_by: TicketSortBy,
    /// Sort direction (default: desc)
    #[serde(default)]
    pub sort: SortDirection,
}

/// Sort fields for tickets
#[derive(Debug, Clone, Copy, Default, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum TicketSortBy {
    #[default]
    CreatedAt,
    SubmittedAt,
    Status,
    ReferenceNumber,
}

impl TicketSortBy {
    pub fn as_sql(&self) -> &'static str {
        match self {
            TicketSortBy::CreatedAt => "created_at",
            TicketSortBy::SubmittedAt => "submitted_at",
            TicketSortBy::Status => "status",
            TicketSortBy::ReferenceNumber => "reference_number",
        }
    }
}

impl TicketQueryParams {
    pub fn offset(&self) -> i64 {
        (self.page.max(1) - 1) * self.limit()
    }
    pub fn limit(&self) -> i64 {
        self.page_size.clamp(1, MAX_PAGE_SIZE)
    }
}

/// Admin view of ticket (list)
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct AdminTicketDto {
    pub id: Uuid,
    pub reference_number: String,
    pub user_id: String,
    pub platform: String,
    pub status: TicketStatus,
    pub confidence_score: f64,
    pub retry_count: i32,
    pub has_error: bool,
    pub report_id: Option<Uuid>,
    pub submitted_at: DateTime<Utc>,
    pub processed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// Admin view of ticket detail (single)
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct AdminTicketDetailDto {
    pub id: Uuid,
    pub reference_number: String,
    pub adk_thread_id: Uuid,
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
    pub last_attempt_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

fn default_page() -> i64 {
    1
}

fn default_page_size() -> i64 {
    DEFAULT_PAGE_SIZE
}
