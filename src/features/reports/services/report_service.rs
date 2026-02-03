use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use crate::core::error::{AppError, Result};
use crate::features::reports::dtos::UpdateReportStatusDto;
use crate::features::reports::models::{
    CreateReport, CreateReportAttachment, CreateReportCategory, CreateReportLocation,
    CreateReportSubmission, CreateReportTag, GeocodingSource, Report, ReportAttachment,
    ReportCategory, ReportLocation, ReportSeverity, ReportStatus, ReportTag, ReportTagType,
};

/// Service for report operations
pub struct ReportService {
    pool: PgPool,
}

impl ReportService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Generate a reference number in format: RPT-YYYY-NNNNNNN
    async fn generate_reference_number(&self) -> Result<String> {
        let year = chrono::Utc::now().format("%Y").to_string();

        // Get next value from sequence
        let seq: i64 = sqlx::query_scalar!("SELECT nextval('report_reference_seq')")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| {
                tracing::error!("Failed to get next sequence value: {:?}", e);
                AppError::Database(e)
            })?
            .unwrap_or(1);

        Ok(format!("RPT-{}-{:07}", year, seq))
    }

    /// Create a new report (legacy - from ticket processor)
    pub async fn create(&self, data: &CreateReport) -> Result<Report> {
        let report = sqlx::query_as!(
            Report,
            r#"
            INSERT INTO reports (ticket_id, title, description, timeline, impact)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING
                id, ticket_id, cluster_id, title, description,
                timeline, impact,
                status as "status: ReportStatus",
                verified_at, verified_by, resolved_at, resolved_by, resolution_notes,
                created_at, updated_at,
                reference_number, adk_thread_id, user_id, platform
            "#,
            data.ticket_id,
            data.title,
            data.description,
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

    /// Create a report submission (new workflow - from agent)
    /// Returns the report with reference number for immediate citizen feedback
    /// Status starts as 'pending' - waiting for background processing
    pub async fn create_submission(&self, data: &CreateReportSubmission) -> Result<Report> {
        let report = sqlx::query_as!(
            Report,
            r#"
            INSERT INTO reports (reference_number, adk_thread_id, user_id, platform, status)
            VALUES ($1, $2, $3, $4, 'pending')
            RETURNING
                id, ticket_id, cluster_id, title, description,
                timeline, impact,
                status as "status: ReportStatus",
                verified_at, verified_by, resolved_at, resolved_by, resolution_notes,
                created_at, updated_at,
                reference_number, adk_thread_id, user_id, platform
            "#,
            data.reference_number,
            data.adk_thread_id,
            data.user_id,
            data.platform.as_deref().unwrap_or("web")
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to create report submission: {:?}", e);
            AppError::Database(e)
        })?;

        tracing::info!(
            "Created report submission: {} (ref: {}) for user: {}",
            report.id,
            data.reference_number,
            data.user_id
        );

        Ok(report)
    }

    /// Create a report submission with auto-generated reference number
    pub async fn create_submission_auto_ref(
        &self,
        adk_thread_id: Uuid,
        user_id: &str,
        platform: Option<&str>,
    ) -> Result<Report> {
        let reference_number = self.generate_reference_number().await?;
        let data = CreateReportSubmission {
            reference_number,
            adk_thread_id,
            user_id: user_id.to_string(),
            platform: platform.map(String::from),
        };
        self.create_submission(&data).await
    }

    /// Update report with extracted content (called by ReportProcessor)
    /// Status changes to 'draft' - processed successfully, waiting for verification
    pub async fn update_content(
        &self,
        report_id: Uuid,
        title: &str,
        description: &str,
        timeline: Option<&str>,
        impact: Option<&str>,
    ) -> Result<Report> {
        let report = sqlx::query_as!(
            Report,
            r#"
            UPDATE reports
            SET title = $2, description = $3, timeline = $4, impact = $5,
                status = 'draft', updated_at = NOW()
            WHERE id = $1
            RETURNING
                id, ticket_id, cluster_id, title, description,
                timeline, impact,
                status as "status: ReportStatus",
                verified_at, verified_by, resolved_at, resolved_by, resolution_notes,
                created_at, updated_at,
                reference_number, adk_thread_id, user_id, platform
            "#,
            report_id,
            title,
            description,
            timeline,
            impact
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to update report content: {:?}", e);
            AppError::Database(e)
        })?
        .ok_or_else(|| AppError::NotFound(format!("Report {} not found", report_id)))?;

        tracing::info!("Updated report content: {}", report_id);
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
                bounding_box, geocoding_source, geocoding_score, geocoded_at,
                province_id, regency_id, district_id, village_id
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, NOW(), $18, $19, $20, $21)
            RETURNING
                id, report_id, raw_input, display_name, lat, lon,
                osm_id, osm_type, road, neighbourhood, suburb, city, state, postcode, country_code,
                bounding_box, geocoding_source as "geocoding_source: GeocodingSource",
                geocoding_score, geocoded_at, created_at,
                province_id, regency_id, district_id, village_id
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
            data.geocoding_score,
            data.province_id,
            data.regency_id,
            data.district_id,
            data.village_id
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
                id, ticket_id, cluster_id, title, description,
                timeline, impact,
                status as "status: ReportStatus",
                verified_at, verified_by, resolved_at, resolved_by, resolution_notes,
                created_at, updated_at,
                reference_number, adk_thread_id, user_id, platform
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

    /// Get report by reference number
    #[allow(dead_code)]
    pub async fn get_by_reference(&self, reference_number: &str) -> Result<Option<Report>> {
        sqlx::query_as!(
            Report,
            r#"
            SELECT
                id, ticket_id, cluster_id, title, description,
                timeline, impact,
                status as "status: ReportStatus",
                verified_at, verified_by, resolved_at, resolved_by, resolution_notes,
                created_at, updated_at,
                reference_number, adk_thread_id, user_id, platform
            FROM reports
            WHERE reference_number = $1
            "#,
            reference_number
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get report by reference: {:?}", e);
            AppError::Database(e)
        })
    }

    /// Get report by ticket ID
    #[allow(dead_code)]
    pub async fn get_by_ticket_id(&self, ticket_id: Uuid) -> Result<Option<Report>> {
        sqlx::query_as!(
            Report,
            r#"
            SELECT
                id, ticket_id, cluster_id, title, description,
                timeline, impact,
                status as "status: ReportStatus",
                verified_at, verified_by, resolved_at, resolved_by, resolution_notes,
                created_at, updated_at,
                reference_number, adk_thread_id, user_id, platform
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
                geocoding_score, geocoded_at, created_at,
                province_id, regency_id, district_id, village_id
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

    /// List reports by user (new workflow - query reports.user_id directly)
    pub async fn list_by_user(&self, user_id: &str) -> Result<Vec<Report>> {
        sqlx::query_as!(
            Report,
            r#"
            SELECT
                id, ticket_id, cluster_id, title, description,
                timeline, impact,
                status as "status: ReportStatus",
                verified_at, verified_by, resolved_at, resolved_by, resolution_notes,
                created_at, updated_at,
                reference_number, adk_thread_id, user_id, platform
            FROM reports
            WHERE user_id = $1
            ORDER BY created_at DESC
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
                id, ticket_id, cluster_id, title, description,
                timeline, impact,
                status as "status: ReportStatus",
                verified_at, verified_by, resolved_at, resolved_by, resolution_notes,
                created_at, updated_at,
                reference_number, adk_thread_id, user_id, platform
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

    /// Mark report as rejected (low confidence or invalid)
    pub async fn reject(&self, report_id: Uuid, reason: Option<&str>) -> Result<Report> {
        let report = sqlx::query_as!(
            Report,
            r#"
            UPDATE reports
            SET status = 'rejected', resolution_notes = $2, updated_at = NOW()
            WHERE id = $1
            RETURNING
                id, ticket_id, cluster_id, title, description,
                timeline, impact,
                status as "status: ReportStatus",
                verified_at, verified_by, resolved_at, resolved_by, resolution_notes,
                created_at, updated_at,
                reference_number, adk_thread_id, user_id, platform
            "#,
            report_id,
            reason
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to reject report: {:?}", e);
            AppError::Database(e)
        })?
        .ok_or_else(|| AppError::NotFound(format!("Report {} not found", report_id)))?;

        tracing::info!("Report {} rejected: {:?}", report_id, reason);
        Ok(report)
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
                id, ticket_id, cluster_id, title, description,
                timeline, impact,
                status as "status: ReportStatus",
                verified_at, verified_by, resolved_at, resolved_by, resolution_notes,
                created_at, updated_at,
                reference_number, adk_thread_id, user_id, platform
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

    // ===== Category Management =====

    /// Assign a category to a report with severity
    pub async fn assign_category(&self, data: &CreateReportCategory) -> Result<ReportCategory> {
        let category = sqlx::query_as!(
            ReportCategory,
            r#"
            INSERT INTO report_categories (report_id, category_id, severity)
            VALUES ($1, $2, $3)
            ON CONFLICT (report_id, category_id)
            DO UPDATE SET severity = $3
            RETURNING
                id, report_id, category_id,
                severity as "severity: ReportSeverity",
                created_at
            "#,
            data.report_id,
            data.category_id,
            data.severity as ReportSeverity
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to assign category: {:?}", e);
            AppError::Database(e)
        })?;

        tracing::debug!(
            "Assigned category {} to report {} with severity {:?}",
            data.category_id,
            data.report_id,
            data.severity
        );

        Ok(category)
    }

    /// Assign multiple categories to a report
    pub async fn assign_categories(
        &self,
        report_id: Uuid,
        categories: &[CreateReportCategory],
    ) -> Result<Vec<ReportCategory>> {
        let mut results = Vec::with_capacity(categories.len());

        for cat in categories {
            // Ensure report_id matches
            let data = CreateReportCategory {
                report_id,
                category_id: cat.category_id,
                severity: cat.severity,
            };
            let result = self.assign_category(&data).await?;
            results.push(result);
        }

        Ok(results)
    }

    /// Get all categories for a report
    #[allow(dead_code)]
    pub async fn get_categories(&self, report_id: Uuid) -> Result<Vec<ReportCategory>> {
        sqlx::query_as!(
            ReportCategory,
            r#"
            SELECT
                id, report_id, category_id,
                severity as "severity: ReportSeverity",
                created_at
            FROM report_categories
            WHERE report_id = $1
            ORDER BY created_at ASC
            "#,
            report_id
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get report categories: {:?}", e);
            AppError::Database(e)
        })
    }

    // ===== Tag Management =====

    /// Add a tag to a report
    pub async fn add_tag(&self, data: &CreateReportTag) -> Result<ReportTag> {
        let tag = sqlx::query_as!(
            ReportTag,
            r#"
            INSERT INTO report_tags (report_id, tag_type)
            VALUES ($1, $2)
            ON CONFLICT (report_id, tag_type) DO NOTHING
            RETURNING
                id, report_id,
                tag_type as "tag_type: ReportTagType",
                created_at
            "#,
            data.report_id,
            data.tag_type as ReportTagType
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to add tag: {:?}", e);
            AppError::Database(e)
        })?;

        // If conflict occurred, fetch the existing tag
        match tag {
            Some(t) => Ok(t),
            None => sqlx::query_as!(
                ReportTag,
                r#"
                    SELECT
                        id, report_id,
                        tag_type as "tag_type: ReportTagType",
                        created_at
                    FROM report_tags
                    WHERE report_id = $1 AND tag_type = $2
                    "#,
                data.report_id,
                data.tag_type as ReportTagType
            )
            .fetch_one(&self.pool)
            .await
            .map_err(|e| {
                tracing::error!("Failed to fetch existing tag: {:?}", e);
                AppError::Database(e)
            }),
        }
    }

    /// Add multiple tags to a report
    pub async fn add_tags(
        &self,
        report_id: Uuid,
        tag_types: &[ReportTagType],
    ) -> Result<Vec<ReportTag>> {
        let mut results = Vec::with_capacity(tag_types.len());

        for tag_type in tag_types {
            let data = CreateReportTag {
                report_id,
                tag_type: *tag_type,
            };
            let result = self.add_tag(&data).await?;
            results.push(result);
        }

        Ok(results)
    }

    /// Get all tags for a report
    #[allow(dead_code)]
    pub async fn get_tags(&self, report_id: Uuid) -> Result<Vec<ReportTag>> {
        sqlx::query_as!(
            ReportTag,
            r#"
            SELECT
                id, report_id,
                tag_type as "tag_type: ReportTagType",
                created_at
            FROM report_tags
            WHERE report_id = $1
            ORDER BY created_at ASC
            "#,
            report_id
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get report tags: {:?}", e);
            AppError::Database(e)
        })
    }

    // ===== Location Region Management =====

    /// Update region FKs for a report location
    #[allow(dead_code)]
    pub async fn update_location_regions(
        &self,
        location_id: Uuid,
        province_id: Option<Uuid>,
        regency_id: Option<Uuid>,
        district_id: Option<Uuid>,
        village_id: Option<Uuid>,
    ) -> Result<()> {
        sqlx::query!(
            r#"
            UPDATE report_locations
            SET
                province_id = COALESCE($2, province_id),
                regency_id = COALESCE($3, regency_id),
                district_id = COALESCE($4, district_id),
                village_id = COALESCE($5, village_id)
            WHERE id = $1
            "#,
            location_id,
            province_id,
            regency_id,
            district_id,
            village_id
        )
        .execute(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to update location regions: {:?}", e);
            AppError::Database(e)
        })?;

        tracing::debug!(
            "Updated location {} regions: province={:?}, regency={:?}, district={:?}, village={:?}",
            location_id,
            province_id,
            regency_id,
            district_id,
            village_id
        );

        Ok(())
    }

    // ===== Attachment Management =====

    /// Link a single attachment to a report
    pub async fn link_attachment(&self, data: &CreateReportAttachment) -> Result<ReportAttachment> {
        let attachment = sqlx::query_as!(
            ReportAttachment,
            r#"
            INSERT INTO report_attachments (report_id, file_id)
            VALUES ($1, $2)
            ON CONFLICT (report_id, file_id) DO NOTHING
            RETURNING id, report_id, file_id, created_at
            "#,
            data.report_id,
            data.file_id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to link attachment: {:?}", e);
            AppError::Database(e)
        })?;

        // If conflict occurred, fetch the existing attachment link
        match attachment {
            Some(a) => Ok(a),
            None => sqlx::query_as!(
                ReportAttachment,
                r#"
                SELECT id, report_id, file_id, created_at
                FROM report_attachments
                WHERE report_id = $1 AND file_id = $2
                "#,
                data.report_id,
                data.file_id
            )
            .fetch_one(&self.pool)
            .await
            .map_err(|e| {
                tracing::error!("Failed to fetch existing attachment link: {:?}", e);
                AppError::Database(e)
            }),
        }
    }

    /// Link multiple attachments to a report
    #[allow(dead_code)]
    pub async fn link_attachments(
        &self,
        report_id: Uuid,
        file_ids: &[Uuid],
    ) -> Result<Vec<ReportAttachment>> {
        let mut results = Vec::with_capacity(file_ids.len());

        for file_id in file_ids {
            let data = CreateReportAttachment {
                report_id,
                file_id: *file_id,
            };
            let result = self.link_attachment(&data).await?;
            results.push(result);
        }

        Ok(results)
    }

    /// Get all attachments for a report
    #[allow(dead_code)]
    pub async fn get_attachments(&self, report_id: Uuid) -> Result<Vec<ReportAttachment>> {
        sqlx::query_as!(
            ReportAttachment,
            r#"
            SELECT id, report_id, file_id, created_at
            FROM report_attachments
            WHERE report_id = $1
            ORDER BY created_at ASC
            "#,
            report_id
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get report attachments: {:?}", e);
            AppError::Database(e)
        })
    }

    /// Copy attachments from thread_attachments to report_attachments
    /// Used by ReportProcessor to link thread files to the processed report
    pub async fn copy_attachments_from_thread(
        &self,
        report_id: Uuid,
        thread_id: Uuid,
    ) -> Result<i64> {
        // Insert attachments from thread_attachments that don't already exist
        let result = sqlx::query!(
            r#"
            INSERT INTO report_attachments (report_id, file_id)
            SELECT $1, ta.file_id
            FROM thread_attachments ta
            WHERE ta.thread_id = $2
            ON CONFLICT (report_id, file_id) DO NOTHING
            "#,
            report_id,
            thread_id
        )
        .execute(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to copy attachments from thread: {:?}", e);
            AppError::Database(e)
        })?;

        let count = result.rows_affected() as i64;

        if count > 0 {
            tracing::info!(
                "Copied {} attachments from thread {} to report {}",
                count,
                thread_id,
                report_id
            );
        }

        Ok(count)
    }
}
