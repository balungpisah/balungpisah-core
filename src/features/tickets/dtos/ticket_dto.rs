use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::features::tickets::models::{Ticket, TicketStatus};

/// Response DTO for ticket
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TicketResponseDto {
    pub id: Uuid,
    pub reference_number: String,
    pub adk_thread_id: Uuid,
    pub platform: String,
    pub status: TicketStatus,
    pub confidence_score: f64,
    pub completeness_score: Option<f64>,
    pub submitted_at: DateTime<Utc>,
    pub processed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl From<Ticket> for TicketResponseDto {
    fn from(t: Ticket) -> Self {
        use rust_decimal::prelude::ToPrimitive;

        Self {
            id: t.id,
            reference_number: t.reference_number,
            adk_thread_id: t.adk_thread_id,
            platform: t.platform,
            status: t.status,
            confidence_score: t.confidence_score.to_f64().unwrap_or(0.5),
            completeness_score: t.completeness_score.and_then(|s| s.to_f64()),
            submitted_at: t.submitted_at,
            processed_at: t.processed_at,
            created_at: t.created_at,
        }
    }
}

/// DTO for creating ticket from agent tool
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct CreateTicketFromAgentDto {
    pub adk_thread_id: Uuid,
    pub user_id: String,
    pub confidence: f64,
    pub platform: Option<String>,
}

/// Simplified response for agent tool (kept for reference - ticket creation disabled)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct TicketCreatedDto {
    pub reference_number: String,
    pub ticket_id: Uuid,
}
