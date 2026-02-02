use chrono::{DateTime, Utc};
use sqlx::FromRow;
use uuid::Uuid;

/// Database model for thread attachments
#[derive(Debug, FromRow)]
#[allow(dead_code)]
pub struct ThreadAttachment {
    pub id: Uuid,
    pub thread_id: Uuid,
    pub file_id: Uuid,
    pub owner_id: String,
    pub created_at: DateTime<Utc>,
}
