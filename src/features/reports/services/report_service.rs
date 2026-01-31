use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use crate::core::error::{AppError, Result};
use crate::features::reports::dtos::UpdateReportStatusDto;
use crate::features::reports::models::{
    CreateReport, CreateReportLocation, GeocodingSource, Report, ReportLocation, ReportSeverity,
    ReportStatus,
};

/// Service for report operations
pub struct ReportService {
    pool: PgPool,
}

impl ReportService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Create a new report
    pub async fn create(&self, data: &CreateReport) -> Result<Report> {
        let report = sqlx::query_as!(
            Report,
            r#"
            INSERT INTO reports (ticket_id, title, description, category_id, severity, timeline, impact)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING
                id, ticket_id, cluster_id, title, description, category_id,
                severity as "severity: ReportSeverity",
                timeline, impact,
                status as "status: ReportStatus",
                verified_at, verified_by, resolved_at, resolved_by, resolution_notes,
                created_at, updated_at
            "#,
            data.ticket_id,
            data.title,
            data.description,
            data.category_id,
            data.severity as Option<ReportSeverity>,
            data.timeline,
            data.impact
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to create report: {:?}", e);
            AppError::Database(e)
        })?;

        tracing::info!(
            "Created report: {} for ticket: {}",
            report.id,
            data.ticket_id
        );

        Ok(report)
    }

    /// Create a report location
    pub async fn create_location(&self, data: &CreateReportLocation) -> Result<ReportLocation> {
        let location = sqlx::query_as!(
            ReportLocation,
            r#"
            INSERT INTO report_locations (
                report_id, raw_input, display_name, lat, lon,
                osm_id, osm_type, road, neighbourhood, suburb, city, state, postcode, country_code,
                bounding_box, geocoding_source, geocoding_score, geocoded_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, NOW())
            RETURNING
                id, report_id, raw_input, display_name, lat, lon,
                osm_id, osm_type, road, neighbourhood, suburb, city, state, postcode, country_code,
                bounding_box, geocoding_source as "geocoding_source: GeocodingSource",
                geocoding_score, geocoded_at, created_at
            "#,
            data.report_id,
            data.raw_input,
            data.display_name,
            data.lat,
            data.lon,
            data.osm_id,
            data.osm_type,
            data.road,
            data.neighbourhood,
            data.suburb,
            data.city,
            data.state,
            data.postcode,
            data.country_code,
            data.bounding_box,
            data.geocoding_source as GeocodingSource,
            data.geocoding_score
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to create report location: {:?}", e);
            AppError::Database(e)
        })?;

        Ok(location)
    }

    /// Get report by ID
    pub async fn get_by_id(&self, id: Uuid) -> Result<Report> {
        sqlx::query_as!(
            Report,
            r#"
            SELECT
                id, ticket_id, cluster_id, title, description, category_id,
                severity as "severity: ReportSeverity",
                timeline, impact,
                status as "status: ReportStatus",
                verified_at, verified_by, resolved_at, resolved_by, resolution_notes,
                created_at, updated_at
            FROM reports
            WHERE id = $1
            "#,
            id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get report: {:?}", e);
            AppError::Database(e)
        })?
        .ok_or_else(|| AppError::NotFound(format!("Report {} not found", id)))
    }

    /// Get report by ticket ID
    #[allow(dead_code)]
    pub async fn get_by_ticket_id(&self, ticket_id: Uuid) -> Result<Option<Report>> {
        sqlx::query_as!(
            Report,
            r#"
            SELECT
                id, ticket_id, cluster_id, title, description, category_id,
                severity as "severity: ReportSeverity",
                timeline, impact,
                status as "status: ReportStatus",
                verified_at, verified_by, resolved_at, resolved_by, resolution_notes,
                created_at, updated_at
            FROM reports
            WHERE ticket_id = $1
            "#,
            ticket_id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get report by ticket: {:?}", e);
            AppError::Database(e)
        })
    }

    /// Get location for a report
    pub async fn get_location(&self, report_id: Uuid) -> Result<Option<ReportLocation>> {
        sqlx::query_as!(
            ReportLocation,
            r#"
            SELECT
                id, report_id, raw_input, display_name, lat, lon,
                osm_id, osm_type, road, neighbourhood, suburb, city, state, postcode, country_code,
                bounding_box, geocoding_source as "geocoding_source: GeocodingSource",
                geocoding_score, geocoded_at, created_at
            FROM report_locations
            WHERE report_id = $1
            "#,
            report_id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get report location: {:?}", e);
            AppError::Database(e)
        })
    }

    /// List reports by user
    pub async fn list_by_user(&self, user_id: &str) -> Result<Vec<Report>> {
        sqlx::query_as!(
            Report,
            r#"
            SELECT
                r.id, r.ticket_id, r.cluster_id, r.title, r.description, r.category_id,
                r.severity as "severity: ReportSeverity",
                r.timeline, r.impact,
                r.status as "status: ReportStatus",
                r.verified_at, r.verified_by, r.resolved_at, r.resolved_by, r.resolution_notes,
                r.created_at, r.updated_at
            FROM reports r
            JOIN tickets t ON t.id = r.ticket_id
            WHERE t.user_id = $1
            ORDER BY r.created_at DESC
            "#,
            user_id
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to list reports by user: {:?}", e);
            AppError::Database(e)
        })
    }

    /// List reports by cluster
    pub async fn list_by_cluster(&self, cluster_id: Uuid) -> Result<Vec<Report>> {
        sqlx::query_as!(
            Report,
            r#"
            SELECT
                id, ticket_id, cluster_id, title, description, category_id,
                severity as "severity: ReportSeverity",
                timeline, impact,
                status as "status: ReportStatus",
                verified_at, verified_by, resolved_at, resolved_by, resolution_notes,
                created_at, updated_at
            FROM reports
            WHERE cluster_id = $1
            ORDER BY created_at DESC
            "#,
            cluster_id
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to list reports by cluster: {:?}", e);
            AppError::Database(e)
        })
    }

    /// Update report cluster assignment
    pub async fn set_cluster(&self, report_id: Uuid, cluster_id: Uuid) -> Result<()> {
        sqlx::query!(
            r#"
            UPDATE reports
            SET cluster_id = $2, updated_at = NOW()
            WHERE id = $1
            "#,
            report_id,
            cluster_id
        )
        .execute(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to set report cluster: {:?}", e);
            AppError::Database(e)
        })?;

        Ok(())
    }

    /// Update report status
    pub async fn update_status(
        &self,
        id: Uuid,
        dto: &UpdateReportStatusDto,
        user_id: &str,
    ) -> Result<Report> {
        let now = Utc::now();

        // Set verified_at/resolved_at based on status transition
        let (verified_at, verified_by, resolved_at, resolved_by) = match dto.status {
            ReportStatus::Verified => (Some(now), Some(user_id.to_string()), None, None),
            ReportStatus::Resolved => (None, None, Some(now), Some(user_id.to_string())),
            _ => (None, None, None, None),
        };

        sqlx::query_as!(
            Report,
            r#"
            UPDATE reports
            SET
                status = $2,
                resolution_notes = COALESCE($3, resolution_notes),
                verified_at = COALESCE($4, verified_at),
                verified_by = COALESCE($5, verified_by),
                resolved_at = COALESCE($6, resolved_at),
                resolved_by = COALESCE($7, resolved_by),
                updated_at = NOW()
            WHERE id = $1
            RETURNING
                id, ticket_id, cluster_id, title, description, category_id,
                severity as "severity: ReportSeverity",
                timeline, impact,
                status as "status: ReportStatus",
                verified_at, verified_by, resolved_at, resolved_by, resolution_notes,
                created_at, updated_at
            "#,
            id,
            dto.status as ReportStatus,
            dto.resolution_notes,
            verified_at,
            verified_by,
            resolved_at,
            resolved_by
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to update report status: {:?}", e);
            AppError::Database(e)
        })?
        .ok_or_else(|| AppError::NotFound(format!("Report {} not found", id)))
    }
}
