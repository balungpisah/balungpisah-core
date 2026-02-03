use std::sync::Arc;

use chrono::{DateTime, Datelike, FixedOffset, TimeZone, Utc};
use sqlx::PgPool;

use crate::core::error::{AppError, Result};
use crate::features::rate_limits::dtos::UserRateLimitStatusDto;
use crate::features::rate_limits::services::RateLimitConfigService;

/// Service for checking and enforcing rate limits
pub struct RateLimitService {
    pool: PgPool,
    config_service: Arc<RateLimitConfigService>,
}

impl RateLimitService {
    pub fn new(pool: PgPool, config_service: Arc<RateLimitConfigService>) -> Self {
        Self {
            pool,
            config_service,
        }
    }

    /// Get the start and end of today in WIB (UTC+7), converted to UTC
    fn get_wib_day_bounds() -> (DateTime<Utc>, DateTime<Utc>) {
        // WIB is UTC+7
        let wib = FixedOffset::east_opt(7 * 3600).expect("Invalid WIB offset");
        let now_wib = Utc::now().with_timezone(&wib);

        // Start of today in WIB (00:00:00)
        let start_of_day_wib = wib
            .with_ymd_and_hms(now_wib.year(), now_wib.month(), now_wib.day(), 0, 0, 0)
            .single()
            .expect("Invalid WIB date");

        // Convert to UTC
        let start_utc = start_of_day_wib.with_timezone(&Utc);

        // End of today (start of tomorrow) in WIB
        let end_of_day_wib = start_of_day_wib + chrono::Duration::days(1);
        let end_utc = end_of_day_wib.with_timezone(&Utc);

        (start_utc, end_utc)
    }

    /// Get the next reset time (00:00 WIB tomorrow) in UTC
    fn get_next_reset_time() -> DateTime<Utc> {
        let (_start, end) = Self::get_wib_day_bounds();
        end
    }

    /// Count reports created by user today (in WIB timezone)
    pub async fn count_user_reports_today(&self, user_id: &str) -> Result<i64> {
        let (start_utc, end_utc) = Self::get_wib_day_bounds();

        let count: i64 = sqlx::query_scalar!(
            r#"
            SELECT COUNT(*) as "count!"
            FROM reports
            WHERE user_id = $1
              AND created_at >= $2
              AND created_at < $3
            "#,
            user_id,
            start_utc,
            end_utc
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to count user reports today: {:?}", e);
            AppError::Database(e)
        })?;

        Ok(count)
    }

    /// Check if user can chat (has not reached daily report limit)
    pub async fn can_user_chat(&self, user_id: &str) -> Result<bool> {
        let limit = self.config_service.get_daily_ticket_limit().await?;
        let count = self.count_user_reports_today(user_id).await?;

        Ok(count < limit as i64)
    }

    /// Get user's rate limit status
    pub async fn get_user_status(&self, user_id: &str) -> Result<UserRateLimitStatusDto> {
        let limit = self.config_service.get_daily_ticket_limit().await?;
        let reports_used = self.count_user_reports_today(user_id).await?;
        let max_reports = limit as i64;
        let reports_remaining = (max_reports - reports_used).max(0);
        let can_chat = reports_used < max_reports;
        let resets_at = Self::get_next_reset_time();

        Ok(UserRateLimitStatusDto {
            reports_used,
            reports_remaining,
            max_reports,
            can_chat,
            resets_at,
        })
    }
}
