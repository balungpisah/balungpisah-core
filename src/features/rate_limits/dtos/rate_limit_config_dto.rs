use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

use crate::features::rate_limits::models::RateLimitConfig;

/// Response DTO for rate limit configuration
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct RateLimitConfigResponseDto {
    pub key: String,
    pub value: i32,
    pub description: Option<String>,
    pub updated_at: DateTime<Utc>,
}

impl From<RateLimitConfig> for RateLimitConfigResponseDto {
    fn from(config: RateLimitConfig) -> Self {
        Self {
            key: config.key,
            value: config.value,
            description: config.description,
            updated_at: config.updated_at,
        }
    }
}

/// Request DTO for updating rate limit configuration
#[derive(Debug, Clone, Deserialize, Validate, ToSchema)]
pub struct UpdateRateLimitConfigDto {
    #[validate(range(min = 1, message = "Value must be at least 1"))]
    pub value: i32,
}

/// Response DTO for user's rate limit status
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct UserRateLimitStatusDto {
    /// Number of reports the user has created today
    pub reports_used: i64,
    /// Number of reports remaining before hitting the limit
    pub reports_remaining: i64,
    /// Maximum reports allowed per day
    pub max_reports: i64,
    /// Whether the user can still chat (hasn't reached the limit)
    pub can_chat: bool,
    /// When the limit resets (next 00:00 WIB in UTC)
    pub resets_at: DateTime<Utc>,
}
