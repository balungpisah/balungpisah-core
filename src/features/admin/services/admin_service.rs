use sqlx::PgPool;
use uuid::Uuid;

use crate::core::error::{AppError, Result};
use crate::features::admin::dtos::*;
use crate::features::reports::models::{ReportSeverity, ReportStatus, ReportTagType};

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

    /// List expectations with pagination and filters
    pub async fn list_expectations(
        &self,
        params: &ExpectationQueryParams,
    ) -> Result<(Vec<AdminExpectationDto>, i64)> {
        let offset = params.offset();
        let limit = params.limit();
        let sort_dir = params.sort.as_sql();

        // Build WHERE clause dynamically
        let mut conditions = Vec::new();
        let mut args: Vec<String> = Vec::new();

        if let Some(has_email) = params.has_email {
            if has_email {
                conditions.push("email IS NOT NULL".to_string());
            } else {
                conditions.push("email IS NULL".to_string());
            }
        }

        if let Some(from_date) = params.from_date {
            args.push(from_date.to_string());
            conditions.push(format!("created_at >= ${}::date", args.len()));
        }

        if let Some(to_date) = params.to_date {
            args.push(to_date.to_string());
            conditions.push(format!(
                "created_at < (${}::date + interval '1 day')",
                args.len()
            ));
        }

        if let Some(ref search) = params.search {
            args.push(format!("%{}%", search.to_lowercase()));
            conditions.push(format!(
                "(LOWER(name) LIKE ${0} OR LOWER(expectation) LIKE ${0})",
                args.len()
            ));
        }

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", conditions.join(" AND "))
        };

        // Get total count
        let count_query = format!(r#"SELECT COUNT(*) FROM expectations {}"#, where_clause);
        let total: i64 = self.execute_count_query(&count_query, &args).await?;

        // Get paginated data with dynamic ORDER BY
        let data_query = format!(
            r#"
            SELECT id, name, email, expectation, created_at
            FROM expectations
            {}
            ORDER BY created_at {}
            OFFSET {} LIMIT {}
            "#,
            where_clause, sort_dir, offset, limit
        );

        let items = self.execute_expectations_query(&data_query, &args).await?;

        Ok((items, total))
    }

    async fn execute_count_query(&self, query: &str, args: &[String]) -> Result<i64> {
        let mut sqlx_query = sqlx::query_scalar::<_, i64>(query);
        for arg in args {
            sqlx_query = sqlx_query.bind(arg);
        }
        sqlx_query.fetch_one(&self.pool).await.map_err(|e| {
            tracing::error!("Failed to execute count query: {:?}", e);
            AppError::Database(e)
        })
    }

    async fn execute_expectations_query(
        &self,
        query: &str,
        args: &[String],
    ) -> Result<Vec<AdminExpectationDto>> {
        let mut sqlx_query = sqlx::query_as::<_, ExpectationRow>(query);
        for arg in args {
            sqlx_query = sqlx_query.bind(arg);
        }
        let rows = sqlx_query.fetch_all(&self.pool).await.map_err(|e| {
            tracing::error!("Failed to execute expectations query: {:?}", e);
            AppError::Database(e)
        })?;

        Ok(rows
            .into_iter()
            .map(|r| AdminExpectationDto {
                id: r.id,
                name: r.name,
                email: r.email,
                expectation: r.expectation,
                created_at: r.created_at,
            })
            .collect())
    }

    /// Get a single expectation by ID
    pub async fn get_expectation(&self, id: Uuid) -> Result<AdminExpectationDto> {
        let row = sqlx::query!(
            r#"
            SELECT id, name, email, expectation, created_at
            FROM expectations
            WHERE id = $1
            "#,
            id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get expectation: {:?}", e);
            AppError::Database(e)
        })?
        .ok_or_else(|| AppError::NotFound("Expectation not found".to_string()))?;

        Ok(AdminExpectationDto {
            id: row.id,
            name: row.name,
            email: row.email,
            expectation: row.expectation,
            created_at: row.created_at,
        })
    }

    // =========================================================================
    // REPORTS
    // =========================================================================

    /// List reports with pagination and filters (optimized single query)
    pub async fn list_reports(
        &self,
        params: &ReportQueryParams,
    ) -> Result<(Vec<AdminReportDto>, i64)> {
        let offset = params.offset();
        let limit = params.limit();
        let sort_by = params.sort_by.as_sql();
        let sort_dir = params.sort.as_sql();

        // Build WHERE clause dynamically
        let mut conditions = Vec::new();
        let mut args: Vec<String> = Vec::new();

        if let Some(ref status) = params.status {
            args.push(status.to_string());
            conditions.push(format!("r.status = ${}::report_status", args.len()));
        }

        if let Some(from_date) = params.from_date {
            args.push(from_date.to_string());
            conditions.push(format!("r.created_at >= ${}::date", args.len()));
        }

        if let Some(to_date) = params.to_date {
            args.push(to_date.to_string());
            conditions.push(format!(
                "r.created_at < (${}::date + interval '1 day')",
                args.len()
            ));
        }

        if let Some(ref search) = params.search {
            args.push(format!("%{}%", search.to_lowercase()));
            conditions.push(format!(
                "(LOWER(r.reference_number) LIKE ${0} OR LOWER(r.title) LIKE ${0})",
                args.len()
            ));
        }

        if let Some(ref user_id) = params.user_id {
            args.push(user_id.clone());
            conditions.push(format!("r.user_id = ${}", args.len()));
        }

        if let Some(ref platform) = params.platform {
            args.push(platform.clone());
            conditions.push(format!("r.platform = ${}", args.len()));
        }

        if let Some(has_attachments) = params.has_attachments {
            if has_attachments {
                conditions.push(
                    "EXISTS (SELECT 1 FROM report_attachments WHERE report_id = r.id)".to_string(),
                );
            } else {
                conditions.push(
                    "NOT EXISTS (SELECT 1 FROM report_attachments WHERE report_id = r.id)"
                        .to_string(),
                );
            }
        }

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", conditions.join(" AND "))
        };

        // Get total count
        let count_query = format!(r#"SELECT COUNT(*) FROM reports r {}"#, where_clause);
        let total: i64 = self.execute_count_query(&count_query, &args).await?;

        // Optimized single query with LEFT JOINs and subqueries for aggregated data
        let data_query = format!(
            r#"
            SELECT
                r.id,
                r.reference_number,
                r.title,
                r.status,
                r.user_id,
                r.platform,
                r.created_at,
                r.updated_at,
                COALESCE(cat_agg.category_count, 0) as category_count,
                cat_agg.primary_category,
                COALESCE(rl.city, rg.name, rl.display_name) as location_summary,
                COALESCE(att_agg.attachment_count, 0) as attachment_count
            FROM reports r
            LEFT JOIN report_locations rl ON rl.report_id = r.id
            LEFT JOIN regencies rg ON rg.id = rl.regency_id
            LEFT JOIN LATERAL (
                SELECT
                    COUNT(*) as category_count,
                    (SELECT c.name FROM report_categories rc2
                     JOIN categories c ON c.id = rc2.category_id
                     WHERE rc2.report_id = r.id
                     ORDER BY c.name LIMIT 1) as primary_category
                FROM report_categories rc
                WHERE rc.report_id = r.id
            ) cat_agg ON true
            LEFT JOIN LATERAL (
                SELECT COUNT(*) as attachment_count
                FROM report_attachments ra
                WHERE ra.report_id = r.id
            ) att_agg ON true
            {}
            ORDER BY r.{} {}
            OFFSET {} LIMIT {}
            "#,
            where_clause, sort_by, sort_dir, offset, limit
        );

        let items = self.execute_reports_list_query(&data_query, &args).await?;

        Ok((items, total))
    }

    async fn execute_reports_list_query(
        &self,
        query: &str,
        args: &[String],
    ) -> Result<Vec<AdminReportDto>> {
        let mut sqlx_query = sqlx::query_as::<_, ReportListRow>(query);
        for arg in args {
            sqlx_query = sqlx_query.bind(arg);
        }
        let rows = sqlx_query.fetch_all(&self.pool).await.map_err(|e| {
            tracing::error!("Failed to execute reports list query: {:?}", e);
            AppError::Database(e)
        })?;

        Ok(rows
            .into_iter()
            .map(|r| AdminReportDto {
                id: r.id,
                reference_number: r.reference_number,
                title: r.title,
                status: r.status,
                user_id: r.user_id,
                platform: r.platform,
                created_at: r.created_at,
                updated_at: r.updated_at,
                category_count: r.category_count,
                primary_category: r.primary_category,
                location_summary: r.location_summary,
                attachment_count: r.attachment_count,
            })
            .collect())
    }

    /// Get a single report by ID with full details
    pub async fn get_report(&self, id: Uuid) -> Result<AdminReportDetailDto> {
        let row = sqlx::query!(
            r#"
            SELECT
                id, reference_number,
                title, description, timeline, impact,
                status as "status: ReportStatus",
                user_id, platform, adk_thread_id,
                verified_at, verified_by,
                resolved_at, resolved_by, resolution_notes,
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
        .ok_or_else(|| AppError::NotFound("Report not found".to_string()))?;

        let categories = self.get_report_categories(id).await?;
        let tags = self.get_report_tags(id).await?;
        let location = self.get_report_location(id).await?;
        let attachments = self.get_report_attachments(id).await?;

        Ok(AdminReportDetailDto {
            id: row.id,
            reference_number: row.reference_number,
            title: row.title,
            description: row.description,
            timeline: row.timeline,
            impact: row.impact,
            status: row.status,
            user_id: row.user_id,
            platform: row.platform,
            adk_thread_id: row.adk_thread_id,
            verified_at: row.verified_at,
            verified_by: row.verified_by,
            resolved_at: row.resolved_at,
            resolved_by: row.resolved_by,
            resolution_notes: row.resolution_notes,
            created_at: row.created_at,
            updated_at: row.updated_at,
            categories,
            tags,
            location,
            attachments,
        })
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

    /// Get tags for a report
    async fn get_report_tags(&self, report_id: Uuid) -> Result<Vec<ReportTagType>> {
        let rows = sqlx::query!(
            r#"
            SELECT tag_type as "tag_type: ReportTagType"
            FROM report_tags
            WHERE report_id = $1
            ORDER BY tag_type
            "#,
            report_id
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get report tags: {:?}", e);
            AppError::Database(e)
        })?;

        Ok(rows.into_iter().map(|r| r.tag_type).collect())
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

    /// List contributors with pagination and filters
    pub async fn list_contributors(
        &self,
        params: &ContributorQueryParams,
    ) -> Result<(Vec<AdminContributorDto>, i64)> {
        let offset = params.offset();
        let limit = params.limit();
        let sort_dir = params.sort.as_sql();

        // Build WHERE clause dynamically
        let mut conditions = Vec::new();
        let mut args: Vec<String> = Vec::new();

        if let Some(ref submission_type) = params.submission_type {
            args.push(submission_type.clone());
            conditions.push(format!("submission_type = ${}", args.len()));
        }

        if let Some(from_date) = params.from_date {
            args.push(from_date.to_string());
            conditions.push(format!("created_at >= ${}::date", args.len()));
        }

        if let Some(to_date) = params.to_date {
            args.push(to_date.to_string());
            conditions.push(format!(
                "created_at < (${}::date + interval '1 day')",
                args.len()
            ));
        }

        if let Some(ref search) = params.search {
            args.push(format!("%{}%", search.to_lowercase()));
            conditions.push(format!(
                "(LOWER(name) LIKE ${0} OR LOWER(email) LIKE ${0} OR LOWER(organization_name) LIKE ${0})",
                args.len()
            ));
        }

        if let Some(ref city) = params.city {
            args.push(city.clone());
            conditions.push(format!("city = ${}", args.len()));
        }

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", conditions.join(" AND "))
        };

        // Get total count
        let count_query = format!(r#"SELECT COUNT(*) FROM contributors {}"#, where_clause);
        let total: i64 = self.execute_count_query(&count_query, &args).await?;

        // Get paginated data with dynamic ORDER BY
        let data_query = format!(
            r#"
            SELECT
                id, submission_type, name, email, city, organization_name, created_at
            FROM contributors
            {}
            ORDER BY created_at {}
            OFFSET {} LIMIT {}
            "#,
            where_clause, sort_dir, offset, limit
        );

        let items = self.execute_contributors_query(&data_query, &args).await?;

        Ok((items, total))
    }

    async fn execute_contributors_query(
        &self,
        query: &str,
        args: &[String],
    ) -> Result<Vec<AdminContributorDto>> {
        let mut sqlx_query = sqlx::query_as::<_, ContributorRow>(query);
        for arg in args {
            sqlx_query = sqlx_query.bind(arg);
        }
        let rows = sqlx_query.fetch_all(&self.pool).await.map_err(|e| {
            tracing::error!("Failed to execute contributors query: {:?}", e);
            AppError::Database(e)
        })?;

        Ok(rows
            .into_iter()
            .map(|r| AdminContributorDto {
                id: r.id,
                submission_type: r.submission_type,
                name: r.name,
                email: r.email,
                city: r.city,
                organization_name: r.organization_name,
                created_at: r.created_at,
            })
            .collect())
    }

    /// Get a single contributor by ID with full details
    pub async fn get_contributor(&self, id: Uuid) -> Result<AdminContributorDetailDto> {
        let row = sqlx::query!(
            r#"
            SELECT
                id, submission_type,
                name, email, whatsapp, city, role, skills, bio, portfolio_url, aspiration,
                organization_name, organization_type, contact_name, contact_position,
                contact_whatsapp, contact_email, contribution_offer,
                agreed, created_at, updated_at
            FROM contributors
            WHERE id = $1
            "#,
            id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get contributor: {:?}", e);
            AppError::Database(e)
        })?
        .ok_or_else(|| AppError::NotFound("Contributor not found".to_string()))?;

        Ok(AdminContributorDetailDto {
            id: row.id,
            submission_type: row.submission_type,
            name: row.name,
            email: row.email,
            whatsapp: row.whatsapp,
            city: row.city,
            role: row.role,
            skills: row.skills,
            bio: row.bio,
            portfolio_url: row.portfolio_url,
            aspiration: row.aspiration,
            organization_name: row.organization_name,
            organization_type: row.organization_type,
            contact_name: row.contact_name,
            contact_position: row.contact_position,
            contact_whatsapp: row.contact_whatsapp,
            contact_email: row.contact_email,
            contribution_offer: row.contribution_offer,
            agreed: row.agreed,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

// =========================================================================
// ROW TYPES FOR DYNAMIC QUERIES
// =========================================================================

#[derive(sqlx::FromRow)]
struct ExpectationRow {
    id: Uuid,
    name: Option<String>,
    email: Option<String>,
    expectation: String,
    created_at: chrono::DateTime<chrono::Utc>,
}

/// Optimized row for reports list with aggregated data
#[derive(sqlx::FromRow)]
struct ReportListRow {
    id: Uuid,
    reference_number: Option<String>,
    title: Option<String>,
    status: ReportStatus,
    user_id: Option<String>,
    platform: Option<String>,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
    category_count: i64,
    primary_category: Option<String>,
    location_summary: Option<String>,
    attachment_count: i64,
}

#[derive(sqlx::FromRow)]
struct ContributorRow {
    id: Uuid,
    submission_type: String,
    name: Option<String>,
    email: Option<String>,
    city: Option<String>,
    organization_name: Option<String>,
    created_at: chrono::DateTime<chrono::Utc>,
}
