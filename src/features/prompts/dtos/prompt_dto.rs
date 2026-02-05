use chrono::{DateTime, Utc};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;
use validator::Validate;

/// Regex for prompt key format: snake_case segments separated by `/`
/// Examples: "citizen_report_agent/system", "extraction/system"
/// No .jinja extension, no leading/trailing slashes, no uppercase
static PROMPT_KEY_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[a-z][a-z0-9_]*(/[a-z][a-z0-9_]*)+$").unwrap());

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
    /// Prompt key in format: `agent_name/prompt_type` (snake_case, no .jinja extension)
    #[validate(length(min = 3, max = 200), regex(path = *PROMPT_KEY_REGEX, message = "key must be in snake_case format with at least two segments separated by '/' (e.g., 'citizen_report_agent/system'). No .jinja extension, no uppercase, no leading/trailing slashes."))]
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
