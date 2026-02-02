use sqlx::PgPool;

use crate::core::error::{AppError, Result};
use crate::features::rate_limits::models::RateLimitConfig;

/// Key for the daily ticket limit configuration
pub const DAILY_TICKET_LIMIT_KEY: &str = "daily_ticket_limit";

/// Service for managing rate limit configurations
pub struct RateLimitConfigService {
    pool: PgPool,
}

impl RateLimitConfigService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Get all rate limit configurations
    pub async fn list_all(&self) -> Result<Vec<RateLimitConfig>> {
        let configs = sqlx::query_as!(
            RateLimitConfig,
            r#"
            SELECT id, key, value, description, updated_at, updated_by
            FROM rate_limit_configs
            ORDER BY key
            "#
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to list rate limit configs: {:?}", e);
            AppError::Database(e)
        })?;

        Ok(configs)
    }

    /// Get a rate limit configuration by key
    pub async fn get_config(&self, key: &str) -> Result<RateLimitConfig> {
        let config = sqlx::query_as!(
            RateLimitConfig,
            r#"
            SELECT id, key, value, description, updated_at, updated_by
            FROM rate_limit_configs
            WHERE key = $1
            "#,
            key
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get rate limit config: {:?}", e);
            AppError::Database(e)
        })?;

        config.ok_or_else(|| AppError::NotFound(format!("Rate limit config '{}' not found", key)))
    }

    /// Get the daily ticket limit value
    pub async fn get_daily_ticket_limit(&self) -> Result<i32> {
        let config = self.get_config(DAILY_TICKET_LIMIT_KEY).await?;
        Ok(config.value)
    }

    /// Update a rate limit configuration
    pub async fn update_config(
        &self,
        key: &str,
        value: i32,
        updated_by: &str,
    ) -> Result<RateLimitConfig> {
        let config = sqlx::query_as!(
            RateLimitConfig,
            r#"
            UPDATE rate_limit_configs
            SET value = $1, updated_at = NOW(), updated_by = $2
            WHERE key = $3
            RETURNING id, key, value, description, updated_at, updated_by
            "#,
            value,
            updated_by,
            key
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to update rate limit config: {:?}", e);
            AppError::Database(e)
        })?;

        config.ok_or_else(|| AppError::NotFound(format!("Rate limit config '{}' not found", key)))
    }
}
