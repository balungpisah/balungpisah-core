use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Type};
use utoipa::ToSchema;
use uuid::Uuid;

/// Report status enum matching database enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type, ToSchema)]
#[sqlx(type_name = "report_status", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum ReportStatus {
    Draft,
    Pending,
    Verified,
    InProgress,
    Resolved,
    Rejected,
}

impl std::fmt::Display for ReportStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReportStatus::Draft => write!(f, "draft"),
            ReportStatus::Pending => write!(f, "pending"),
            ReportStatus::Verified => write!(f, "verified"),
            ReportStatus::InProgress => write!(f, "in_progress"),
            ReportStatus::Resolved => write!(f, "resolved"),
            ReportStatus::Rejected => write!(f, "rejected"),
        }
    }
}

/// Report severity enum matching database enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type, ToSchema, JsonSchema)]
#[sqlx(type_name = "report_severity", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum ReportSeverity {
    Low,
    Medium,
    High,
    Critical,
}

impl std::fmt::Display for ReportSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReportSeverity::Low => write!(f, "low"),
            ReportSeverity::Medium => write!(f, "medium"),
            ReportSeverity::High => write!(f, "high"),
            ReportSeverity::Critical => write!(f, "critical"),
        }
    }
}

/// Database model for report
#[derive(Debug, Clone, FromRow)]
#[allow(dead_code)]
pub struct Report {
    pub id: Uuid,
    pub ticket_id: Uuid,
    pub cluster_id: Option<Uuid>,
    pub title: String,
    pub description: String,
    pub category_id: Option<Uuid>,
    pub severity: Option<ReportSeverity>,
    pub timeline: Option<String>,
    pub impact: Option<String>,
    pub status: ReportStatus,
    pub verified_at: Option<DateTime<Utc>>,
    pub verified_by: Option<String>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub resolved_by: Option<String>,
    pub resolution_notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Data for creating a new report
#[derive(Debug)]
pub struct CreateReport {
    pub ticket_id: Uuid,
    pub title: String,
    pub description: String,
    pub category_id: Option<Uuid>,
    pub severity: Option<ReportSeverity>,
    pub timeline: Option<String>,
    pub impact: Option<String>,
}
