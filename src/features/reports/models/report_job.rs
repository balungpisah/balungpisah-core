use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Type};
use utoipa::ToSchema;
use uuid::Uuid;

/// Report job status enum matching database enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type, ToSchema)]
#[sqlx(type_name = "report_job_status", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum ReportJobStatus {
    Submitted,
    Processing,
    Completed,
    Failed,
}

impl std::fmt::Display for ReportJobStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReportJobStatus::Submitted => write!(f, "submitted"),
            ReportJobStatus::Processing => write!(f, "processing"),
            ReportJobStatus::Completed => write!(f, "completed"),
            ReportJobStatus::Failed => write!(f, "failed"),
        }
    }
}

/// Database model for report job (background processing queue)
#[derive(Debug, Clone, FromRow)]
#[allow(dead_code)]
pub struct ReportJob {
    pub id: Uuid,
    pub report_id: Uuid,
    pub status: ReportJobStatus,
    pub confidence_score: Option<Decimal>,
    pub retry_count: i32,
    pub error_message: Option<String>,
    pub submitted_at: DateTime<Utc>,
    pub processed_at: Option<DateTime<Utc>>,
    pub last_attempt_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// Data for creating a new report job
#[derive(Debug)]
pub struct CreateReportJob {
    pub report_id: Uuid,
    pub confidence_score: Option<f64>,
}
