use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Type};
use utoipa::ToSchema;
use uuid::Uuid;

/// Report tag type enum matching database enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type, ToSchema, JsonSchema)]
#[sqlx(type_name = "report_tag_type", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum ReportTagType {
    Report,
    Proposal,
    Complaint,
    Inquiry,
    Appreciation,
}

impl std::fmt::Display for ReportTagType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReportTagType::Report => write!(f, "report"),
            ReportTagType::Proposal => write!(f, "proposal"),
            ReportTagType::Complaint => write!(f, "complaint"),
            ReportTagType::Inquiry => write!(f, "inquiry"),
            ReportTagType::Appreciation => write!(f, "appreciation"),
        }
    }
}

/// Database model for report tag
#[derive(Debug, Clone, FromRow)]
#[allow(dead_code)]
pub struct ReportTag {
    pub id: Uuid,
    pub report_id: Uuid,
    pub tag_type: ReportTagType,
    pub created_at: DateTime<Utc>,
}

/// Data for creating a new report tag
#[derive(Debug, Clone)]
pub struct CreateReportTag {
    pub report_id: Uuid,
    pub tag_type: ReportTagType,
}
