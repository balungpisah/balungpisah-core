use chrono::{DateTime, Utc};
use sqlx::FromRow;
use uuid::Uuid;

/// Database model for category
#[derive(Debug, Clone, FromRow)]
#[allow(dead_code)]
pub struct Category {
    pub id: Uuid,
    pub parent_id: Option<Uuid>,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub display_order: i32,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
