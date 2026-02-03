use sqlx::PgPool;
use uuid::Uuid;

use crate::core::error::{AppError, Result};
use crate::features::dashboard::dtos::*;
use crate::features::reports::models::{ReportSeverity, ReportStatus, ReportTagType};

/// Service for public dashboard queries
pub struct DashboardService {
    pool: PgPool,
}

impl DashboardService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    // ========================================================================
    // Summary (lightweight stats for header)
    // ========================================================================

    /// Get lightweight summary for dashboard header
    pub async fn get_summary(&self) -> Result<DashboardSummaryDto> {
        let counts = sqlx::query!(
            r#"
            SELECT
                COUNT(*) as "total_reports!",
                COUNT(*) FILTER (WHERE status = 'pending') as "pending_count!",
                COUNT(*) FILTER (WHERE status = 'resolved') as "resolved_count!",
                COUNT(*) FILTER (WHERE created_at >= date_trunc('week', CURRENT_DATE)) as "reports_this_week!",
                COUNT(*) FILTER (WHERE created_at >= date_trunc('month', CURRENT_DATE)) as "reports_this_month!"
            FROM reports
            WHERE status NOT IN ('pending', 'rejected')
            "#
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get summary counts: {:?}", e);
            AppError::Database(e)
        })?;

        Ok(DashboardSummaryDto {
            total_reports: counts.total_reports,
            pending_count: counts.pending_count,
            resolved_count: counts.resolved_count,
            reports_this_week: counts.reports_this_week,
            reports_this_month: counts.reports_this_month,
        })
    }

    // ========================================================================
    // List Reports (paginated)
    // ========================================================================

    /// List all reports with pagination
    /// Returns (reports, total_count)
    pub async fn list_reports(
        &self,
        params: &PaginationParams,
    ) -> Result<(Vec<DashboardReportDto>, i64)> {
        let offset = params.offset();
        let limit = params.limit();

        // Get total count
        let total = sqlx::query_scalar!(
            r#"SELECT COUNT(*) as "count!" FROM reports WHERE status NOT IN ('pending', 'rejected')"#
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to count reports: {:?}", e);
            AppError::Database(e)
        })?;

        // Get reports
        let rows = sqlx::query!(
            r#"
            SELECT
                r.id,
                r.title,
                r.description,
                r.status as "status: ReportStatus",
                r.timeline,
                r.impact,
                r.created_at
            FROM reports r
            WHERE r.status NOT IN ('pending', 'rejected')
            ORDER BY r.created_at DESC
            OFFSET $1 LIMIT $2
            "#,
            offset,
            limit
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch reports: {:?}", e);
            AppError::Database(e)
        })?;

        let mut reports = Vec::with_capacity(rows.len());
        for row in rows {
            let categories = self.get_report_categories(row.id).await?;
            let location = self.get_report_location(row.id).await?;
            let tag_type = self.get_report_tag(row.id).await?;

            reports.push(DashboardReportDto {
                id: row.id,
                title: row.title,
                description: row.description,
                status: row.status,
                tag_type,
                timeline: row.timeline,
                impact: row.impact,
                created_at: row.created_at,
                categories,
                location,
            });
        }

        Ok((reports, total))
    }

    /// Get single report detail
    pub async fn get_report(&self, id: Uuid) -> Result<DashboardReportDetailDto> {
        let row = sqlx::query!(
            r#"
            SELECT
                r.id,
                r.ticket_id,
                r.reference_number,
                r.title,
                r.description,
                r.status as "status: ReportStatus",
                r.timeline,
                r.impact,
                r.verified_at,
                r.resolved_at,
                r.resolution_notes,
                r.created_at,
                r.updated_at
            FROM reports r
            WHERE r.id = $1
            "#,
            id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch report: {:?}", e);
            AppError::Database(e)
        })?
        .ok_or_else(|| AppError::NotFound("Report not found".to_string()))?;

        let categories = self.get_report_categories(row.id).await?;
        let location = self.get_report_location(row.id).await?;
        let tag_type = self.get_report_tag(row.id).await?;

        Ok(DashboardReportDetailDto {
            id: row.id,
            ticket_id: row.ticket_id,
            reference_number: row.reference_number,
            title: row.title,
            description: row.description,
            status: row.status,
            tag_type,
            timeline: row.timeline,
            impact: row.impact,
            verified_at: row.verified_at,
            resolved_at: row.resolved_at,
            resolution_notes: row.resolution_notes,
            created_at: row.created_at,
            updated_at: row.updated_at,
            categories,
            location,
        })
    }

    // ========================================================================
    // By Location
    // ========================================================================

    /// Get location overview (provinces -> regencies -> reports)
    pub async fn get_by_location(
        &self,
        params: &LocationQueryParams,
    ) -> Result<DashboardLocationOverviewDto> {
        // Always include province summary
        let provinces = self.get_province_summary().await?;

        // If province_id provided, get regencies
        let regencies = if let Some(province_id) = params.province_id {
            Some(self.get_regency_summary(province_id).await?)
        } else {
            None
        };

        // If regency_id provided, get actual reports
        let (reports, pagination) = if let Some(regency_id) = params.regency_id {
            let offset = params.offset();
            let limit = params.limit();

            let total = sqlx::query_scalar!(
                r#"
                SELECT COUNT(*) as "count!"
                FROM reports r
                JOIN report_locations rl ON rl.report_id = r.id
                WHERE rl.regency_id = $1
                  AND r.status NOT IN ('pending', 'rejected')
                "#,
                regency_id
            )
            .fetch_one(&self.pool)
            .await
            .map_err(|e| {
                tracing::error!("Failed to count regency reports: {:?}", e);
                AppError::Database(e)
            })?;

            let reports = self
                .get_reports_by_regency(regency_id, offset, limit)
                .await?;
            let pagination = PaginationMeta::new(params.page, params.page_size, total);
            (Some(reports), Some(pagination))
        } else {
            (None, None)
        };

        Ok(DashboardLocationOverviewDto {
            provinces,
            regencies,
            reports,
            pagination,
        })
    }

    async fn get_province_summary(&self) -> Result<Vec<ProvinceReportSummary>> {
        let rows = sqlx::query!(
            r#"
            SELECT
                p.id,
                p.name,
                p.code,
                p.lat,
                p.lng,
                COUNT(rl.id) as "report_count!"
            FROM provinces p
            LEFT JOIN report_locations rl ON rl.province_id = p.id
            LEFT JOIN reports r ON r.id = rl.report_id AND r.status NOT IN ('pending', 'rejected')
            GROUP BY p.id, p.name, p.code, p.lat, p.lng
            HAVING COUNT(rl.id) > 0
            ORDER BY COUNT(rl.id) DESC, p.name ASC
            "#
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get province summary: {:?}", e);
            AppError::Database(e)
        })?;

        Ok(rows
            .into_iter()
            .map(|r| ProvinceReportSummary {
                id: r.id,
                name: r.name,
                code: r.code,
                lat: r.lat,
                lng: r.lng,
                report_count: r.report_count,
            })
            .collect())
    }

    async fn get_regency_summary(&self, province_id: Uuid) -> Result<Vec<RegencyReportSummary>> {
        let rows = sqlx::query!(
            r#"
            SELECT
                rg.id,
                rg.province_id,
                rg.name,
                rg.code,
                rg.lat,
                rg.lng,
                COUNT(rl.id) as "report_count!"
            FROM regencies rg
            LEFT JOIN report_locations rl ON rl.regency_id = rg.id
            LEFT JOIN reports r ON r.id = rl.report_id AND r.status NOT IN ('pending', 'rejected')
            WHERE rg.province_id = $1
            GROUP BY rg.id, rg.province_id, rg.name, rg.code, rg.lat, rg.lng
            HAVING COUNT(rl.id) > 0
            ORDER BY COUNT(rl.id) DESC, rg.name ASC
            "#,
            province_id
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get regency summary: {:?}", e);
            AppError::Database(e)
        })?;

        Ok(rows
            .into_iter()
            .map(|r| RegencyReportSummary {
                id: r.id,
                province_id: r.province_id,
                name: r.name,
                code: r.code,
                lat: r.lat,
                lng: r.lng,
                report_count: r.report_count,
            })
            .collect())
    }

    async fn get_reports_by_regency(
        &self,
        regency_id: Uuid,
        offset: i64,
        limit: i64,
    ) -> Result<Vec<DashboardReportDto>> {
        let rows = sqlx::query!(
            r#"
            SELECT
                r.id,
                r.title,
                r.description,
                r.status as "status: ReportStatus",
                r.timeline,
                r.impact,
                r.created_at
            FROM reports r
            JOIN report_locations rl ON rl.report_id = r.id
            WHERE rl.regency_id = $1
              AND r.status NOT IN ('pending', 'rejected')
            ORDER BY r.created_at DESC
            OFFSET $2 LIMIT $3
            "#,
            regency_id,
            offset,
            limit
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get regency reports: {:?}", e);
            AppError::Database(e)
        })?;

        let mut reports = Vec::with_capacity(rows.len());
        for row in rows {
            let categories = self.get_report_categories(row.id).await?;
            let location = self.get_report_location(row.id).await?;
            let tag_type = self.get_report_tag(row.id).await?;

            reports.push(DashboardReportDto {
                id: row.id,
                title: row.title,
                description: row.description,
                status: row.status,
                tag_type,
                timeline: row.timeline,
                impact: row.impact,
                created_at: row.created_at,
                categories,
                location,
            });
        }

        Ok(reports)
    }

    // ========================================================================
    // By Category
    // ========================================================================

    /// Get category overview with optional report listing
    pub async fn get_by_category(
        &self,
        params: &CategoryQueryParams,
    ) -> Result<DashboardCategoryOverviewDto> {
        // Get category summary
        let categories = self.get_category_summary().await?;

        // If slug provided, get reports for that category
        let (reports, pagination) = if let Some(slug) = &params.slug {
            let offset = params.offset();
            let limit = params.limit();

            let total = sqlx::query_scalar!(
                r#"
                SELECT COUNT(DISTINCT r.id) as "count!"
                FROM reports r
                JOIN report_categories rc ON rc.report_id = r.id
                JOIN categories c ON c.id = rc.category_id
                WHERE c.slug = $1
                  AND r.status NOT IN ('pending', 'rejected')
                "#,
                slug
            )
            .fetch_one(&self.pool)
            .await
            .map_err(|e| {
                tracing::error!("Failed to count category reports: {:?}", e);
                AppError::Database(e)
            })?;

            let reports = self.get_reports_by_category(slug, offset, limit).await?;
            let pagination = PaginationMeta::new(params.page, params.page_size, total);
            (Some(reports), Some(pagination))
        } else {
            (None, None)
        };

        Ok(DashboardCategoryOverviewDto {
            categories,
            reports,
            pagination,
        })
    }

    async fn get_category_summary(&self) -> Result<Vec<CategoryReportSummary>> {
        let rows = sqlx::query!(
            r#"
            SELECT
                c.id,
                c.name,
                c.slug,
                c.description,
                c.color,
                c.icon,
                COUNT(DISTINCT rc.report_id) as "report_count!"
            FROM categories c
            LEFT JOIN report_categories rc ON rc.category_id = c.id
            LEFT JOIN reports r ON r.id = rc.report_id AND r.status NOT IN ('pending', 'rejected')
            WHERE c.is_active = true
            GROUP BY c.id, c.name, c.slug, c.description, c.color, c.icon, c.display_order
            ORDER BY COUNT(DISTINCT rc.report_id) DESC, c.display_order ASC
            "#
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get category summary: {:?}", e);
            AppError::Database(e)
        })?;

        Ok(rows
            .into_iter()
            .map(|r| CategoryReportSummary {
                id: r.id,
                name: r.name,
                slug: r.slug,
                description: r.description,
                color: r.color,
                icon: r.icon,
                report_count: r.report_count,
            })
            .collect())
    }

    async fn get_reports_by_category(
        &self,
        slug: &str,
        offset: i64,
        limit: i64,
    ) -> Result<Vec<DashboardReportDto>> {
        let rows = sqlx::query!(
            r#"
            SELECT DISTINCT
                r.id,
                r.title,
                r.description,
                r.status as "status: ReportStatus",
                r.timeline,
                r.impact,
                r.created_at
            FROM reports r
            JOIN report_categories rc ON rc.report_id = r.id
            JOIN categories c ON c.id = rc.category_id
            WHERE c.slug = $1
              AND r.status NOT IN ('pending', 'rejected')
            ORDER BY r.created_at DESC
            OFFSET $2 LIMIT $3
            "#,
            slug,
            offset,
            limit
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get category reports: {:?}", e);
            AppError::Database(e)
        })?;

        let mut reports = Vec::with_capacity(rows.len());
        for row in rows {
            let categories = self.get_report_categories(row.id).await?;
            let location = self.get_report_location(row.id).await?;
            let tag_type = self.get_report_tag(row.id).await?;

            reports.push(DashboardReportDto {
                id: row.id,
                title: row.title,
                description: row.description,
                status: row.status,
                tag_type,
                timeline: row.timeline,
                impact: row.impact,
                created_at: row.created_at,
                categories,
                location,
            });
        }

        Ok(reports)
    }

    // ========================================================================
    // By Tag
    // ========================================================================

    /// Get tag overview with optional report listing
    pub async fn get_by_tag(&self, params: &TagQueryParams) -> Result<DashboardTagOverviewDto> {
        // Get tag summary
        let tags = if let Some(tag_type) = &params.tag_type {
            self.get_tag_summary_filtered(Some(tag_type)).await?
        } else {
            self.get_tag_summary().await?
        };

        // If tag_type provided, get reports
        let (reports, pagination) = if let Some(tag_type) = &params.tag_type {
            let offset = params.offset();
            let limit = params.limit();

            let total = sqlx::query_scalar!(
                r#"
                SELECT COUNT(*) as "count!"
                FROM reports r
                JOIN report_tags rt ON rt.report_id = r.id
                WHERE rt.tag_type = $1
                  AND r.status NOT IN ('pending', 'rejected')
                "#,
                tag_type as &ReportTagType
            )
            .fetch_one(&self.pool)
            .await
            .map_err(|e| {
                tracing::error!("Failed to count tag reports: {:?}", e);
                AppError::Database(e)
            })?;

            let reports = self.get_reports_by_tag(tag_type, offset, limit).await?;
            let pagination = PaginationMeta::new(params.page, params.page_size, total);
            (Some(reports), Some(pagination))
        } else {
            (None, None)
        };

        Ok(DashboardTagOverviewDto {
            tags,
            reports,
            pagination,
        })
    }

    async fn get_tag_summary(&self) -> Result<Vec<TagReportSummary>> {
        let rows = sqlx::query!(
            r#"
            SELECT
                rt.tag_type as "tag_type: ReportTagType",
                COUNT(*) as "report_count!"
            FROM report_tags rt
            JOIN reports r ON r.id = rt.report_id
            WHERE r.status NOT IN ('pending', 'rejected')
            GROUP BY rt.tag_type
            ORDER BY COUNT(*) DESC
            "#
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get tag summary: {:?}", e);
            AppError::Database(e)
        })?;

        Ok(rows
            .into_iter()
            .map(|r| TagReportSummary {
                tag_type: r.tag_type,
                label: tag_label(&r.tag_type),
                report_count: r.report_count,
            })
            .collect())
    }

    async fn get_tag_summary_filtered(
        &self,
        filter_tag: Option<&ReportTagType>,
    ) -> Result<Vec<TagReportSummary>> {
        // Convert the enum to a string if it exists
        let filter_str = filter_tag.map(|t| format!("{:?}", t).to_lowercase());

        let rows = sqlx::query!(
            r#"
            SELECT 
                rt.tag_type as "tag_type: ReportTagType",
                COUNT(r.id) as "report_count!"
            FROM report_tags rt
            JOIN reports r ON r.id = rt.report_id
            WHERE ($1::TEXT IS NULL OR rt.tag_type::TEXT = $1::TEXT)
            AND r.status NOT IN ('pending', 'rejected')
            GROUP BY rt.tag_type
            "#,
            filter_str // This is $1
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch tag summary: {:?}", e);
            AppError::Database(e)
        })?;

        // Map rows to TagReportSummary
        Ok(rows
            .into_iter()
            .map(|row| TagReportSummary {
                tag_type: row.tag_type,
                label: format!("{:?}", row.tag_type),
                report_count: row.report_count,
            })
            .collect())
    }

    async fn get_reports_by_tag(
        &self,
        tag_type: &ReportTagType,
        offset: i64,
        limit: i64,
    ) -> Result<Vec<DashboardReportDto>> {
        let rows = sqlx::query!(
            r#"
            SELECT
                r.id,
                r.title,
                r.description,
                r.status as "status: ReportStatus",
                r.timeline,
                r.impact,
                r.created_at
            FROM reports r
            JOIN report_tags rt ON rt.report_id = r.id
            WHERE rt.tag_type::TEXT = $1::TEXT
              AND r.status NOT IN ('pending', 'rejected')
            ORDER BY r.created_at DESC
            OFFSET $2 LIMIT $3
            "#,
            tag_type as &ReportTagType,
            offset,
            limit
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get tag reports: {:?}", e);
            AppError::Database(e)
        })?;

        let mut reports = Vec::with_capacity(rows.len());
        for row in rows {
            let categories = self.get_report_categories(row.id).await?;
            let location = self.get_report_location(row.id).await?;
            let tag = self.get_report_tag(row.id).await?;

            reports.push(DashboardReportDto {
                id: row.id,
                title: row.title,
                description: row.description,
                status: row.status,
                tag_type: tag,
                timeline: row.timeline,
                impact: row.impact,
                created_at: row.created_at,
                categories,
                location,
            });
        }

        Ok(reports)
    }

    // ========================================================================
    // Recent Reports
    // ========================================================================

    /// Get recent reports (last N days)
    pub async fn get_recent(&self, params: &RecentQueryParams) -> Result<DashboardRecentDto> {
        let days = params.days.clamp(1, 365);
        let limit = params.limit.clamp(1, 100);

        let total = sqlx::query_scalar!(
            r#"
            SELECT COUNT(*) as "count!"
            FROM reports
            WHERE created_at >= CURRENT_DATE - $1::int
              AND status NOT IN ('pending', 'rejected')
            "#,
            days
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to count recent reports: {:?}", e);
            AppError::Database(e)
        })?;

        let rows = sqlx::query!(
            r#"
            SELECT
                r.id,
                r.title,
                r.description,
                r.status as "status: ReportStatus",
                r.timeline,
                r.impact,
                r.created_at
            FROM reports r
            WHERE r.created_at >= CURRENT_DATE - $1::int
              AND r.status NOT IN ('pending', 'rejected')
            ORDER BY r.created_at DESC
            LIMIT $2
            "#,
            days,
            limit
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch recent reports: {:?}", e);
            AppError::Database(e)
        })?;

        let mut reports = Vec::with_capacity(rows.len());
        for row in rows {
            let categories = self.get_report_categories(row.id).await?;
            let location = self.get_report_location(row.id).await?;
            let tag_type = self.get_report_tag(row.id).await?;

            reports.push(DashboardReportDto {
                id: row.id,
                title: row.title,
                description: row.description,
                status: row.status,
                tag_type,
                timeline: row.timeline,
                impact: row.impact,
                created_at: row.created_at,
                categories,
                location,
            });
        }

        Ok(DashboardRecentDto {
            reports,
            days,
            total_count: total,
        })
    }

    // ========================================================================
    // Map View
    // ========================================================================

    /// Get map markers for all reports with coordinates
    pub async fn get_map_data(&self, params: &MapQueryParams) -> Result<DashboardMapDto> {
        let limit = params.limit.clamp(1, 1000);

        let rows = sqlx::query!(
            r#"
            SELECT
                r.id,
                r.title,
                r.status as "status: ReportStatus",
                r.created_at,
                rl.lat as "lat!",
                rl.lon as "lon!",
                c.slug as "category_slug?",
                c.color as "category_color?"
            FROM reports r
            JOIN report_locations rl ON rl.report_id = r.id
            LEFT JOIN report_categories rc ON rc.report_id = r.id
            LEFT JOIN categories c ON c.id = rc.category_id
            WHERE rl.lat IS NOT NULL
              AND rl.lon IS NOT NULL
              AND r.status NOT IN ('pending', 'rejected')
              AND ($1::uuid IS NULL OR rl.province_id = $1)
              AND ($2::uuid IS NULL OR rl.regency_id = $2)
              AND ($3::text IS NULL OR c.slug = $3)
              AND ($4::report_status IS NULL OR r.status = $4)
            ORDER BY r.created_at DESC
            LIMIT $5
            "#,
            params.province_id,
            params.regency_id,
            params.category.as_deref(),
            params.status.as_ref() as Option<&ReportStatus>,
            limit
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch map data: {:?}", e);
            AppError::Database(e)
        })?;

        let total_count = rows.len() as i64;

        let markers: Vec<MapReportMarker> = rows
            .into_iter()
            .map(|r| MapReportMarker {
                id: r.id,
                title: r.title,
                lat: r.lat,
                lon: r.lon,
                status: r.status,
                category_slug: r.category_slug,
                category_color: r.category_color,
                created_at: r.created_at,
            })
            .collect();

        // Calculate bounds if we have markers
        let bounds = if !markers.is_empty() {
            let min_lat = markers.iter().map(|m| m.lat).fold(f64::INFINITY, f64::min);
            let min_lon = markers.iter().map(|m| m.lon).fold(f64::INFINITY, f64::min);
            let max_lat = markers
                .iter()
                .map(|m| m.lat)
                .fold(f64::NEG_INFINITY, f64::max);
            let max_lon = markers
                .iter()
                .map(|m| m.lon)
                .fold(f64::NEG_INFINITY, f64::max);
            Some([min_lat, min_lon, max_lat, max_lon])
        } else {
            None
        };

        Ok(DashboardMapDto {
            markers,
            total_count,
            bounds,
        })
    }

    pub async fn get_map_data_markers(
        &self,
        params: &LocationQueryParams,
    ) -> Result<DashboardMapDataDto> {
        // Query untuk mengambil data titik koordinat saja
        // Kita join dengan kategori untuk mendapatkan warna visual di peta
        let points = sqlx::query_as!(
            MapPointDto,
            r#"
            SELECT 
                r.id, 
                rl.lat as "lat!", 
                rl.lon as "lon!", 
                r.status as "status: ReportStatus",
                (
                    SELECT c.color 
                    FROM categories c
                    JOIN report_categories rc ON rc.category_id = c.id
                    WHERE rc.report_id = r.id
                    LIMIT 1
                ) as category_color
            FROM reports r
            JOIN report_locations rl ON rl.report_id = r.id
            WHERE rl.lat IS NOT NULL 
              AND rl.lon IS NOT NULL
              AND r.status NOT IN ('pending', 'rejected')
              AND ($1::uuid IS NULL OR rl.province_id = $1)
              AND ($2::uuid IS NULL OR rl.regency_id = $2)
            LIMIT 5000
            "#,
            params.province_id,
            params.regency_id
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch map data: {:?}", e);
            AppError::Database(e)
        })?;

        Ok(DashboardMapDataDto { points })
    }

    // ========================================================================
    // Helper functions for fetching related data
    // ========================================================================

    /// Get categories for a report
    async fn get_report_categories(&self, report_id: Uuid) -> Result<Vec<ReportCategoryInfo>> {
        let rows = sqlx::query!(
            r#"
            SELECT
                c.id as category_id,
                c.name,
                c.slug,
                rc.severity as "severity: ReportSeverity",
                c.color,
                c.icon
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
            tracing::error!("Failed to fetch report categories: {:?}", e);
            AppError::Database(e)
        })?;

        Ok(rows
            .into_iter()
            .map(|r| ReportCategoryInfo {
                category_id: r.category_id,
                name: r.name,
                slug: r.slug,
                severity: r.severity,
                color: r.color,
                icon: r.icon,
            })
            .collect())
    }

    /// Get location for a report
    async fn get_report_location(&self, report_id: Uuid) -> Result<Option<ReportLocationInfo>> {
        let row = sqlx::query!(
            r#"
            SELECT
                rl.raw_input,
                rl.display_name,
                rl.lat,
                rl.lon,
                rl.road,
                rl.city,
                rl.state,
                rl.province_id,
                p.name as "province_name?",
                rl.regency_id,
                rg.name as "regency_name?"
            FROM report_locations rl
            LEFT JOIN provinces p ON p.id = rl.province_id
            LEFT JOIN regencies rg ON rg.id = rl.regency_id
            WHERE rl.report_id = $1
            LIMIT 1
            "#,
            report_id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch report location: {:?}", e);
            AppError::Database(e)
        })?;

        Ok(row.map(|r| ReportLocationInfo {
            raw_input: r.raw_input,
            display_name: r.display_name,
            lat: r.lat,
            lon: r.lon,
            road: r.road,
            city: r.city,
            state: r.state,
            province_id: r.province_id,
            province_name: r.province_name,
            regency_id: r.regency_id,
            regency_name: r.regency_name,
        }))
    }

    /// Get primary tag for a report (first one if multiple)
    async fn get_report_tag(&self, report_id: Uuid) -> Result<Option<ReportTagType>> {
        let row = sqlx::query_scalar!(
            r#"
            SELECT tag_type as "tag_type: ReportTagType"
            FROM report_tags
            WHERE report_id = $1
            ORDER BY created_at
            LIMIT 1
            "#,
            report_id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch report tag: {:?}", e);
            AppError::Database(e)
        })?;

        Ok(row)
    }
}

// ============================================================================
// Helper functions
// ============================================================================

fn tag_label(tag_type: &ReportTagType) -> String {
    match tag_type {
        ReportTagType::Report => "Laporan".to_string(),
        ReportTagType::Proposal => "Usulan".to_string(),
        ReportTagType::Complaint => "Keluhan".to_string(),
        ReportTagType::Inquiry => "Pertanyaan".to_string(),
        ReportTagType::Appreciation => "Apresiasi".to_string(),
    }
}
