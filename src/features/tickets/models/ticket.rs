use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Type};
use utoipa::ToSchema;
use uuid::Uuid;

/// Ticket status enum matching database enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type, ToSchema)]
#[sqlx(type_name = "ticket_status", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum TicketStatus {
    Submitted,
    Processing,
    Completed,
    Failed,
}

impl std::fmt::Display for TicketStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TicketStatus::Submitted => write!(f, "submitted"),
            TicketStatus::Processing => write!(f, "processing"),
            TicketStatus::Completed => write!(f, "completed"),
            TicketStatus::Failed => write!(f, "failed"),
        }
    }
}

/// Database model for ticket
#[derive(Debug, Clone, FromRow)]
#[allow(dead_code)]
pub struct Ticket {
    pub id: Uuid,
    pub adk_thread_id: Uuid,
    pub user_id: String,
    pub reference_number: String,
    pub platform: String,
    pub confidence_score: rust_decimal::Decimal,
    pub completeness_score: Option<rust_decimal::Decimal>,
    pub missing_fields: Option<serde_json::Value>,
    pub preliminary_data: Option<serde_json::Value>,
    pub status: TicketStatus,
    pub submitted_at: DateTime<Utc>,
    pub processed_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
    pub retry_count: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
