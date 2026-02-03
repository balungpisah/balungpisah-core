use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;

use crate::core::error::{AppError, Result};
use crate::features::reports::models::{CreateReportJob, ReportJob, ReportJobStatus};

/// Service for report job operations (background processing queue)
pub struct ReportJobService {
    pool: PgPool,
}

impl ReportJobService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Create a new report job
    pub async fn create(&self, data: &CreateReportJob) -> Result<ReportJob> {
        let confidence_decimal = data
            .confidence_score
            .map(|c| Decimal::try_from(c.clamp(0.0, 1.0)).unwrap_or_else(|_| Decimal::new(50, 2)));

        let job = sqlx::query_as!(
            ReportJob,
            r#"
            INSERT INTO report_jobs (report_id, confidence_score, status)
            VALUES ($1, $2, $3)
            RETURNING
                id, report_id, status as "status: ReportJobStatus",
                confidence_score, retry_count, error_message,
                submitted_at, processed_at, last_attempt_at, created_at
            "#,
            data.report_id,
            confidence_decimal,
            ReportJobStatus::Submitted as ReportJobStatus
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to create report job: {:?}", e);
            AppError::Database(e)
        })?;

        tracing::info!(
            "Report job created: {} for report {}",
            job.id,
            job.report_id
        );
        Ok(job)
    }

    /// Get job by report ID
    #[allow(dead_code)]
    pub async fn get_by_report_id(&self, report_id: Uuid) -> Result<Option<ReportJob>> {
        sqlx::query_as!(
            ReportJob,
            r#"
            SELECT
                id, report_id, status as "status: ReportJobStatus",
                confidence_score, retry_count, error_message,
                submitted_at, processed_at, last_attempt_at, created_at
            FROM report_jobs
            WHERE report_id = $1
            "#,
            report_id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get report job: {:?}", e);
            AppError::Database(e)
        })
    }

    /// Fetch pending jobs for processing
    pub async fn fetch_pending(&self, max_retries: i32, batch_size: i64) -> Result<Vec<ReportJob>> {
        sqlx::query_as!(
            ReportJob,
            r#"
            SELECT
                id, report_id, status as "status: ReportJobStatus",
                confidence_score, retry_count, error_message,
                submitted_at, processed_at, last_attempt_at, created_at
            FROM report_jobs
            WHERE status = 'submitted'
            AND retry_count < $1
            ORDER BY submitted_at ASC
            LIMIT $2
            "#,
            max_retries,
            batch_size
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch pending report jobs: {:?}", e);
            AppError::Database(e)
        })
    }

    /// Update job status
    pub async fn update_status(&self, job_id: Uuid, status: ReportJobStatus) -> Result<()> {
        sqlx::query!(
            r#"
            UPDATE report_jobs
            SET status = $2, last_attempt_at = NOW()
            WHERE id = $1
            "#,
            job_id,
            status as ReportJobStatus
        )
        .execute(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to update report job status: {:?}", e);
            AppError::Database(e)
        })?;

        Ok(())
    }

    /// Mark job as completed
    pub async fn mark_completed(&self, job_id: Uuid) -> Result<()> {
        sqlx::query!(
            r#"
            UPDATE report_jobs
            SET status = $2, processed_at = NOW(), last_attempt_at = NOW()
            WHERE id = $1
            "#,
            job_id,
            ReportJobStatus::Completed as ReportJobStatus
        )
        .execute(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to mark report job as completed: {:?}", e);
            AppError::Database(e)
        })?;

        tracing::info!("Report job {} marked as completed", job_id);
        Ok(())
    }

    /// Mark job as failed with error message
    pub async fn mark_failed(
        &self,
        job_id: Uuid,
        current_retry_count: i32,
        max_retries: i32,
        error_message: &str,
    ) -> Result<()> {
        let new_retry_count = current_retry_count + 1;
        let new_status = if new_retry_count >= max_retries {
            ReportJobStatus::Failed
        } else {
            ReportJobStatus::Submitted // Keep as submitted for retry
        };

        sqlx::query!(
            r#"
            UPDATE report_jobs
            SET status = $2, error_message = $3, retry_count = $4, last_attempt_at = NOW()
            WHERE id = $1
            "#,
            job_id,
            new_status as ReportJobStatus,
            error_message,
            new_retry_count
        )
        .execute(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to mark report job as failed: {:?}", e);
            AppError::Database(e)
        })?;

        if new_retry_count >= max_retries {
            tracing::warn!(
                "Report job {} permanently failed after {} retries",
                job_id,
                max_retries
            );
        } else {
            tracing::info!(
                "Report job {} marked for retry ({}/{})",
                job_id,
                new_retry_count,
                max_retries
            );
        }

        Ok(())
    }

    /// Count reports created today by user (for rate limiting)
    #[allow(dead_code)]
    pub async fn count_user_reports_today(&self, user_id: &str) -> Result<i64> {
        // We count from reports table directly since that's where user_id is stored
        let count = sqlx::query_scalar!(
            r#"
            SELECT COUNT(*) as "count!"
            FROM reports
            WHERE user_id = $1
            AND created_at >= (CURRENT_DATE AT TIME ZONE 'Asia/Jakarta')
            AND created_at < ((CURRENT_DATE + INTERVAL '1 day') AT TIME ZONE 'Asia/Jakarta')
            "#,
            user_id
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to count user reports today: {:?}", e);
            AppError::Database(e)
        })?;

        Ok(count)
    }
}
