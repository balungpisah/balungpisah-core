use chrono::{DateTime, Utc};
use sqlx::FromRow;
use uuid::Uuid;

/// Database model for report attachment junction table
#[derive(Debug, Clone, FromRow)]
#[allow(dead_code)]
pub struct ReportAttachment {
    pub id: Uuid,
    pub report_id: Uuid,
    pub file_id: Uuid,
    pub created_at: DateTime<Utc>,
}

/// Data for creating a new report attachment link
#[derive(Debug, Clone)]
pub struct CreateReportAttachment {
    pub report_id: Uuid,
    pub file_id: Uuid,
}
