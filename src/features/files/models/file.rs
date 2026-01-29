use chrono::{DateTime, Utc};
use sqlx::FromRow;
use uuid::Uuid;

/// Database model for files
#[derive(Debug, FromRow)]
#[allow(dead_code)]
pub struct File {
    pub id: Uuid,
    pub file_key: String,
    pub original_filename: String,
    pub content_type: String,
    pub file_size: i64,
    pub url: String,
    pub visibility: String,
    pub purpose: Option<String>,
    pub uploaded_by: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
