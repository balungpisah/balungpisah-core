use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use crate::shared::constants::{DEFAULT_PAGE_SIZE, MAX_PAGE_SIZE};

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub message: Option<String>,
    pub meta: Option<Meta>,
    pub errors: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct Meta {
    pub total: i64,
}

// =============================================================================
// PAGINATION
// =============================================================================

/// Standard pagination query parameters for all list endpoints.
/// This is a shared struct that can be embedded or used directly in handlers.
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize, IntoParams)]
pub struct PaginationQuery {
    /// Page number (1-indexed, default: 1)
    #[serde(default = "default_page")]
    #[param(minimum = 1)]
    pub page: i64,

    /// Number of items per page (default: 10, max: 100)
    #[serde(default = "default_page_size")]
    #[param(minimum = 1, maximum = 100)]
    pub page_size: i64,
}

fn default_page() -> i64 {
    1
}

fn default_page_size() -> i64 {
    DEFAULT_PAGE_SIZE
}

impl Default for PaginationQuery {
    fn default() -> Self {
        Self {
            page: 1,
            page_size: DEFAULT_PAGE_SIZE,
        }
    }
}

#[allow(dead_code)]
impl PaginationQuery {
    /// Calculate SQL OFFSET from page number
    pub fn offset(&self) -> i64 {
        (self.page.max(1) - 1) * self.limit()
    }

    /// Get clamped page_size (respects MAX_PAGE_SIZE)
    pub fn limit(&self) -> i64 {
        self.page_size.clamp(1, MAX_PAGE_SIZE)
    }
}

impl<T> ApiResponse<T> {
    pub fn success(data: Option<T>, message: Option<String>, meta: Option<Meta>) -> Self {
        Self {
            success: true,
            data,
            message,
            meta,
            errors: None,
        }
    }

    pub fn error(message: Option<String>, errors: Option<Vec<String>>) -> ApiResponse<()> {
        ApiResponse {
            success: false,
            data: None,
            message,
            meta: None,
            errors,
        }
    }
}
