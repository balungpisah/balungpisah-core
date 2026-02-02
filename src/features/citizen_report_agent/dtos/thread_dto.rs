use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::shared::constants::{DEFAULT_PAGE_SIZE, MAX_PAGE_SIZE};

/// Response DTO for a conversation thread
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ThreadResponseDto {
    /// Thread ID
    pub id: Uuid,

    /// Optional thread title
    pub title: Option<String>,

    /// When the thread was created
    pub created_at: DateTime<Utc>,

    /// When the thread was last updated
    pub updated_at: DateTime<Utc>,
}

/// Response DTO for thread details (includes message count)
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ThreadDetailDto {
    /// Thread ID
    pub id: Uuid,

    /// Optional thread title
    pub title: Option<String>,

    /// Number of messages in the thread
    pub message_count: i64,

    /// When the thread was created
    pub created_at: DateTime<Utc>,

    /// When the thread was last updated
    pub updated_at: DateTime<Utc>,
}

/// Response DTO for a message in a thread
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct MessageResponseDto {
    /// Message ID
    pub id: Uuid,

    /// Thread ID this message belongs to
    pub thread_id: Uuid,

    /// Message role: "user" or "assistant"
    pub role: String,

    /// Message content - can be a string or array of content blocks (text, tool_use, tool_result)
    pub content: Value,

    /// TensorZero episode ID (for assistant messages)
    pub episode_id: Option<Uuid>,

    /// When the message was created
    pub created_at: DateTime<Utc>,
}

fn default_page() -> i64 {
    1
}

fn default_page_size() -> i64 {
    DEFAULT_PAGE_SIZE
}

/// Query parameters for listing threads
#[derive(Debug, Clone, Deserialize, IntoParams)]
pub struct ListThreadsQuery {
    /// Page number (1-indexed)
    #[serde(default = "default_page")]
    #[param(minimum = 1)]
    pub page: i64,

    /// Number of items per page (default: 10, max: 100)
    #[serde(default = "default_page_size")]
    #[param(minimum = 1, maximum = 100)]
    pub page_size: i64,
}

impl ListThreadsQuery {
    pub fn limit(&self) -> i64 {
        self.page_size.clamp(1, MAX_PAGE_SIZE)
    }

    pub fn offset(&self) -> i64 {
        (self.page.max(1) - 1) * self.limit()
    }
}

fn default_messages_page_size() -> i64 {
    50
}

const MAX_MESSAGES_PAGE_SIZE: i64 = 200;

/// Query parameters for listing messages
#[derive(Debug, Clone, Deserialize, IntoParams)]
pub struct ListMessagesQuery {
    /// Page number (1-indexed)
    #[serde(default = "default_page")]
    #[param(minimum = 1)]
    pub page: i64,

    /// Number of items per page (default: 50, max: 200)
    #[serde(default = "default_messages_page_size")]
    #[param(minimum = 1, maximum = 200)]
    pub page_size: i64,
}

impl ListMessagesQuery {
    pub fn limit(&self) -> i64 {
        self.page_size.clamp(1, MAX_MESSAGES_PAGE_SIZE)
    }

    pub fn offset(&self) -> i64 {
        (self.page.max(1) - 1) * self.limit()
    }
}
