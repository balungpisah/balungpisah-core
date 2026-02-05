use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;
use validator::Validate;

use crate::shared::constants::{DEFAULT_PAGE_SIZE, MAX_PAGE_SIZE};

// Sort direction
#[derive(Debug, Clone, Copy, Default, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum SortDirection {
    #[default]
    Desc,
    Asc,
}

impl SortDirection {
    pub fn as_sql(&self) -> &'static str {
        match self {
            SortDirection::Asc => "ASC",
            SortDirection::Desc => "DESC",
        }
    }
}

// Helper functions for defaults
fn default_page() -> i64 {
    1
}

fn default_page_size() -> i64 {
    DEFAULT_PAGE_SIZE
}

// Query params for listing prompts
#[derive(Debug, Clone, Deserialize, IntoParams, ToSchema)]
pub struct PromptQueryParams {
    /// Page number (1-indexed)
    #[serde(default = "default_page")]
    #[param(minimum = 1)]
    pub page: i64,

    /// Items per page
    #[serde(default = "default_page_size")]
    #[param(minimum = 1, maximum = 100)]
    pub page_size: i64,

    /// Search in key, name, or description
    pub search: Option<String>,

    /// Filter by active status (true = active, false = inactive, none = all)
    pub is_active: Option<bool>,

    /// Sort direction (default: desc by created_at)
    #[serde(default)]
    pub sort: SortDirection,
}

impl PromptQueryParams {
    pub fn offset(&self) -> i64 {
        (self.page.max(1) - 1) * self.limit()
    }

    pub fn limit(&self) -> i64 {
        self.page_size.clamp(1, MAX_PAGE_SIZE)
    }
}

// Create request
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CreatePromptDto {
    #[validate(length(min = 1, max = 200))]
    pub key: String,

    #[validate(length(min = 1, max = 200))]
    pub name: String,

    pub description: Option<String>,

    #[validate(length(min = 1))]
    pub template_content: String,

    pub variables: Option<serde_json::Value>,
}

// Update request
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct UpdatePromptDto {
    #[validate(length(min = 1, max = 200))]
    pub name: Option<String>,

    pub description: Option<String>,

    #[validate(length(min = 1))]
    pub template_content: Option<String>,

    pub variables: Option<serde_json::Value>,
}

// Response DTO
#[derive(Debug, Serialize, ToSchema)]
pub struct PromptResponseDto {
    pub id: Uuid,
    pub key: String,
    pub name: String,
    pub description: Option<String>,
    pub template_content: String,
    pub variables: Option<serde_json::Value>,
    pub version: i32,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<crate::features::prompts::models::Prompt> for PromptResponseDto {
    fn from(p: crate::features::prompts::models::Prompt) -> Self {
        Self {
            id: p.id,
            key: p.key,
            name: p.name,
            description: p.description,
            template_content: p.template_content,
            variables: p.variables,
            version: p.version,
            is_active: p.is_active,
            created_at: p.created_at,
            updated_at: p.updated_at,
        }
    }
}
