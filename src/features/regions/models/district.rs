use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::FromRow;
use uuid::Uuid;

/// District model representing Indonesian districts (kecamatan)
#[derive(Debug, Clone, FromRow, Serialize)]
pub struct District {
    pub id: Uuid,
    pub code: String,
    pub name: String,
    pub lat: Option<f64>,
    pub lng: Option<f64>,
    pub regency_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
