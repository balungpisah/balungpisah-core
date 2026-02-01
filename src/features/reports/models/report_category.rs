use chrono::{DateTime, Utc};
use sqlx::FromRow;
use uuid::Uuid;

use super::ReportSeverity;

/// Database model for report category junction table
#[derive(Debug, Clone, FromRow)]
#[allow(dead_code)]
pub struct ReportCategory {
    pub id: Uuid,
    pub report_id: Uuid,
    pub category_id: Uuid,
    pub severity: ReportSeverity,
    pub created_at: DateTime<Utc>,
}

/// Data for creating a new report category assignment
#[derive(Debug, Clone)]
pub struct CreateReportCategory {
    pub report_id: Uuid,
    pub category_id: Uuid,
    pub severity: ReportSeverity,
}
