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

// ============================================================================
// Report List Params
// ============================================================================

/// Query params for report listing with sort and search
#[derive(Debug, Clone, Deserialize, IntoParams)]
pub struct ReportListParams {
    /// Page number (1-indexed)
    #[serde(default = "default_page")]
    #[param(minimum = 1)]
    pub page: i64,
    /// Number of items per page
    #[serde(default = "default_page_size")]
    #[param(minimum = 1, maximum = 100)]
    pub page_size: i64,
    /// Sort by field (created_at, title, status)
    pub sort_by: Option<String>,
    /// Sort direction (asc, desc)
    pub sort: Option<String>,
    /// Search by report title
    pub search: Option<String>,
}

impl ReportListParams {
    pub fn offset(&self) -> i64 {
        (self.page.max(1) - 1) * self.limit()
    }

    pub fn limit(&self) -> i64 {
        self.page_size.clamp(1, MAX_PAGE_SIZE)
    }

    /// Returns validated sort column, defaults to "created_at"
    pub fn sort_column(&self) -> &str {
        match self.sort_by.as_deref() {
            Some("title") => "r.title",
            Some("status") => "r.status",
            _ => "r.created_at",
        }
    }

    /// Returns validated sort direction, defaults to "DESC"
    pub fn sort_direction(&self) -> &str {
        match self.sort.as_deref() {
            Some(d) if d.eq_ignore_ascii_case("asc") => "ASC",
            _ => "DESC",
        }
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
    /// Report title (may be None for unprocessed reports)
    pub title: Option<String>,
    /// Report description (may be None for unprocessed reports)
    pub description: Option<String>,
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
    /// Report reference number
    pub reference_number: Option<String>,
    /// Report title (may be None for unprocessed reports)
    pub title: Option<String>,
    /// Report description (may be None for unprocessed reports)
    pub description: Option<String>,
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
    /// Page number (1-indexed)
    #[serde(default = "default_page")]
    #[param(minimum = 1)]
    pub page: i64,
    /// Number of items per page
    #[serde(default = "default_page_size")]
    #[param(minimum = 1, maximum = 100)]
    pub page_size: i64,
    /// Alias for page_size (backward compatibility)
    pub limit: Option<i64>,
    /// Search by report title
    pub search: Option<String>,
}

impl RecentQueryParams {
    /// Effective page size: `limit` overrides `page_size` when provided
    fn effective_page_size(&self) -> i64 {
        self.limit.unwrap_or(self.page_size).clamp(1, MAX_PAGE_SIZE)
    }

    pub fn offset(&self) -> i64 {
        (self.page.max(1) - 1) * self.effective_page_size()
    }

    pub fn limit(&self) -> i64 {
        self.effective_page_size()
    }
}

fn default_days() -> i32 {
    7
}

/// Recent reports response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DashboardRecentDto {
    pub reports: Vec<DashboardReportDto>,
    pub days: i32,
    pub total_count: i64,
    pub pagination: PaginationMeta,
}

// ============================================================================
// Map View
// ============================================================================

