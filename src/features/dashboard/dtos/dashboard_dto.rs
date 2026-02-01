use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::features::reports::models::{ReportSeverity, ReportStatus, ReportTagType};
use crate::shared::constants::{DEFAULT_PAGE_SIZE, MAX_PAGE_SIZE};

// ============================================================================
// Pagination
// ============================================================================

/// Pagination metadata for response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PaginationMeta {
    pub page: i64,
    pub page_size: i64,
    pub total_items: i64,
    pub total_pages: i64,
}

impl PaginationMeta {
    pub fn new(page: i64, page_size: i64, total_items: i64) -> Self {
        let clamped_page_size = page_size.clamp(1, MAX_PAGE_SIZE);
        let total_pages = (total_items as f64 / clamped_page_size as f64).ceil() as i64;
        Self {
            page,
            page_size: clamped_page_size,
            total_items,
            total_pages,
        }
    }
}

fn default_page() -> i64 {
    1
}

fn default_page_size() -> i64 {
    DEFAULT_PAGE_SIZE
}

/// Standard pagination query parameters
#[derive(Debug, Clone, Deserialize, IntoParams)]
pub struct PaginationParams {
    /// Page number (1-indexed)
    #[serde(default = "default_page")]
    #[param(minimum = 1)]
    pub page: i64,

    /// Number of items per page
    #[serde(default = "default_page_size")]
    #[param(minimum = 1, maximum = 100)]
    pub page_size: i64,
}

impl PaginationParams {
    pub fn offset(&self) -> i64 {
        (self.page.max(1) - 1) * self.limit()
    }

    pub fn limit(&self) -> i64 {
        self.page_size.clamp(1, MAX_PAGE_SIZE)
    }
}

// ============================================================================
// Report DTOs for Dashboard
// ============================================================================

/// Category info for a report
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ReportCategoryInfo {
    pub category_id: Uuid,
    pub name: String,
    pub slug: String,
    pub severity: ReportSeverity,
    pub color: Option<String>,
    pub icon: Option<String>,
}

/// Location info for a report
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ReportLocationInfo {
    pub raw_input: String,
    pub display_name: Option<String>,
    pub lat: Option<f64>,
    pub lon: Option<f64>,
    pub road: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub province_id: Option<Uuid>,
    pub province_name: Option<String>,
    pub regency_id: Option<Uuid>,
    pub regency_name: Option<String>,
}

/// Report item for listing
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DashboardReportDto {
    pub id: Uuid,
    pub title: String,
    pub description: String,
    pub status: ReportStatus,
    pub tag_type: Option<ReportTagType>,
    pub timeline: Option<String>,
    pub impact: Option<String>,
    pub created_at: DateTime<Utc>,
    pub categories: Vec<ReportCategoryInfo>,
    pub location: Option<ReportLocationInfo>,
}

/// Report detail with full information
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DashboardReportDetailDto {
    pub id: Uuid,
    pub ticket_id: Uuid,
    pub title: String,
    pub description: String,
    pub status: ReportStatus,
    pub tag_type: Option<ReportTagType>,
    pub timeline: Option<String>,
    pub impact: Option<String>,
    pub verified_at: Option<DateTime<Utc>>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub resolution_notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub categories: Vec<ReportCategoryInfo>,
    pub location: Option<ReportLocationInfo>,
}

// ============================================================================
// By Location
// ============================================================================

/// Query params for location-based listing
#[derive(Debug, Clone, Deserialize, IntoParams)]
pub struct LocationQueryParams {
    /// Filter by province ID
    pub province_id: Option<Uuid>,
    /// Filter by regency ID
    pub regency_id: Option<Uuid>,
    /// Page number (1-indexed)
    #[serde(default = "default_page")]
    #[param(minimum = 1)]
    pub page: i64,
    /// Number of items per page
    #[serde(default = "default_page_size")]
    #[param(minimum = 1, maximum = 100)]
    pub page_size: i64,
}

impl LocationQueryParams {
    pub fn offset(&self) -> i64 {
        (self.page.max(1) - 1) * self.limit()
    }

    pub fn limit(&self) -> i64 {
        self.page_size.clamp(1, MAX_PAGE_SIZE)
    }
}

/// Province with report count
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ProvinceReportSummary {
    pub id: Uuid,
    pub name: String,
    pub code: String,
    pub lat: Option<f64>,
    pub lng: Option<f64>,
    pub report_count: i64,
}

/// Regency with report count
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RegencyReportSummary {
    pub id: Uuid,
    pub province_id: Uuid,
    pub name: String,
    pub code: String,
    pub lat: Option<f64>,
    pub lng: Option<f64>,
    pub report_count: i64,
}

