use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Rate limit configuration stored in the database
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct RateLimitConfig {
    pub id: Uuid,
    pub key: String,
    pub value: i32,
    pub description: Option<String>,
    pub updated_at: DateTime<Utc>,
    pub updated_by: Option<String>,
}