/// Report marker for map
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MapReportMarker {
    pub id: Uuid,
    /// Report title (may be None for unprocessed reports)
    pub title: Option<String>,
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

// Query Data for map

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MapPointDto {
    pub id: Uuid,
    pub lat: f64,
    pub lon: f64,
    pub status: ReportStatus,
    pub category_color: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DashboardMapDataDto {
    pub points: Vec<MapPointDto>,
}

// ============================================================================
// Enhanced Map Markers (FASE 1)
// ============================================================================

/// Enhanced query params for map markers with comprehensive filtering
#[derive(Debug, Clone, Deserialize, IntoParams, ToSchema)]
pub struct EnhancedMapQueryParams {
    /// Filter by province ID
    pub province_id: Option<Uuid>,
    /// Filter by regency ID
    pub regency_id: Option<Uuid>,
    /// Filter by district ID
    pub district_id: Option<Uuid>,
    /// Filter by village ID
    pub village_id: Option<Uuid>,
    /// Filter by category IDs (comma-separated UUIDs)
    pub category_ids: Option<String>,
    /// Filter by severity (comma-separated: low,medium,high,critical)
    pub severity: Option<String>,
    /// Filter by tag types (comma-separated: report,complaint,proposal,inquiry,appreciation)
    pub tag_types: Option<String>,
    /// Filter by status (comma-separated: verified,pending,rejected)
    pub status: Option<String>,
    /// Filter by date from (ISO 8601)
    pub date_from: Option<String>,
    /// Filter by date to (ISO 8601)
    pub date_to: Option<String>,
    /// Viewport bounds (sw_lat,sw_lon,ne_lat,ne_lon)
    pub bounds: Option<String>,
    /// Maximum markers to return
    #[serde(default = "default_map_limit")]
    pub limit: i64,
}

/// Enhanced map marker with severity and category info
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct EnhancedMapMarker {
    pub id: Uuid,
    pub reference_number: Option<String>,
    pub title: Option<String>,
    pub lat: f64,
    pub lon: f64,
    pub max_severity: ReportSeverity,
    pub primary_category_name: Option<String>,
    pub primary_category_color: Option<String>,
    pub tag_type: Option<ReportTagType>,
    pub status: ReportStatus,
    pub created_at: DateTime<Utc>,
}

/// Enhanced map response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct EnhancedMapDto {
    pub markers: Vec<EnhancedMapMarker>,
    pub total_count: i64,
}

// ============================================================================
// Comprehensive Stats (FASE 1)
// ============================================================================

/// Breakdown by severity
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SeverityBreakdown {
    pub critical: i64,
    pub high: i64,
    pub medium: i64,
    pub low: i64,
}

/// Breakdown by status
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct StatusBreakdown {
    pub draft: i64,
    pub verified: i64,
}

/// Breakdown by tag type
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TagBreakdown {
    pub report: i64,
    pub complaint: i64,
    pub proposal: i64,
    pub inquiry: i64,
    pub appreciation: i64,
}

/// Category count for top categories
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CategoryCount {
    pub category_id: Uuid,
    pub category_name: String,
    pub count: i64,
}

/// Weekly count for trend chart
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WeeklyCount {
    pub week: String, // Format: "2026-W05"
    pub count: i64,
}

/// Region count for regional breakdown
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RegionCount {
    pub region_id: Uuid,
    pub region_name: String,
    pub region_type: String, // "province" | "regency"
    pub count: i64,
}

/// Comprehensive dashboard statistics
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ComprehensiveStatsDto {
    pub total: i64,
    pub by_severity: SeverityBreakdown,
    pub by_status: StatusBreakdown,
    pub by_tag: TagBreakdown,
    pub by_category: Vec<CategoryCount>,
    pub by_period: Vec<WeeklyCount>,
    pub by_region: Vec<RegionCount>,
}

// ============================================================================
// Cluster Analysis (FASE 2)
// ============================================================================

/// Clustering mode
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum ClusterMode {
    /// Group by geographic proximity
    Geographic,
    /// Group by same category + proximity
    Category,
}

/// Cluster analysis request
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct ClusterRequest {
    /// Clustering mode
    pub mode: ClusterMode,
    /// Radius in kilometers for proximity clustering (default: 1.0)
    #[serde(default = "default_radius_km")]
    pub radius_km: f64,
    /// Same filters as EnhancedMapQueryParams
    #[serde(flatten)]
    pub filters: EnhancedMapQueryParams,
}

fn default_radius_km() -> f64 {
    1.0
}

/// Date range for cluster
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DateRange {
    pub from: DateTime<Utc>,
    pub to: DateTime<Utc>,
}

/// Report cluster information
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ReportCluster {
    /// Generated cluster ID
    pub cluster_id: String,
    /// Auto-generated label: "[category] — [area]"
    pub label: String,
    /// Cluster center latitude
    pub center_lat: f64,
    /// Cluster center longitude
    pub center_lon: f64,
    /// Number of reports in cluster
    pub report_count: i64,
    /// Number of unique citizens
    pub citizen_count: i64,
    /// Highest severity in cluster
    pub max_severity: ReportSeverity,
    /// Most common category
    pub dominant_category: String,
    /// Date range of reports in cluster
    pub date_range: DateRange,
    /// Report IDs in this cluster
    pub report_ids: Vec<Uuid>,
}

/// Cluster analysis response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ClusterAnalysisDto {
    pub clusters: Vec<ReportCluster>,
    pub total_reports: i64,
    pub total_clusters: i64,
}
