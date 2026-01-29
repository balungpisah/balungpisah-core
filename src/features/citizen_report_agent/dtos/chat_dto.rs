use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

/// Request DTO for sending a chat message
#[derive(Debug, Clone, Deserialize, Validate, ToSchema)]
pub struct ChatRequestDto {
    /// Optional thread ID.
    /// - If not provided, a new thread will be created.
    /// - If provided but not found, the thread will be created with this ID (optimistic UI).
    /// - If provided and found, the existing thread will be used.
    pub thread_id: Option<Uuid>,

    /// Optional user message ID for optimistic UI or edit mode.
    /// - If not provided, a new message ID will be auto-generated.
    /// - If provided but not found, the message will be created with this ID (optimistic UI).
    /// - If provided and found, edit mode is triggered: the message is updated and
    ///   all subsequent messages in the thread are deleted before generating a new response.
    pub user_message_id: Option<Uuid>,

    /// The user's message (1-10000 characters)
    #[validate(length(
        min = 1,
        max = 10000,
        message = "Message must be between 1 and 10000 characters"
    ))]
    pub message: String,
}

/// Response DTO for synchronous chat
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ChatResponseDto {
    /// The thread ID (useful if a new thread was created)
    pub thread_id: Uuid,

    /// The assistant's response text
    pub response: String,

    /// TensorZero episode ID for tracking
    pub episode_id: Uuid,
}

// Note: SSE events are now directly forwarded from ADK as raw SSE strings.
// The ADK emits events like:
// - message.started
// - block.created
// - block.delta (with text/thought/tool_call data)
// - block.completed
// - message.usage
// - message.completed
// - error
//
// Each event follows the format:
// event: <event_type>
// data: <json_payload>
