use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Type};
use utoipa::ToSchema;
use uuid::Uuid;

/// Cluster status enum matching database enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type, ToSchema)]
#[sqlx(type_name = "cluster_status", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum ClusterStatus {
    Active,
    Monitoring,
    Resolved,
    Archived,
}

impl std::fmt::Display for ClusterStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ClusterStatus::Active => write!(f, "active"),
            ClusterStatus::Monitoring => write!(f, "monitoring"),
            ClusterStatus::Resolved => write!(f, "resolved"),
            ClusterStatus::Archived => write!(f, "archived"),
        }
    }
}

/// Database model for report cluster
#[derive(Debug, Clone, FromRow)]
#[allow(dead_code)]
pub struct ReportCluster {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub center_lat: f64,
    pub center_lon: f64,
    pub radius_meters: i32,
    pub report_count: i32,
    pub status: ClusterStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Data for creating a new cluster
#[derive(Debug)]
pub struct CreateCluster {
    pub name: String,
    pub description: Option<String>,
    pub center_lat: f64,
    pub center_lon: f64,
    pub radius_meters: i32,
}
