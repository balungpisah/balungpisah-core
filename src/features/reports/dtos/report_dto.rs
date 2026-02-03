use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

use crate::features::reports::models::{
    ClusterStatus, GeocodingSource, Report, ReportCategory, ReportCluster, ReportLocation,
    ReportSeverity, ReportStatus, ReportTag, ReportTagType,
};

/// Response DTO for a category assigned to a report
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ReportCategoryDto {
    pub category_id: Uuid,
    pub category_name: Option<String>,
    pub category_slug: Option<String>,
    pub severity: ReportSeverity,
}

impl From<ReportCategory> for ReportCategoryDto {
    fn from(rc: ReportCategory) -> Self {
        Self {
            category_id: rc.category_id,
            category_name: None, // Will be populated by handler
            category_slug: None, // Will be populated by handler
            severity: rc.severity,
        }
    }
}

/// Response DTO for a tag assigned to a report
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ReportTagDto {
    pub tag_type: ReportTagType,
}

impl From<ReportTag> for ReportTagDto {
    fn from(rt: ReportTag) -> Self {
        Self {
            tag_type: rt.tag_type,
        }
    }
}

/// Response DTO for report
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ReportResponseDto {
    pub id: Uuid,
    /// Human-readable reference number for tracking
    pub reference_number: Option<String>,
    pub ticket_id: Option<Uuid>,
    pub cluster_id: Option<Uuid>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub timeline: Option<String>,
    pub impact: Option<String>,
    pub status: ReportStatus,
    pub verified_at: Option<DateTime<Utc>>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    /// Categories assigned to this report with their severities
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub categories: Vec<ReportCategoryDto>,
    /// Tags assigned to this report
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<ReportTagDto>,
}

impl From<Report> for ReportResponseDto {
    fn from(r: Report) -> Self {
        Self {
            id: r.id,
            reference_number: r.reference_number,
            ticket_id: r.ticket_id,
            cluster_id: r.cluster_id,
            title: r.title,
            description: r.description,
            timeline: r.timeline,
            impact: r.impact,
            status: r.status,
            verified_at: r.verified_at,
            resolved_at: r.resolved_at,
            created_at: r.created_at,
            updated_at: r.updated_at,
            categories: Vec::new(), // Will be populated by handler
            tags: Vec::new(),       // Will be populated by handler
        }
    }
}

/// Response DTO for report with location
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ReportDetailResponseDto {
    #[serde(flatten)]
    pub report: ReportResponseDto,
    pub location: Option<ReportLocationResponseDto>,
}

/// Response DTO for report location
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ReportLocationResponseDto {
    pub id: Uuid,
    pub raw_input: String,
    pub display_name: Option<String>,
    pub lat: Option<f64>,
    pub lon: Option<f64>,
    pub road: Option<String>,
    pub neighbourhood: Option<String>,
    pub suburb: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub postcode: Option<String>,
    pub geocoding_source: GeocodingSource,
    pub geocoding_score: Option<f64>,
    // Region names resolved from FKs
    pub province_name: Option<String>,
    pub regency_name: Option<String>,
    pub district_name: Option<String>,
    pub village_name: Option<String>,
}

impl From<ReportLocation> for ReportLocationResponseDto {
    fn from(l: ReportLocation) -> Self {
        Self {
            id: l.id,
            raw_input: l.raw_input,
            display_name: l.display_name,
            lat: l.lat,
            lon: l.lon,
            road: l.road,
            neighbourhood: l.neighbourhood,
            suburb: l.suburb,
            city: l.city,
            state: l.state,
            postcode: l.postcode,
            geocoding_source: l.geocoding_source,
            geocoding_score: l
                .geocoding_score
                .map(|s| s.to_string().parse().unwrap_or(0.0)),
            // Region names will be populated by handler/service
            province_name: None,
            regency_name: None,
            district_name: None,
            village_name: None,
        }
    }
}

/// Response DTO for report cluster
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ReportClusterResponseDto {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub center_lat: f64,
    pub center_lon: f64,
    pub radius_meters: i32,
    pub report_count: i32,
    pub status: ClusterStatus,
    pub created_at: DateTime<Utc>,
}

impl From<ReportCluster> for ReportClusterResponseDto {
    fn from(c: ReportCluster) -> Self {
        Self {
            id: c.id,
            name: c.name,
            description: c.description,
            center_lat: c.center_lat,
            center_lon: c.center_lon,
            radius_meters: c.radius_meters,
            report_count: c.report_count,
            status: c.status,
            created_at: c.created_at,
        }
    }
}

/// Response DTO for cluster with reports
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ClusterDetailResponseDto {
    #[serde(flatten)]
    pub cluster: ReportClusterResponseDto,
    pub reports: Vec<ReportResponseDto>,
}

/// Request DTO for updating report status
#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct UpdateReportStatusDto {
    pub status: ReportStatus,
    pub resolution_notes: Option<String>,
}
