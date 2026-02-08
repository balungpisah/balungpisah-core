use sqlx::{PgPool, Row};
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

    /// List all reports with pagination, sorting, and search
    /// Returns (reports, total_count)
    pub async fn list_reports(
        &self,
        params: &ReportListParams,
    ) -> Result<(Vec<DashboardReportDto>, i64)> {
        let offset = params.offset();
        let limit = params.limit();
        let search_pattern = params.search.as_deref().map(|s| format!("%{}%", s));

        // Get total count
        let total = sqlx::query_scalar!(
            r#"
            SELECT COUNT(*) as "count!"
            FROM reports
            WHERE status IN ('draft', 'verified')
              AND ($1::text IS NULL OR title ILIKE $1)
            "#,
            search_pattern
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to count reports: {:?}", e);
            AppError::Database(e)
        })?;

        // Get reports — dynamic ORDER BY requires QueryBuilder
        let sort_col = params.sort_column();
        let sort_dir = params.sort_direction();

        let mut qb: sqlx::QueryBuilder<sqlx::Postgres> = sqlx::QueryBuilder::new(
            r#"
            SELECT
                r.id,
                r.title,
                r.description,
                r.status::text as status,
                r.timeline,
                r.impact,
                r.created_at
            FROM reports r
            WHERE r.status IN ('draft', 'verified')
            "#,
        );

        if let Some(ref pattern) = search_pattern {
            qb.push(" AND r.title ILIKE ");
            qb.push_bind(pattern);
        }

        // sort_column() and sort_direction() are validated/whitelisted, safe to push raw
        qb.push(" ORDER BY ");
        qb.push(sort_col);
        qb.push(" ");
        qb.push(sort_dir);

        qb.push(" OFFSET ");
        qb.push_bind(offset);
        qb.push(" LIMIT ");
        qb.push_bind(limit);

        let rows = qb.build().fetch_all(&self.pool).await.map_err(|e| {
            tracing::error!("Failed to fetch reports: {:?}", e);
            AppError::Database(e)
        })?;

        let mut reports = Vec::with_capacity(rows.len());
        for row in &rows {
            let id: Uuid = row.get("id");
            let status_str: String = row.get("status");
            let status = parse_status(&status_str).unwrap_or(ReportStatus::Draft);

            let categories = self.get_report_categories(id).await?;
            let location = self.get_report_location(id).await?;
            let tag_type = self.get_report_tag(id).await?;

            reports.push(DashboardReportDto {
                id,
                title: row.get("title"),
                description: row.get("description"),
                status,
                tag_type,
                timeline: row.get("timeline"),
                impact: row.get("impact"),
                created_at: row.get("created_at"),
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
        let offset = params.offset();
        let limit = params.limit();
        let search_pattern = params.search.as_deref().map(|s| format!("%{}%", s));

        let total = sqlx::query_scalar!(
            r#"
            SELECT COUNT(*) as "count!"
            FROM reports
            WHERE created_at >= CURRENT_DATE - $1::int
              AND status IN ('draft', 'verified')
              AND ($2::text IS NULL OR title ILIKE $2)
            "#,
            days,
            search_pattern
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
              AND r.status IN ('draft', 'verified')
              AND ($2::text IS NULL OR r.title ILIKE $2)
            ORDER BY r.created_at DESC
            OFFSET $3 LIMIT $4
            "#,
            days,
            search_pattern,
            offset,
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

        let pagination = PaginationMeta::new(params.page, params.page_size, total);

        Ok(DashboardRecentDto {
            reports,
            days,
            total_count: total,
            pagination,
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
    // FASE 1: Enhanced Map Markers & Comprehensive Stats (Optimized)
    // ========================================================================

    /// Get enhanced map markers — single query, all filters at SQL level
    pub async fn get_enhanced_markers(
        &self,
        params: &EnhancedMapQueryParams,
    ) -> Result<EnhancedMapDto> {
        let filters = ParsedFilters::from_params(params);

        let mut qb: sqlx::QueryBuilder<sqlx::Postgres> = sqlx::QueryBuilder::new(
            r#"
            SELECT
                r.id,
                r.reference_number,
                r.title,
                rl.lat,
                rl.lon,
                r.status::text as status,
                r.created_at,
                top_cat.severity::text as max_severity,
                top_cat.cat_name as primary_category_name,
                top_cat.color as primary_category_color,
                (SELECT rt.tag_type::text FROM report_tags rt WHERE rt.report_id = r.id LIMIT 1) as tag_type
            FROM reports r
            INNER JOIN report_locations rl ON rl.report_id = r.id
            LEFT JOIN LATERAL (
                SELECT rc.severity, c.name as cat_name, c.color
                FROM report_categories rc
                JOIN categories c ON c.id = rc.category_id
                WHERE rc.report_id = r.id
                ORDER BY CASE rc.severity
                    WHEN 'critical' THEN 1 WHEN 'high' THEN 2
                    WHEN 'medium' THEN 3 WHEN 'low' THEN 4
                END
                LIMIT 1
            ) top_cat ON true
            WHERE rl.lat IS NOT NULL AND rl.lon IS NOT NULL
            "#,
        );

        filters.apply_to(&mut qb);

        qb.push(" ORDER BY r.created_at DESC LIMIT ");
        qb.push_bind(params.limit.min(1000));

        let rows = qb.build().fetch_all(&self.pool).await.map_err(|e| {
            tracing::error!("Failed to fetch enhanced markers: {:?}", e);
            AppError::Database(e)
        })?;

        let markers: Vec<EnhancedMapMarker> = rows
            .iter()
            .map(|row| {
                let severity_str: Option<String> = row.get("max_severity");
                let max_severity = severity_str
                    .as_deref()
                    .and_then(parse_severity)
                    .unwrap_or(ReportSeverity::Low);

                let tag_str: Option<String> = row.get("tag_type");
                let tag_type = tag_str.as_deref().and_then(parse_tag_type);

                let status_str: String = row.get("status");
                let status = parse_status(&status_str).unwrap_or(ReportStatus::Pending);

                EnhancedMapMarker {
                    id: row.get("id"),
                    reference_number: row.get("reference_number"),
                    title: row.get("title"),
                    lat: row.get("lat"),
                    lon: row.get("lon"),
                    max_severity,
                    primary_category_name: row.get("primary_category_name"),
                    primary_category_color: row.get("primary_category_color"),
                    tag_type,
                    status,
                    created_at: row.get("created_at"),
                }
            })
            .collect();

        let total_count = markers.len() as i64;

        Ok(EnhancedMapDto {
            markers,
            total_count,
        })
    }

    /// Get comprehensive statistics — all filters applied at SQL level
    pub async fn get_comprehensive_stats(
        &self,
        params: &EnhancedMapQueryParams,
    ) -> Result<ComprehensiveStatsDto> {
        let filters = ParsedFilters::from_params(params);

        // --- Total + status breakdown ---
        let mut qb: sqlx::QueryBuilder<sqlx::Postgres> = sqlx::QueryBuilder::new(
            r#"
            SELECT
                COUNT(DISTINCT r.id) as total,
                COUNT(DISTINCT r.id) FILTER (WHERE r.status = 'draft') as draft_cnt,
                COUNT(DISTINCT r.id) FILTER (WHERE r.status = 'verified') as verified_cnt
            FROM reports r
            INNER JOIN report_locations rl ON rl.report_id = r.id
            WHERE rl.lat IS NOT NULL AND rl.lon IS NOT NULL
            "#,
        );
        // For the total query, skip the status filter so we can count all statuses
        filters.apply_to_skip_status(&mut qb);

        let stats_row = qb.build().fetch_one(&self.pool).await?;

        let total: i64 = stats_row.get("total");
        let by_status = StatusBreakdown {
            draft: stats_row.get("draft_cnt"),
            verified: stats_row.get("verified_cnt"),
        };

        // --- Severity breakdown ---
        let mut qb: sqlx::QueryBuilder<sqlx::Postgres> = sqlx::QueryBuilder::new(
            r#"
            SELECT
                rc.severity::text as severity,
                COUNT(DISTINCT r.id) as cnt
            FROM reports r
            INNER JOIN report_locations rl ON rl.report_id = r.id
            INNER JOIN report_categories rc ON rc.report_id = r.id
            WHERE rl.lat IS NOT NULL AND rl.lon IS NOT NULL
            "#,
        );
        filters.apply_to(&mut qb);
        qb.push(" GROUP BY rc.severity::text");

        let sev_rows = qb.build().fetch_all(&self.pool).await?;

        let mut by_severity = SeverityBreakdown {
            critical: 0,
            high: 0,
            medium: 0,
            low: 0,
        };
        for row in &sev_rows {
            let sev: String = row.get("severity");
            let cnt: i64 = row.get("cnt");
            match sev.as_str() {
                "critical" => by_severity.critical = cnt,
                "high" => by_severity.high = cnt,
                "medium" => by_severity.medium = cnt,
                "low" => by_severity.low = cnt,
                _ => {}
            }
        }

        // --- Tag breakdown ---
        let mut qb: sqlx::QueryBuilder<sqlx::Postgres> = sqlx::QueryBuilder::new(
            r#"
            SELECT
                rt.tag_type::text as tag_type,
                COUNT(DISTINCT r.id) as cnt
            FROM reports r
            INNER JOIN report_locations rl ON rl.report_id = r.id
            INNER JOIN report_tags rt ON rt.report_id = r.id
            WHERE rl.lat IS NOT NULL AND rl.lon IS NOT NULL
            "#,
        );
        filters.apply_to(&mut qb);
        qb.push(" GROUP BY rt.tag_type::text");

        let tag_rows = qb.build().fetch_all(&self.pool).await?;

        let mut by_tag = TagBreakdown {
            report: 0,
            complaint: 0,
            proposal: 0,
            inquiry: 0,
            appreciation: 0,
        };
        for row in &tag_rows {
            let tag: String = row.get("tag_type");
            let cnt: i64 = row.get("cnt");
            match tag.as_str() {
                "report" => by_tag.report = cnt,
                "complaint" => by_tag.complaint = cnt,
                "proposal" => by_tag.proposal = cnt,
                "inquiry" => by_tag.inquiry = cnt,
                "appreciation" => by_tag.appreciation = cnt,
                _ => {}
            }
        }

        // --- Top 10 categories ---
        let mut qb: sqlx::QueryBuilder<sqlx::Postgres> = sqlx::QueryBuilder::new(
            r#"
            SELECT
                c.id as category_id,
                c.name as category_name,
                COUNT(DISTINCT r.id) as cnt
            FROM reports r
            INNER JOIN report_locations rl ON rl.report_id = r.id
            INNER JOIN report_categories rc ON rc.report_id = r.id
            INNER JOIN categories c ON c.id = rc.category_id
            WHERE rl.lat IS NOT NULL AND rl.lon IS NOT NULL
            "#,
        );
        filters.apply_to(&mut qb);
        qb.push(" GROUP BY c.id, c.name ORDER BY cnt DESC LIMIT 10");

        let cat_rows = qb.build().fetch_all(&self.pool).await?;

        let by_category: Vec<CategoryCount> = cat_rows
            .iter()
            .map(|row| CategoryCount {
                category_id: row.get("category_id"),
                category_name: row.get("category_name"),
                count: row.get::<i64, _>("cnt"),
            })
            .collect();

        // --- Weekly trend (last 12 weeks) ---
        let mut qb: sqlx::QueryBuilder<sqlx::Postgres> = sqlx::QueryBuilder::new(
            r#"
            SELECT
                TO_CHAR(date_trunc('week', r.created_at), 'IYYY-"W"IW') as week,
                COUNT(DISTINCT r.id) as cnt
            FROM reports r
            INNER JOIN report_locations rl ON rl.report_id = r.id
            WHERE rl.lat IS NOT NULL AND rl.lon IS NOT NULL
                AND r.created_at >= NOW() - INTERVAL '12 weeks'
            "#,
        );
        filters.apply_to(&mut qb);
        qb.push(
            " GROUP BY date_trunc('week', r.created_at) ORDER BY date_trunc('week', r.created_at)",
        );

        let period_rows = qb.build().fetch_all(&self.pool).await?;

        let by_period: Vec<WeeklyCount> = period_rows
            .iter()
            .map(|row| WeeklyCount {
                week: row.get("week"),
                count: row.get::<i64, _>("cnt"),
            })
            .collect();

        // --- Top 10 regions ---
        let mut qb: sqlx::QueryBuilder<sqlx::Postgres> = sqlx::QueryBuilder::new(
            r#"
            SELECT
                p.id as region_id,
                p.name as region_name,
                COUNT(DISTINCT r.id) as cnt
            FROM reports r
            INNER JOIN report_locations rl ON rl.report_id = r.id
            INNER JOIN provinces p ON p.id = rl.province_id
            WHERE rl.lat IS NOT NULL AND rl.lon IS NOT NULL
            "#,
        );
        filters.apply_to(&mut qb);
        qb.push(" GROUP BY p.id, p.name ORDER BY cnt DESC LIMIT 10");

        let region_rows = qb.build().fetch_all(&self.pool).await?;

        let by_region: Vec<RegionCount> = region_rows
            .iter()
            .map(|row| RegionCount {
                region_id: row.get("region_id"),
                region_name: row.get("region_name"),
                region_type: "province".to_string(),
                count: row.get::<i64, _>("cnt"),
            })
            .collect();

        Ok(ComprehensiveStatsDto {
            total,
            by_severity,
            by_status,
            by_tag,
            by_category,
            by_period,
            by_region,
        })
    }

    // ========================================================================
    // FASE 2: Cluster Analysis
    // ========================================================================

    /// Perform cluster analysis on reports
    pub async fn cluster_reports(&self, request: &ClusterRequest) -> Result<ClusterAnalysisDto> {
        // 1. Fetch reports with applied filters (reuse optimized markers query)
        let markers = self.get_enhanced_markers(&request.filters).await?;

        if markers.markers.is_empty() {
            return Ok(ClusterAnalysisDto {
                clusters: vec![],
                total_reports: 0,
                total_clusters: 0,
            });
        }

        // 2. Batch-fetch categories for all reports in one query
        let report_ids: Vec<Uuid> = markers.markers.iter().map(|m| m.id).collect();
        let cat_rows = sqlx::query(
            r#"
            SELECT rc.report_id, c.name
            FROM report_categories rc
            JOIN categories c ON c.id = rc.category_id
            WHERE rc.report_id = ANY($1)
            "#,
        )
        .bind(&report_ids)
        .fetch_all(&self.pool)
        .await?;

        // Build report_id -> categories map
        let mut categories_map: std::collections::HashMap<Uuid, Vec<String>> =
            std::collections::HashMap::new();
        for row in &cat_rows {
            let rid: Uuid = row.get("report_id");
            let name: String = row.get("name");
            categories_map.entry(rid).or_default().push(name);
        }

        // 3. Build report data
        let report_data: Vec<ReportData> = markers
            .markers
            .iter()
            .map(|marker| ReportData {
                id: marker.id,
                lat: marker.lat,
                lon: marker.lon,
                max_severity: marker.max_severity,
                primary_category: marker.primary_category_name.clone(),
                categories: categories_map.get(&marker.id).cloned().unwrap_or_default(),
                created_at: marker.created_at,
            })
            .collect();

        // 4. Perform clustering based on mode
        let clusters = match request.mode {
            ClusterMode::Geographic => {
                self.cluster_geographic(&report_data, request.radius_km)
                    .await?
            }
            ClusterMode::Category => {
                self.cluster_by_category(&report_data, request.radius_km)
                    .await?
            }
        };

        let total_reports = report_data.len() as i64;
        let total_clusters = clusters.len() as i64;

        Ok(ClusterAnalysisDto {
            clusters,
            total_reports,
            total_clusters,
        })
    }

    /// Geographic clustering: group by proximity only
    async fn cluster_geographic(
        &self,
        reports: &[ReportData],
        radius_km: f64,
    ) -> Result<Vec<ReportCluster>> {
        let mut clusters: Vec<Vec<usize>> = Vec::new();
        let mut visited = vec![false; reports.len()];

        // Simple DBSCAN-like algorithm
        for i in 0..reports.len() {
            if visited[i] {
                continue;
            }

            let mut cluster = vec![i];
            visited[i] = true;

            // Find all reports within radius
            for j in (i + 1)..reports.len() {
                if visited[j] {
                    continue;
                }

                let distance = haversine_distance(
                    reports[i].lat,
                    reports[i].lon,
                    reports[j].lat,
                    reports[j].lon,
                );

                if distance <= radius_km {
                    cluster.push(j);
                    visited[j] = true;
                }
            }

            clusters.push(cluster);
        }

        // Convert to ReportCluster
        self.build_clusters(reports, clusters).await
    }

    /// Category-based clustering: same category + proximity
    async fn cluster_by_category(
        &self,
        reports: &[ReportData],
        radius_km: f64,
    ) -> Result<Vec<ReportCluster>> {
        let mut clusters: Vec<Vec<usize>> = Vec::new();
        let mut visited = vec![false; reports.len()];

        for i in 0..reports.len() {
            if visited[i] {
                continue;
            }

            let mut cluster = vec![i];
            visited[i] = true;

            // Find reports with same category AND within radius
            for j in (i + 1)..reports.len() {
                if visited[j] {
                    continue;
                }

                // Check if they share at least one category
                let has_common_category = reports[i]
                    .categories
                    .iter()
                    .any(|cat| reports[j].categories.contains(cat));

                if !has_common_category {
                    continue;
                }

                let distance = haversine_distance(
                    reports[i].lat,
                    reports[i].lon,
                    reports[j].lat,
                    reports[j].lon,
                );

                if distance <= radius_km {
                    cluster.push(j);
                    visited[j] = true;
                }
            }

            clusters.push(cluster);
        }

        self.build_clusters(reports, clusters).await
    }

    /// Build ReportCluster objects from indices
    async fn build_clusters(
        &self,
        reports: &[ReportData],
        cluster_indices: Vec<Vec<usize>>,
    ) -> Result<Vec<ReportCluster>> {
        let mut result = Vec::new();

        for (idx, indices) in cluster_indices.iter().enumerate() {
            if indices.is_empty() {
                continue;
            }

            // Calculate cluster center (average of coordinates)
            let center_lat =
                indices.iter().map(|&i| reports[i].lat).sum::<f64>() / indices.len() as f64;
            let center_lon =
                indices.iter().map(|&i| reports[i].lon).sum::<f64>() / indices.len() as f64;

            // Find max severity
            let max_severity = indices
                .iter()
                .map(|&i| &reports[i].max_severity)
                .max_by_key(|s| match s {
                    ReportSeverity::Critical => 4,
                    ReportSeverity::High => 3,
                    ReportSeverity::Medium => 2,
                    ReportSeverity::Low => 1,
                })
                .cloned()
                .unwrap_or(ReportSeverity::Low);

            // Find dominant category (most common)
            let mut category_counts: std::collections::HashMap<String, usize> =
                std::collections::HashMap::new();
            for &i in indices {
                if let Some(ref cat) = reports[i].primary_category {
                    *category_counts.entry(cat.clone()).or_insert(0) += 1;
                }
            }
            let dominant_category = category_counts
                .into_iter()
                .max_by_key(|(_, count)| *count)
                .map(|(cat, _)| cat)
                .unwrap_or_else(|| "Unknown".to_string());

            // Date range
            let dates: Vec<_> = indices.iter().map(|&i| reports[i].created_at).collect();
            let min_date = dates.iter().min().copied().unwrap();
            let max_date = dates.iter().max().copied().unwrap();

            // Report IDs
            let report_ids: Vec<Uuid> = indices.iter().map(|&i| reports[i].id).collect();

            // Get location name for label (approximate from first report)
            let first_report_location = self.get_report_location(reports[indices[0]].id).await?;
            let area_name = first_report_location
                .as_ref()
                .and_then(|loc| loc.regency_name.clone())
                .or_else(|| {
                    first_report_location
                        .as_ref()
                        .and_then(|loc| loc.province_name.clone())
                })
                .unwrap_or_else(|| "Unknown Area".to_string());

            // Generate label
            let label = format!("{} — {}", dominant_category, area_name);

            // Count unique citizens (would need user_id from reports table)
            // For now, approximate as report count
            let citizen_count = indices.len() as i64;

            result.push(ReportCluster {
                cluster_id: format!("cluster_{}", idx + 1),
                label,
                center_lat,
                center_lon,
                report_count: indices.len() as i64,
                citizen_count,
                max_severity,
                dominant_category,
                date_range: DateRange {
                    from: min_date,
                    to: max_date,
                },
                report_ids,
            });
        }

        Ok(result)
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
// ParsedFilters — shared filter builder for SQL queries
// ============================================================================

/// Pre-parsed and validated filter values
struct ParsedFilters {
    province_id: Option<Uuid>,
    regency_id: Option<Uuid>,
    district_id: Option<Uuid>,
    village_id: Option<Uuid>,
    category_ids: Vec<Uuid>,
    severities: Vec<String>,
    tag_types: Vec<String>,
    statuses: Vec<String>,
    date_from: Option<chrono::NaiveDate>,
    date_to: Option<chrono::NaiveDate>,
    bounds: Option<(f64, f64, f64, f64)>, // sw_lat, sw_lon, ne_lat, ne_lon
}

impl ParsedFilters {
    fn from_params(params: &EnhancedMapQueryParams) -> Self {
        let category_ids = params
            .category_ids
            .as_deref()
            .map(|s| {
                s.split(',')
                    .filter_map(|v| Uuid::parse_str(v.trim()).ok())
                    .collect()
            })
            .unwrap_or_default();

        let severities = params
            .severity
            .as_deref()
            .map(|s| {
                s.split(',')
                    .map(|v| v.trim().to_lowercase())
                    .filter(|v| ["critical", "high", "medium", "low"].contains(&v.as_str()))
                    .collect()
            })
            .unwrap_or_default();

        let tag_types = params
            .tag_types
            .as_deref()
            .map(|s| {
                s.split(',')
                    .map(|v| v.trim().to_lowercase())
                    .filter(|v| {
                        ["report", "complaint", "proposal", "inquiry", "appreciation"]
                            .contains(&v.as_str())
                    })
                    .collect()
            })
            .unwrap_or_default();

        let statuses = params
            .status
            .as_deref()
            .map(|s| {
                s.split(',')
                    .map(|v| v.trim().to_lowercase())
                    .filter(|v| {
                        ["draft", "pending", "verified", "rejected", "resolved"]
                            .contains(&v.as_str())
                    })
                    .collect()
            })
            .unwrap_or_default();

        let date_from = params
            .date_from
            .as_deref()
            .and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok());

        let date_to = params
            .date_to
            .as_deref()
            .and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok());

        let bounds = params.bounds.as_deref().and_then(|s| {
            let parts: Vec<f64> = s.split(',').filter_map(|v| v.trim().parse().ok()).collect();
            if parts.len() == 4 {
                Some((parts[0], parts[1], parts[2], parts[3]))
            } else {
                None
            }
        });

        Self {
            province_id: params.province_id,
            regency_id: params.regency_id,
            district_id: params.district_id,
            village_id: params.village_id,
            category_ids,
            severities,
            tag_types,
            statuses,
            date_from,
            date_to,
            bounds,
        }
    }

    /// Append WHERE conditions to a QueryBuilder (assumes WHERE already started)
    fn apply_to<'a>(&'a self, qb: &mut sqlx::QueryBuilder<'a, sqlx::Postgres>) {
        self.apply_common(qb);

        // Status filter
        if !self.statuses.is_empty() {
            qb.push(" AND r.status::text = ANY(");
            qb.push_bind(&self.statuses);
            qb.push(")");
        } else {
            qb.push(" AND r.status::text IN ('draft', 'verified')");
        }
    }

    /// Same as apply_to but skips status filter (used for total/status breakdown)
    fn apply_to_skip_status<'a>(&'a self, qb: &mut sqlx::QueryBuilder<'a, sqlx::Postgres>) {
        self.apply_common(qb);
        // Default: only draft and verified
        qb.push(" AND r.status::text IN ('draft', 'verified')");
    }

    fn apply_common<'a>(&'a self, qb: &mut sqlx::QueryBuilder<'a, sqlx::Postgres>) {
        // Region filters
        if let Some(province_id) = self.province_id {
            qb.push(" AND rl.province_id = ");
            qb.push_bind(province_id);
        }
        if let Some(regency_id) = self.regency_id {
            qb.push(" AND rl.regency_id = ");
            qb.push_bind(regency_id);
        }
        if let Some(district_id) = self.district_id {
            qb.push(" AND rl.district_id = ");
            qb.push_bind(district_id);
        }
        if let Some(village_id) = self.village_id {
            qb.push(" AND rl.village_id = ");
            qb.push_bind(village_id);
        }

        // Category filter
        if !self.category_ids.is_empty() {
            qb.push(
                " AND EXISTS (SELECT 1 FROM report_categories rc2 WHERE rc2.report_id = r.id AND rc2.category_id = ANY(",
            );
            qb.push_bind(&self.category_ids);
            qb.push("))");
        }

        // Severity filter
        if !self.severities.is_empty() {
            qb.push(
                " AND EXISTS (SELECT 1 FROM report_categories rc3 WHERE rc3.report_id = r.id AND rc3.severity::text = ANY(",
            );
            qb.push_bind(&self.severities);
            qb.push("))");
        }

        // Tag types filter
        if !self.tag_types.is_empty() {
            qb.push(
                " AND EXISTS (SELECT 1 FROM report_tags rt2 WHERE rt2.report_id = r.id AND rt2.tag_type::text = ANY(",
            );
            qb.push_bind(&self.tag_types);
            qb.push("))");
        }

        // Date range filters
        if let Some(date_from) = self.date_from {
            qb.push(" AND r.created_at >= ");
            qb.push_bind(date_from);
        }
        if let Some(date_to) = self.date_to {
            qb.push(" AND r.created_at < (");
            qb.push_bind(date_to);
            qb.push(" + INTERVAL '1 day')");
        }

        // Viewport bounds filter
        if let Some((sw_lat, sw_lon, ne_lat, ne_lon)) = self.bounds {
            qb.push(" AND rl.lat BETWEEN ");
            qb.push_bind(sw_lat);
            qb.push(" AND ");
            qb.push_bind(ne_lat);
            qb.push(" AND rl.lon BETWEEN ");
            qb.push_bind(sw_lon);
            qb.push(" AND ");
            qb.push_bind(ne_lon);
        }
    }
}

// ============================================================================
// Parse helpers for runtime query results
// ============================================================================

fn parse_severity(s: &str) -> Option<ReportSeverity> {
    match s {
        "critical" => Some(ReportSeverity::Critical),
        "high" => Some(ReportSeverity::High),
        "medium" => Some(ReportSeverity::Medium),
        "low" => Some(ReportSeverity::Low),
        _ => None,
    }
}

fn parse_tag_type(s: &str) -> Option<ReportTagType> {
    match s {
        "report" => Some(ReportTagType::Report),
        "complaint" => Some(ReportTagType::Complaint),
        "proposal" => Some(ReportTagType::Proposal),
        "inquiry" => Some(ReportTagType::Inquiry),
        "appreciation" => Some(ReportTagType::Appreciation),
        _ => None,
    }
}

fn parse_status(s: &str) -> Option<ReportStatus> {
    match s {
        "draft" => Some(ReportStatus::Draft),
        "pending" => Some(ReportStatus::Pending),
        "verified" => Some(ReportStatus::Verified),
        "rejected" => Some(ReportStatus::Rejected),
        "resolved" => Some(ReportStatus::Resolved),
        _ => None,
    }
}

fn tag_label(tag_type: &ReportTagType) -> String {
    match tag_type {
        ReportTagType::Report => "Laporan".to_string(),
        ReportTagType::Proposal => "Usulan".to_string(),
        ReportTagType::Complaint => "Keluhan".to_string(),
        ReportTagType::Inquiry => "Pertanyaan".to_string(),
        ReportTagType::Appreciation => "Apresiasi".to_string(),
    }
}

// ============================================================================
// Clustering Helper Structures
// ============================================================================

/// Internal structure for clustering
#[derive(Debug, Clone)]
struct ReportData {
    id: Uuid,
    lat: f64,
    lon: f64,
    max_severity: ReportSeverity,
    primary_category: Option<String>,
    categories: Vec<String>,
    created_at: chrono::DateTime<chrono::Utc>,
}

/// Calculate distance between two points using Haversine formula
/// Returns distance in kilometers
fn haversine_distance(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    const EARTH_RADIUS_KM: f64 = 6371.0;

    let lat1_rad = lat1.to_radians();
    let lat2_rad = lat2.to_radians();
    let delta_lat = (lat2 - lat1).to_radians();
    let delta_lon = (lon2 - lon1).to_radians();

    let a = (delta_lat / 2.0).sin().powi(2)
        + lat1_rad.cos() * lat2_rad.cos() * (delta_lon / 2.0).sin().powi(2);

    let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());

    EARTH_RADIUS_KM * c
}
