use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::FromRow;
use uuid::Uuid;

/// Village model representing Indonesian villages (kelurahan/desa)
#[derive(Debug, Clone, FromRow, Serialize)]
pub struct Village {
    pub id: Uuid,
    pub code: String,
    pub name: String,
    pub lat: Option<f64>,
    pub lng: Option<f64>,
    pub district_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
