use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Type};
use utoipa::ToSchema;
use uuid::Uuid;

/// Geocoding source enum matching database enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type, ToSchema)]
#[sqlx(type_name = "geocoding_source", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum GeocodingSource {
    Nominatim,
    Manual,
    Fallback,
}

impl std::fmt::Display for GeocodingSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GeocodingSource::Nominatim => write!(f, "nominatim"),
            GeocodingSource::Manual => write!(f, "manual"),
            GeocodingSource::Fallback => write!(f, "fallback"),
        }
    }
}

/// Database model for report location
#[derive(Debug, Clone, FromRow)]
#[allow(dead_code)]
pub struct ReportLocation {
    pub id: Uuid,
    pub report_id: Uuid,
    pub raw_input: String,
    pub display_name: Option<String>,
    pub lat: Option<f64>,
    pub lon: Option<f64>,
    pub osm_id: Option<i64>,
    pub osm_type: Option<String>,
    pub road: Option<String>,
    pub neighbourhood: Option<String>,
    pub suburb: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub postcode: Option<String>,
    pub country_code: Option<String>,
    pub bounding_box: Option<serde_json::Value>,
    pub geocoding_source: GeocodingSource,
    pub geocoding_score: Option<Decimal>,
    pub geocoded_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    // Regional clustering FKs
    pub province_id: Option<Uuid>,
    pub regency_id: Option<Uuid>,
    pub district_id: Option<Uuid>,
    pub village_id: Option<Uuid>,
}

/// Data for creating a new report location
#[derive(Debug)]
pub struct CreateReportLocation {
    pub report_id: Uuid,
    pub raw_input: String,
    pub display_name: Option<String>,
    pub lat: Option<f64>,
    pub lon: Option<f64>,
    pub osm_id: Option<i64>,
    pub osm_type: Option<String>,
    pub road: Option<String>,
    pub neighbourhood: Option<String>,
    pub suburb: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub postcode: Option<String>,
    pub country_code: Option<String>,
    pub bounding_box: Option<serde_json::Value>,
    pub geocoding_source: GeocodingSource,
    pub geocoding_score: Option<Decimal>,
    // Regional clustering FKs
    pub province_id: Option<Uuid>,
    pub regency_id: Option<Uuid>,
    pub district_id: Option<Uuid>,
    pub village_id: Option<Uuid>,
}