/// Location overview with provinces and optional regencies
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DashboardLocationOverviewDto {
    pub provinces: Vec<ProvinceReportSummary>,
    /// Regencies (only if province_id filter applied)
    pub regencies: Option<Vec<RegencyReportSummary>>,
    /// Reports (only if regency_id filter applied)
    pub reports: Option<Vec<DashboardReportDto>>,
    pub pagination: Option<PaginationMeta>,
}

// ============================================================================
// By Category
// ============================================================================

/// Query params for category-based listing
#[derive(Debug, Clone, Deserialize, IntoParams)]
pub struct CategoryQueryParams {
    /// Category slug to filter by
    pub slug: Option<String>,
    /// Page number (1-indexed)
    #[serde(default = "default_page")]
    #[param(minimum = 1)]
    pub page: i64,
    /// Number of items per page
    #[serde(default = "default_page_size")]
    #[param(minimum = 1, maximum = 100)]
    pub page_size: i64,
}

impl CategoryQueryParams {
    pub fn offset(&self) -> i64 {
        (self.page.max(1) - 1) * self.limit()
    }

    pub fn limit(&self) -> i64 {
        self.page_size.clamp(1, MAX_PAGE_SIZE)
    }
}

/// Category with report count
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CategoryReportSummary {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub color: Option<String>,
    pub icon: Option<String>,
    pub report_count: i64,
}

/// Category overview
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DashboardCategoryOverviewDto {
    pub categories: Vec<CategoryReportSummary>,
    /// Reports (only if slug filter applied)
    pub reports: Option<Vec<DashboardReportDto>>,
    pub pagination: Option<PaginationMeta>,
}

// ============================================================================
// By Tag
// ============================================================================

/// Query params for tag-based listing
#[derive(Debug, Clone, Deserialize, IntoParams)]
pub struct TagQueryParams {
    /// Tag type to filter by
    pub tag_type: Option<ReportTagType>,
    /// Page number (1-indexed)
    #[serde(default = "default_page")]
    #[param(minimum = 1)]
    pub page: i64,
    /// Number of items per page
    #[serde(default = "default_page_size")]
    #[param(minimum = 1, maximum = 100)]
    pub page_size: i64,
}

impl TagQueryParams {
    pub fn offset(&self) -> i64 {
        (self.page.max(1) - 1) * self.limit()
    }

    pub fn limit(&self) -> i64 {
        self.page_size.clamp(1, MAX_PAGE_SIZE)
    }
}

/// Tag type with report count
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TagReportSummary {
    pub tag_type: ReportTagType,
    pub label: String,
    pub report_count: i64,
}

/// Tag overview
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DashboardTagOverviewDto {
    pub tags: Vec<TagReportSummary>,
    /// Reports (only if tag_type filter applied)
    pub reports: Option<Vec<DashboardReportDto>>,
    pub pagination: Option<PaginationMeta>,
}

// ============================================================================
// Recent Reports
// ============================================================================

/// Query params for recent reports
#[derive(Debug, Clone, Deserialize, IntoParams)]
pub struct RecentQueryParams {
    /// Number of days to look back (default: 7)
    #[serde(default = "default_days")]
    pub days: i32,
    /// Maximum reports to return
    #[serde(default = "default_limit")]
    pub limit: i64,
}

fn default_days() -> i32 {
    7
}

fn default_limit() -> i64 {
    50
}

/// Recent reports response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DashboardRecentDto {
    pub reports: Vec<DashboardReportDto>,
    pub days: i32,
    pub total_count: i64,
}

// ============================================================================
// Map View
// ============================================================================

/// Report marker for map
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MapReportMarker {
    pub id: Uuid,
    pub title: String,
    pub lat: f64,
    pub lon: f64,
    pub status: ReportStatus,
    pub category_slug: Option<String>,
    pub category_color: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Map data response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DashboardMapDto {
    pub markers: Vec<MapReportMarker>,
    pub total_count: i64,
    /// Bounding box [min_lat, min_lon, max_lat, max_lon]
    pub bounds: Option<[f64; 4]>,
}

/// Query params for map
#[derive(Debug, Clone, Deserialize, IntoParams)]
pub struct MapQueryParams {
    /// Filter by province ID
    pub province_id: Option<Uuid>,
    /// Filter by regency ID
    pub regency_id: Option<Uuid>,
    /// Filter by category slug
    pub category: Option<String>,
    /// Filter by status
    pub status: Option<ReportStatus>,
    /// Maximum markers to return
    #[serde(default = "default_map_limit")]
    pub limit: i64,
}

fn default_map_limit() -> i64 {
    500
}

// ============================================================================
// Summary (lightweight stats for header/overview)
// ============================================================================

/// Lightweight summary for dashboard header
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DashboardSummaryDto {
    pub total_reports: i64,
    pub pending_count: i64,
    pub resolved_count: i64,
    pub reports_this_week: i64,
    pub reports_this_month: i64,
}
