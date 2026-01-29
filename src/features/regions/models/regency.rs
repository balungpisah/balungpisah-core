use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::FromRow;
use uuid::Uuid;

/// Regency model representing Indonesian regencies/cities (kabupaten/kota)
#[derive(Debug, Clone, FromRow, Serialize)]
pub struct Regency {
    pub id: Uuid,
    pub code: String,
    pub name: String,
    pub lat: Option<f64>,
    pub lng: Option<f64>,
    pub province_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
