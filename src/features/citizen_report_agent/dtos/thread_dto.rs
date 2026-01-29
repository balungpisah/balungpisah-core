use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

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

    /// Message content (text)
    pub content: String,

    /// TensorZero episode ID (for assistant messages)
    pub episode_id: Option<Uuid>,

    /// When the message was created
    pub created_at: DateTime<Utc>,
}

/// Query parameters for listing threads
#[derive(Debug, Clone, Deserialize, IntoParams)]
pub struct ListThreadsQuery {
    /// Maximum number of threads to return (default: 20, max: 100)
    #[param(minimum = 1, maximum = 100)]
    pub limit: Option<i64>,

    /// Number of threads to skip (default: 0)
    #[param(minimum = 0)]
    pub offset: Option<i64>,
}

impl ListThreadsQuery {
    pub fn limit(&self) -> i64 {
        self.limit.unwrap_or(20).clamp(1, 100)
    }

    pub fn offset(&self) -> i64 {
        self.offset.unwrap_or(0).max(0)
    }
}

/// Query parameters for listing messages
#[derive(Debug, Clone, Deserialize, IntoParams)]
pub struct ListMessagesQuery {
    /// Maximum number of messages to return (default: 50, max: 200)
    #[param(minimum = 1, maximum = 200)]
    pub limit: Option<i64>,

    /// Number of messages to skip (default: 0)
    #[param(minimum = 0)]
    pub offset: Option<i64>,
}

impl ListMessagesQuery {
    pub fn limit(&self) -> i64 {
        self.limit.unwrap_or(50).clamp(1, 200)
    }

    pub fn offset(&self) -> i64 {
        self.offset.unwrap_or(0).max(0)
    }
}
