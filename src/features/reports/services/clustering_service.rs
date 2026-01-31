use sqlx::PgPool;
use uuid::Uuid;

use crate::core::error::{AppError, Result};
use crate::features::reports::models::{ClusterStatus, CreateCluster, ReportCluster};

/// Default clustering radius in meters
const DEFAULT_CLUSTER_RADIUS_METERS: i32 = 500;

/// Earth's radius in meters (for Haversine formula)
const EARTH_RADIUS_METERS: f64 = 6_371_000.0;

/// Service for clustering reports by location
pub struct ClusteringService {
    pool: PgPool,
}

impl ClusteringService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Calculate Haversine distance between two points in meters
    pub fn haversine_distance(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
        let lat1_rad = lat1.to_radians();
        let lat2_rad = lat2.to_radians();
        let delta_lat = (lat2 - lat1).to_radians();
        let delta_lon = (lon2 - lon1).to_radians();

        let a = (delta_lat / 2.0).sin().powi(2)
            + lat1_rad.cos() * lat2_rad.cos() * (delta_lon / 2.0).sin().powi(2);
        let c = 2.0 * a.sqrt().asin();

        EARTH_RADIUS_METERS * c
    }

    /// Find nearby active clusters within the default radius
    pub async fn find_nearby_clusters(&self, lat: f64, lon: f64) -> Result<Vec<ReportCluster>> {
        // Use a bounding box approximation for initial filtering
        // 1 degree of latitude is approximately 111km
        // For longitude, it varies by latitude, but we use a conservative estimate
        let lat_delta = (DEFAULT_CLUSTER_RADIUS_METERS as f64 / 111_000.0) * 2.0;
        let lon_delta = lat_delta / lat.to_radians().cos().abs().max(0.01);

        let clusters = sqlx::query_as!(
            ReportCluster,
            r#"
            SELECT
                id, name, description, center_lat, center_lon,
                radius_meters, report_count,
                status as "status: ClusterStatus",
                created_at, updated_at
            FROM report_clusters
            WHERE status = 'active'
            AND center_lat BETWEEN $1 AND $2
            AND center_lon BETWEEN $3 AND $4
            "#,
            lat - lat_delta,
            lat + lat_delta,
            lon - lon_delta,
            lon + lon_delta
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to find nearby clusters: {:?}", e);
            AppError::Database(e)
        })?;

        // Filter by actual distance using Haversine
        let nearby: Vec<ReportCluster> = clusters
            .into_iter()
            .filter(|c| {
                let distance = Self::haversine_distance(lat, lon, c.center_lat, c.center_lon);
                distance <= c.radius_meters as f64
            })
            .collect();

        Ok(nearby)
    }

    /// Find or create a cluster for a report location
    pub async fn find_or_create_cluster(
        &self,
        lat: f64,
        lon: f64,
        location_name: Option<&str>,
    ) -> Result<Uuid> {
        // Find nearby clusters
        let nearby = self.find_nearby_clusters(lat, lon).await?;

        if let Some(cluster) = nearby.into_iter().next() {
            // Add to existing cluster and update centroid
            self.add_to_cluster(cluster.id, lat, lon).await?;
            Ok(cluster.id)
        } else {
            // Create new cluster
            let name = location_name
                .map(|n| format!("Cluster: {}", n))
                .unwrap_or_else(|| format!("Cluster: {:.4}, {:.4}", lat, lon));

            let create = CreateCluster {
                name,
                description: None,
                center_lat: lat,
                center_lon: lon,
                radius_meters: DEFAULT_CLUSTER_RADIUS_METERS,
            };

            self.create_cluster(&create).await
        }
    }

    /// Create a new cluster
    pub async fn create_cluster(&self, data: &CreateCluster) -> Result<Uuid> {
        let id = sqlx::query_scalar!(
            r#"
            INSERT INTO report_clusters (name, description, center_lat, center_lon, radius_meters, report_count)
            VALUES ($1, $2, $3, $4, $5, 1)
            RETURNING id
            "#,
            data.name,
            data.description,
            data.center_lat,
            data.center_lon,
            data.radius_meters
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to create cluster: {:?}", e);
            AppError::Database(e)
        })?;

        tracing::info!(
            "Created new cluster: {} at ({}, {})",
            id,
            data.center_lat,
            data.center_lon
        );

        Ok(id)
    }

    /// Add a report to an existing cluster and update the centroid
    pub async fn add_to_cluster(&self, cluster_id: Uuid, lat: f64, lon: f64) -> Result<()> {
        // Use weighted average to update centroid
        // new_center = (old_center * old_count + new_point) / (old_count + 1)
        sqlx::query!(
            r#"
            UPDATE report_clusters
            SET
                center_lat = (center_lat * report_count + $2) / (report_count + 1),
                center_lon = (center_lon * report_count + $3) / (report_count + 1),
                report_count = report_count + 1,
                updated_at = NOW()
            WHERE id = $1
            "#,
            cluster_id,
            lat,
            lon
        )
        .execute(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to update cluster: {:?}", e);
            AppError::Database(e)
        })?;

        tracing::debug!("Added report to cluster: {}", cluster_id);

        Ok(())
    }

    /// Get cluster by ID
    pub async fn get_by_id(&self, id: Uuid) -> Result<ReportCluster> {
        sqlx::query_as!(
            ReportCluster,
            r#"
            SELECT
                id, name, description, center_lat, center_lon,
                radius_meters, report_count,
                status as "status: ClusterStatus",
                created_at, updated_at
            FROM report_clusters
            WHERE id = $1
            "#,
            id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get cluster: {:?}", e);
            AppError::Database(e)
        })?
        .ok_or_else(|| AppError::NotFound(format!("Cluster {} not found", id)))
    }

    /// List active clusters
    pub async fn list_active(&self) -> Result<Vec<ReportCluster>> {
        sqlx::query_as!(
            ReportCluster,
            r#"
            SELECT
                id, name, description, center_lat, center_lon,
                radius_meters, report_count,
                status as "status: ClusterStatus",
                created_at, updated_at
            FROM report_clusters
            WHERE status = 'active'
            ORDER BY report_count DESC, created_at DESC
            "#
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to list clusters: {:?}", e);
            AppError::Database(e)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_haversine_distance() {
        // Test with known coordinates (Jakarta to Bandung, approx 116km by Haversine)
        let jakarta = (-6.2088, 106.8456);
        let bandung = (-6.9175, 107.6191);

        let distance =
            ClusteringService::haversine_distance(jakarta.0, jakarta.1, bandung.0, bandung.1);

        // Should be approximately 116km (road distance is ~140km but Haversine is ~116km)
        assert!(distance > 110_000.0 && distance < 125_000.0);
    }

    #[test]
    fn test_haversine_same_point() {
        let distance = ClusteringService::haversine_distance(-6.2088, 106.8456, -6.2088, 106.8456);

        assert!(distance < 1.0); // Less than 1 meter
    }
}
