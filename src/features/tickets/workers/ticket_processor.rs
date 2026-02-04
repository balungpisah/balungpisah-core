use sqlx::PgPool;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::interval;
use uuid::Uuid;

use crate::core::error::{AppError, Result};
use crate::features::reports::models::{CreateReport, CreateReportCategory};
use crate::features::reports::services::ExtractionService;
use crate::features::reports::services::{
    ClusteringService, GeocodingLevel, GeocodingService, RegionLookupService, ReportService,
};
use crate::features::tickets::models::{Ticket, TicketStatus};

/// Maximum retry attempts for failed tickets
const MAX_RETRIES: i32 = 3;

/// Delay between processing batches
const BATCH_INTERVAL_SECS: u64 = 30;

/// Batch size for processing
const BATCH_SIZE: i64 = 10;

/// Ticket processor worker that runs in the background
pub struct TicketProcessor {
    pool: PgPool,
    extraction_service: Arc<ExtractionService>,
    geocoding_service: Arc<GeocodingService>,
    clustering_service: Arc<ClusteringService>,
    report_service: Arc<ReportService>,
    region_lookup_service: Arc<RegionLookupService>,
}

impl TicketProcessor {
    pub fn new(
        pool: PgPool,
        extraction_service: Arc<ExtractionService>,
        geocoding_service: Arc<GeocodingService>,
        clustering_service: Arc<ClusteringService>,
        report_service: Arc<ReportService>,
        region_lookup_service: Arc<RegionLookupService>,
    ) -> Self {
        Self {
            pool,
            extraction_service,
            geocoding_service,
            clustering_service,
            report_service,
            region_lookup_service,
        }
    }

    /// Run the processor in a background loop
    pub async fn run(&self) {
        tracing::info!("Starting ticket processor worker");

        let mut interval = interval(Duration::from_secs(BATCH_INTERVAL_SECS));

        loop {
            interval.tick().await;

            if let Err(e) = self.process_batch().await {
                tracing::error!("Error processing ticket batch: {:?}", e);
            }
        }
    }

    /// Process a batch of pending tickets
    async fn process_batch(&self) -> Result<()> {
        // Fetch submitted tickets that haven't been processed
        let tickets = self.fetch_pending_tickets().await?;

        if tickets.is_empty() {
            return Ok(());
        }

        tracing::info!("Processing {} pending tickets", tickets.len());

        for ticket in tickets {
            if let Err(e) = self.process_ticket(&ticket).await {
                tracing::error!("Failed to process ticket {}: {:?}", ticket.id, e);
                self.mark_failed(&ticket, &e.to_string()).await?;
            }
        }

        Ok(())
    }

