use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::FromRow;
use uuid::Uuid;

/// Province model representing Indonesian provinces (provinsi)
#[derive(Debug, Clone, FromRow, Serialize)]
pub struct Province {
    pub id: Uuid,
    pub code: String,
    pub name: String,
    pub lat: Option<f64>,
    pub lng: Option<f64>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
