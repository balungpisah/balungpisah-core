use sqlx::PgPool;
use uuid::Uuid;

use crate::core::error::{AppError, Result};
use crate::features::admin::dtos::*;
use crate::features::reports::models::{ReportSeverity, ReportStatus};
use crate::features::tickets::models::TicketStatus;

/// Service for admin queries
pub struct AdminService {
    pool: PgPool,
}

impl AdminService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    // =========================================================================
    // EXPECTATIONS
    // =========================================================================

    /// List expectations with pagination
    pub async fn list_expectations(
        &self,
        offset: i64,
        limit: i64,
    ) -> Result<(Vec<AdminExpectationDto>, i64)> {
        // Get total count
        let total = sqlx::query_scalar!(r#"SELECT COUNT(*) as "count!" FROM expectations"#)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| {
                tracing::error!("Failed to count expectations: {:?}", e);
                AppError::Database(e)
            })?;

        // Get paginated data
        let rows = sqlx::query!(
            r#"
            SELECT id, name, email, expectation, created_at
            FROM expectations
            ORDER BY created_at DESC
            OFFSET $1 LIMIT $2
            "#,
            offset,
            limit
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to list expectations: {:?}", e);
            AppError::Database(e)
        })?;

        let items = rows
            .into_iter()
            .map(|r| AdminExpectationDto {
                id: r.id,
                name: r.name,
                email: r.email,
                expectation: r.expectation,
                created_at: r.created_at,
            })
            .collect();

        Ok((items, total))
    }

    // =========================================================================
    // REPORTS
    // =========================================================================

    /// List reports with pagination (includes all statuses for admin)
    pub async fn list_reports(
        &self,
        offset: i64,
        limit: i64,
    ) -> Result<(Vec<AdminReportDto>, i64)> {
        // Get total count
        let total = sqlx::query_scalar!(r#"SELECT COUNT(*) as "count!" FROM reports"#)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| {
                tracing::error!("Failed to count reports: {:?}", e);
                AppError::Database(e)
            })?;

        // Get paginated reports
        let rows = sqlx::query!(
            r#"
            SELECT
                id, reference_number, title, description,
                status as "status: ReportStatus",
                user_id, platform,
                created_at, updated_at
            FROM reports
            ORDER BY created_at DESC
            OFFSET $1 LIMIT $2
            "#,
            offset,
            limit
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to list reports: {:?}", e);
            AppError::Database(e)
        })?;

        let mut items = Vec::with_capacity(rows.len());
        for row in rows {
            let categories = self.get_report_categories(row.id).await?;
            let location = self.get_report_location(row.id).await?;
            let attachments = self.get_report_attachments(row.id).await?;

            items.push(AdminReportDto {
                id: row.id,
                reference_number: row.reference_number,
                title: row.title,
                description: row.description,
                status: row.status,
                user_id: row.user_id,
                platform: row.platform,
                created_at: row.created_at,
                updated_at: row.updated_at,
                categories,
                location,
                attachments,
            });
        }

        Ok((items, total))
    }

    /// Get categories for a report
    async fn get_report_categories(&self, report_id: Uuid) -> Result<Vec<AdminReportCategoryDto>> {
        let rows = sqlx::query!(
            r#"
            SELECT
                rc.category_id,
                c.name as category_name,
                c.slug as category_slug,
                rc.severity as "severity: ReportSeverity"
            FROM report_categories rc
            JOIN categories c ON c.id = rc.category_id
            WHERE rc.report_id = $1
            ORDER BY c.name
            "#,
            report_id
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get report categories: {:?}", e);
            AppError::Database(e)
        })?;

        Ok(rows
            .into_iter()
            .map(|r| AdminReportCategoryDto {
                category_id: r.category_id,
                category_name: r.category_name,
                category_slug: r.category_slug,
                severity: r.severity,
            })
            .collect())
    }

    /// Get location for a report
    async fn get_report_location(&self, report_id: Uuid) -> Result<Option<AdminReportLocationDto>> {
        let row = sqlx::query!(
            r#"
            SELECT
                rl.raw_input,
                rl.display_name,
                rl.lat,
                rl.lon,
                rl.city,
                rl.state,
                p.name as "province_name?",
                rg.name as "regency_name?"
            FROM report_locations rl
            LEFT JOIN provinces p ON p.id = rl.province_id
            LEFT JOIN regencies rg ON rg.id = rl.regency_id
            WHERE rl.report_id = $1
            "#,
            report_id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get report location: {:?}", e);
            AppError::Database(e)
        })?;

        Ok(row.map(|r| AdminReportLocationDto {
            raw_input: r.raw_input,
            display_name: r.display_name,
            lat: r.lat,
            lon: r.lon,
            city: r.city,
            state: r.state,
            province_name: r.province_name,
            regency_name: r.regency_name,
        }))
    }

    /// Get attachments for a report
    async fn get_report_attachments(
        &self,
        report_id: Uuid,
    ) -> Result<Vec<AdminReportAttachmentDto>> {
        let rows = sqlx::query!(
            r#"
            SELECT
                f.id as file_id,
                f.original_filename,
                f.content_type,
                f.file_size,
                f.url
            FROM report_attachments ra
            JOIN files f ON f.id = ra.file_id
            WHERE ra.report_id = $1
            ORDER BY ra.created_at
            "#,
            report_id
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get report attachments: {:?}", e);
            AppError::Database(e)
        })?;

        Ok(rows
            .into_iter()
            .map(|r| AdminReportAttachmentDto {
                file_id: r.file_id,
                original_filename: r.original_filename,
                content_type: r.content_type,
                file_size: r.file_size,
                url: r.url,
            })
            .collect())
    }

    // =========================================================================
    // CONTRIBUTORS
    // =========================================================================

    /// List contributors with pagination
    pub async fn list_contributors(
        &self,
        offset: i64,
        limit: i64,
    ) -> Result<(Vec<AdminContributorDto>, i64)> {
        // Get total count
        let total = sqlx::query_scalar!(r#"SELECT COUNT(*) as "count!" FROM contributors"#)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| {
                tracing::error!("Failed to count contributors: {:?}", e);
                AppError::Database(e)
            })?;

        // Get paginated data
        let rows = sqlx::query!(
            r#"
            SELECT
                id, submission_type,
                name, email, whatsapp, city, role, skills, bio, portfolio_url, aspiration,
                organization_name, organization_type, contact_name, contact_position,
                contact_whatsapp, contact_email, contribution_offer,
                agreed, created_at
            FROM contributors
            ORDER BY created_at DESC
            OFFSET $1 LIMIT $2
            "#,
            offset,
            limit
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to list contributors: {:?}", e);
            AppError::Database(e)
        })?;

        let items = rows
            .into_iter()
            .map(|r| AdminContributorDto {
                id: r.id,
                submission_type: r.submission_type,
                name: r.name,
                email: r.email,
                whatsapp: r.whatsapp,
                city: r.city,
                role: r.role,
                skills: r.skills,
                bio: r.bio,
                portfolio_url: r.portfolio_url,
                aspiration: r.aspiration,
                organization_name: r.organization_name,
                organization_type: r.organization_type,
                contact_name: r.contact_name,
                contact_position: r.contact_position,
                contact_whatsapp: r.contact_whatsapp,
                contact_email: r.contact_email,
                contribution_offer: r.contribution_offer,
                agreed: r.agreed,
                created_at: r.created_at,
            })
            .collect();

        Ok((items, total))
    }

    // =========================================================================
    // TICKETS
    // =========================================================================

    /// List tickets with pagination
    pub async fn list_tickets(
        &self,
        offset: i64,
        limit: i64,
    ) -> Result<(Vec<AdminTicketDto>, i64)> {
        // Get total count
        let total = sqlx::query_scalar!(r#"SELECT COUNT(*) as "count!" FROM tickets"#)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| {
                tracing::error!("Failed to count tickets: {:?}", e);
                AppError::Database(e)
            })?;

        // Get paginated data
        let rows = sqlx::query!(
            r#"
            SELECT
                id, reference_number, user_id, platform,
                status as "status: TicketStatus",
                confidence_score, completeness_score,
                retry_count, error_message, report_id,
                submitted_at, processed_at, created_at
            FROM tickets
            ORDER BY created_at DESC
            OFFSET $1 LIMIT $2
            "#,
            offset,
            limit
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to list tickets: {:?}", e);
            AppError::Database(e)
        })?;

        let items = rows
            .into_iter()
            .map(|r| AdminTicketDto {
                id: r.id,
                reference_number: r.reference_number,
                user_id: r.user_id,
                platform: r.platform,
                status: r.status,
                confidence_score: r.confidence_score.to_string().parse::<f64>().unwrap_or(0.0),
                completeness_score: r
                    .completeness_score
                    .map(|d| d.to_string().parse::<f64>().unwrap_or(0.0)),
                retry_count: r.retry_count,
                error_message: r.error_message,
                report_id: r.report_id,
                submitted_at: r.submitted_at,
                processed_at: r.processed_at,
                created_at: r.created_at,
            })
            .collect();

        Ok((items, total))
    }
}
