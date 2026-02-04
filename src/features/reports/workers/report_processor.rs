use sqlx::PgPool;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::interval;
use uuid::Uuid;

use crate::core::error::{AppError, Result};
use crate::features::reports::models::{CreateReportCategory, ReportJob, ReportJobStatus};
use crate::features::reports::services::ExtractionService;
use crate::features::reports::services::{
    GeocodingService, RegionLookupService, ReportJobService, ReportService,
};

/// Maximum retry attempts for failed jobs
const MAX_RETRIES: i32 = 3;

/// Delay between processing batches
const BATCH_INTERVAL_SECS: u64 = 30;

/// Batch size for processing
const BATCH_SIZE: i64 = 10;

/// Minimum confidence score required for processing
/// Reports below this threshold will be rejected
const MIN_CONFIDENCE_THRESHOLD: f64 = 0.7;

/// Report processor worker that runs in the background
/// Processes report submissions by extracting data from conversations
pub struct ReportProcessor {
    pool: PgPool,
    extraction_service: Arc<ExtractionService>,
    geocoding_service: Arc<GeocodingService>,
    report_service: Arc<ReportService>,
    report_job_service: Arc<ReportJobService>,
    region_lookup_service: Arc<RegionLookupService>,
}

impl ReportProcessor {
    pub fn new(
        pool: PgPool,
        extraction_service: Arc<ExtractionService>,
        geocoding_service: Arc<GeocodingService>,
        report_service: Arc<ReportService>,
        report_job_service: Arc<ReportJobService>,
        region_lookup_service: Arc<RegionLookupService>,
    ) -> Self {
        Self {
            pool,
            extraction_service,
            geocoding_service,
            report_service,
            report_job_service,
            region_lookup_service,
        }
    }

    /// Run the processor in a background loop
    pub async fn run(&self) {
        tracing::info!("Starting report processor worker");

        let mut interval = interval(Duration::from_secs(BATCH_INTERVAL_SECS));

        loop {
            interval.tick().await;

            if let Err(e) = self.process_batch().await {
                tracing::error!("Error processing report batch: {:?}", e);
            }
        }
    }

    /// Process a batch of pending report jobs
    async fn process_batch(&self) -> Result<()> {
        // Fetch submitted jobs that haven't been processed
        let jobs = self
            .report_job_service
            .fetch_pending(MAX_RETRIES, BATCH_SIZE)
            .await?;

        if jobs.is_empty() {
            return Ok(());
        }

        tracing::info!("Processing {} pending report jobs", jobs.len());

        for job in jobs {
            if let Err(e) = self.process_job(&job).await {
                tracing::error!("Failed to process report job {}: {:?}", job.id, e);
                self.report_job_service
                    .mark_failed(job.id, job.retry_count, MAX_RETRIES, &e.to_string())
                    .await?;
            }
        }

        Ok(())
    }

    /// Process a single report job
    async fn process_job(&self, job: &ReportJob) -> Result<()> {
        // Get the report to find adk_thread_id
        let report = self.report_service.get_by_id(job.report_id).await?;

        let adk_thread_id = report.adk_thread_id.ok_or_else(|| {
            AppError::Internal(format!("Report {} has no adk_thread_id", report.id))
        })?;

        // Check confidence score - reject low confidence reports
        let confidence = job
            .confidence_score
            .map(|d| d.to_string().parse::<f64>().unwrap_or(0.0))
            .unwrap_or(0.0);

        if confidence < MIN_CONFIDENCE_THRESHOLD {
            tracing::info!(
                "Rejecting report {} (ref: {:?}) due to low confidence: {:.2}",
                report.id,
                report.reference_number,
                confidence
            );

            self.report_service
                .reject(
                    report.id,
                    Some(&format!(
                        "Low confidence score: {:.2} (threshold: {:.2})",
                        confidence, MIN_CONFIDENCE_THRESHOLD
                    )),
                )
                .await?;

            self.report_job_service.mark_completed(job.id).await?;

            tracing::info!(
                "Report job {} completed (rejected) for report {} (ref: {:?})",
                job.id,
                report.id,
                report.reference_number
            );

            return Ok(());
        }

        tracing::info!(
            "Processing report job: {} for report: {} (ref: {:?}, confidence: {:.2})",
            job.id,
            report.id,
            report.reference_number,
            confidence
        );

        // Mark as processing
        self.report_job_service
            .update_status(job.id, ReportJobStatus::Processing)
            .await?;

        // Extract data from conversation using LLM
        let extracted = self
            .extraction_service
            .extract_from_thread(adk_thread_id)
            .await?;

        // Update report with extracted content
        self.report_service
            .update_content(
                report.id,
                &extracted.title,
                &extracted.description,
                extracted.timeline.as_deref(),
                extracted.impact.as_deref(),
            )
            .await?;

        tracing::info!("Updated report {} with extracted content", report.id);

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

        // Geocode location if provided
        if extracted.location_raw.is_some()
            || extracted.location_query.is_some()
            || extracted.location_street.is_some()
        {
            // Use structured geocoding with extracted location fields
            let geocode_result = self
                .geocoding_service
                .geocode_structured(
                    extracted.location_query.as_deref(),
                    extracted.location_street.as_deref(),
                    extracted.location_city.as_deref(),
                    extracted.location_state.as_deref(),
                )
                .await?;

            // Fall back to raw location if structured geocoding fails
            let geocode_result = match geocode_result {
                Some(r) => Some(r),
                None if extracted.location_raw.is_some() => {
                    tracing::debug!("Structured geocoding failed, falling back to location_raw");
                    self.geocoding_service
                        .geocode(extracted.location_raw.as_ref().unwrap())
                        .await?
                }
                None => None,
            };

            let raw_input = extracted
                .location_raw
                .clone()
                .or_else(|| extracted.location_query.clone())
                .unwrap_or_default();

            // Resolve region FKs before creating location
            let resolved_regions = self
                .region_lookup_service
                .resolve(
                    extracted.location_city.as_deref(),
                    extracted.location_state.as_deref(),
                )
                .await?;

            let mut create_location =
                self.geocoding_service
                    .to_create_location(report.id, raw_input, geocode_result);

            // Set region FKs
            create_location.province_id = resolved_regions.province_id;
            create_location.regency_id = resolved_regions.regency_id;
            create_location.district_id = resolved_regions.district_id;
            create_location.village_id = resolved_regions.village_id;

            let location = self
                .report_service
                .create_location(&create_location)
                .await?;
            tracing::info!(
                "Created report location: {} (province: {:?}, regency: {:?})",
                location.id,
                resolved_regions.province_id,
                resolved_regions.regency_id
            );

            // NOTE: Geographic clustering disabled - use regional hierarchy instead
            // (province_id, regency_id, district_id, village_id in report_locations)
        }

        // Copy attachments from thread to report
        if let Some(thread_id) = report.adk_thread_id {
            let attachment_count = self
                .report_service
                .copy_attachments_from_thread(report.id, thread_id)
                .await?;

            if attachment_count > 0 {
                tracing::info!(
                    "Linked {} attachments from thread {} to report {}",
                    attachment_count,
                    thread_id,
                    report.id
                );
            }
        }

        // Mark job as completed
        self.report_job_service.mark_completed(job.id).await?;

        tracing::info!(
            "Report job {} completed for report {} (ref: {:?})",
            job.id,
            report.id,
            report.reference_number
        );

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
}