    /// Fetch pending tickets for processing
    async fn fetch_pending_tickets(&self) -> Result<Vec<Ticket>> {
        sqlx::query_as!(
            Ticket,
            r#"
            SELECT
                id, adk_thread_id, user_id, reference_number, platform,
                confidence_score, completeness_score, missing_fields, preliminary_data,
                status as "status: TicketStatus",
                submitted_at, processed_at, error_message, retry_count,
                last_attempt_at, report_id,
                created_at, updated_at
            FROM tickets
            WHERE status = 'submitted'
            AND retry_count < $1
            ORDER BY submitted_at ASC
            LIMIT $2
            "#,
            MAX_RETRIES,
            BATCH_SIZE
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch pending tickets: {:?}", e);
            AppError::Database(e)
        })
    }

    /// Process a single ticket
    async fn process_ticket(&self, ticket: &Ticket) -> Result<()> {
        tracing::info!(
            "Processing ticket: {} ({})",
            ticket.id,
            ticket.reference_number
        );

        // Mark as processing
        self.update_status(ticket.id, TicketStatus::Processing)
            .await?;

        // Extract data from conversation using LLM
        let extracted = self
            .extraction_service
            .extract_from_thread(ticket.adk_thread_id)
            .await?;

        // Create report (without category_id/severity - those are now in junction table)
        let create_report = CreateReport {
            ticket_id: ticket.id,
            title: extracted.title,
            description: extracted.description,
            timeline: extracted.timeline,
            impact: extracted.impact,
        };

        let report = self.report_service.create(&create_report).await?;
        tracing::info!("Created report: {} for ticket: {}", report.id, ticket.id);

        // Assign multiple categories with their severities
        if !extracted.categories.is_empty() {
            let mut category_assignments = Vec::new();

            for cat in &extracted.categories {
                if let Some(category_id) = self.lookup_category_id(&cat.slug).await? {
                    category_assignments.push(CreateReportCategory {
                        report_id: report.id,
                        category_id,
                        severity: cat.severity,
                    });
                } else {
                    tracing::warn!(
                        "Category slug '{}' not found, skipping for report {}",
                        cat.slug,
                        report.id
                    );
                }
            }

            if !category_assignments.is_empty() {
                self.report_service
                    .assign_categories(report.id, &category_assignments)
                    .await?;
                tracing::info!(
                    "Assigned {} categories to report {}",
                    category_assignments.len(),
                    report.id
                );
            }
        }

        // Add tag if extracted
        if let Some(tag_type) = extracted.tag_type {
            self.report_service.add_tags(report.id, &[tag_type]).await?;
            tracing::info!("Added tag {:?} to report {}", tag_type, report.id);
        }

        // Geocode location if we have at least regency or district
        let has_location = extracted.location_regency.is_some()
            || extracted.location_district.is_some()
            || extracted.location_village.is_some()
            || extracted.location_province.is_some();

        if has_location {
            // Use cascading geocoding: tries most specific to least specific
            let geocode_result = self
                .geocoding_service
                .geocode_cascading(
                    extracted.location_village.as_deref(),
                    extracted.location_district.as_deref(),
                    extracted.location_regency.as_deref(),
                    extracted.location_province.as_deref(),
                )
                .await?;

            // Build raw_input for storage
            let raw_input = extracted
                .location_raw
                .clone()
                .or_else(|| {
                    let parts: Vec<&str> = [
                        extracted.location_street.as_deref(),
                        extracted.location_village.as_deref(),
                        extracted.location_district.as_deref(),
                        extracted.location_regency.as_deref(),
                        extracted.location_province.as_deref(),
                    ]
                    .into_iter()
                    .flatten()
                    .collect();

                    if parts.is_empty() {
                        None
                    } else {
                        Some(parts.join(", "))
                    }
                })
                .unwrap_or_default();

            // Extract response and level from CascadingGeocodingResult
            let (nominatim_response, geocoding_level) = match geocode_result {
                Some(result) => (Some(result.response), Some(result.level)),
                None => (None, None),
            };

            // Get location fields from Nominatim response if LLM didn't extract them
            let nominatim_regency = nominatim_response
                .as_ref()
                .and_then(|r| r.address.as_ref())
                .and_then(|a| a.get_regency());
            let nominatim_district = nominatim_response
                .as_ref()
                .and_then(|r| r.address.as_ref())
                .and_then(|a| a.get_district());
            let nominatim_village = nominatim_response
                .as_ref()
                .and_then(|r| r.address.as_ref())
                .and_then(|a| a.get_village());

            // Use LLM-extracted values first, fallback to Nominatim response
            let regency = extracted
                .location_regency
                .as_deref()
                .or(nominatim_regency.as_deref());
            let district = extracted
                .location_district
                .as_deref()
                .or(nominatim_district.as_deref());
            let village = extracted
                .location_village
                .as_deref()
                .or(nominatim_village.as_deref());

            // Resolve region FKs based on the geocoding level that succeeded
            let resolved_regions = self
                .region_lookup_service
                .resolve(
                    regency,
                    extracted.location_province.as_deref(),
                    // Only resolve district if geocoding level is Village or District
                    match geocoding_level {
                        Some(GeocodingLevel::Village) | Some(GeocodingLevel::District) => district,
                        _ => None,
                    },
                    // Only resolve village if geocoding level is Village
                    match geocoding_level {
                        Some(GeocodingLevel::Village) => village,
                        _ => None,
                    },
                )
                .await?;

            let mut create_location = self.geocoding_service.to_create_location(
                report.id,
                raw_input,
                nominatim_response,
                crate::features::reports::services::LocationNames {
                    street: extracted.location_street.as_deref(),
                    village: extracted.location_village.as_deref(),
                    district: extracted.location_district.as_deref(),
                    regency: extracted.location_regency.as_deref(),
                    province: extracted.location_province.as_deref(),
                },
            );

            // Set region FKs based on geocoding level
            create_location.province_id = resolved_regions.province_id;
            create_location.regency_id = resolved_regions.regency_id;
            create_location.district_id = match geocoding_level {
                Some(GeocodingLevel::Village) | Some(GeocodingLevel::District) => {
                    resolved_regions.district_id
                }
                _ => None,
            };
            create_location.village_id = match geocoding_level {
                Some(GeocodingLevel::Village) => resolved_regions.village_id,
                _ => None,
            };

            let location = self
                .report_service
                .create_location(&create_location)
                .await?;
            tracing::info!(
                "Created report location: {} (level={:?}, province={:?}, regency={:?}, district={:?}, village={:?})",
                location.id,
                geocoding_level,
                create_location.province_id,
                create_location.regency_id,
                create_location.district_id,
                create_location.village_id
            );

            // Cluster by location if we have coordinates
            if let (Some(lat), Some(lon)) = (create_location.lat, create_location.lon) {
                let location_name = create_location
                    .city
                    .as_ref()
                    .or(create_location.suburb.as_ref())
                    .map(|s| s.as_str());

                let cluster_id = self
                    .clustering_service
                    .find_or_create_cluster(lat, lon, location_name)
                    .await?;

                self.report_service
                    .set_cluster(report.id, cluster_id)
                    .await?;
                tracing::info!("Assigned report {} to cluster {}", report.id, cluster_id);
            }
        }

        // Mark ticket as completed
        self.complete_ticket(ticket.id, report.id).await?;

        Ok(())
    }

    /// Look up category ID by slug
    async fn lookup_category_id(&self, slug: &str) -> Result<Option<Uuid>> {
        let result = sqlx::query_scalar!(
            r#"SELECT id FROM categories WHERE slug = $1 AND is_active = true"#,
            slug
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to lookup category: {:?}", e);
            AppError::Database(e)
        })?;

        Ok(result)
    }

    /// Update ticket status
    async fn update_status(&self, ticket_id: Uuid, status: TicketStatus) -> Result<()> {
        sqlx::query!(
            r#"
            UPDATE tickets
            SET status = $2, last_attempt_at = NOW(), updated_at = NOW()
            WHERE id = $1
            "#,
            ticket_id,
            status as TicketStatus
        )
        .execute(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to update ticket status: {:?}", e);
            AppError::Database(e)
        })?;

        Ok(())
    }

    /// Mark ticket as completed and link to report
    async fn complete_ticket(&self, ticket_id: Uuid, report_id: Uuid) -> Result<()> {
        sqlx::query!(
            r#"
            UPDATE tickets
            SET
                status = $2,
                processed_at = NOW(),
                report_id = $3,
                updated_at = NOW()
            WHERE id = $1
            "#,
            ticket_id,
            TicketStatus::Completed as TicketStatus,
            report_id
        )
        .execute(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to complete ticket: {:?}", e);
            AppError::Database(e)
        })?;

        tracing::info!("Ticket {} completed with report {}", ticket_id, report_id);

        Ok(())
    }

    /// Mark ticket as failed
    async fn mark_failed(&self, ticket: &Ticket, error_message: &str) -> Result<()> {
        let new_retry_count = ticket.retry_count + 1;
        let new_status = if new_retry_count >= MAX_RETRIES {
            TicketStatus::Failed
        } else {
            TicketStatus::Submitted // Keep as submitted for retry
        };

        sqlx::query!(
            r#"
            UPDATE tickets
            SET
                status = $2,
                error_message = $3,
                retry_count = $4,
                last_attempt_at = NOW(),
                updated_at = NOW()
            WHERE id = $1
            "#,
            ticket.id,
            new_status as TicketStatus,
            error_message,
            new_retry_count
        )
        .execute(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to mark ticket as failed: {:?}", e);
            AppError::Database(e)
        })?;

        if new_retry_count >= MAX_RETRIES {
            tracing::warn!(
                "Ticket {} permanently failed after {} retries",
                ticket.id,
                MAX_RETRIES
            );
        } else {
            tracing::info!(
                "Ticket {} marked for retry ({}/{})",
                ticket.id,
                new_retry_count,
                MAX_RETRIES
            );
        }

        Ok(())
    }
}
