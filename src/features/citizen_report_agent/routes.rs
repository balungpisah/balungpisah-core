use std::sync::Arc;

use axum::{
    extract::DefaultBodyLimit,
    routing::{delete, get, post},
    Router,
};

use super::handlers::{
    attachment_handler::{
        count_attachments, delete_attachment, list_attachments, upload_attachment, AttachmentState,
    },
    chat_handler::{chat_stream, chat_sync, ChatState},
    conversation_handler::{get_thread, list_messages, list_threads},
};
use super::services::{AgentRuntimeService, ConversationService, ThreadAttachmentService};

/// Maximum body size for attachment uploads (21MB to account for multipart overhead)
const ATTACHMENT_BODY_LIMIT: usize = 21 * 1024 * 1024;

/// Create routes for the citizen report agent feature
pub fn routes(
    agent_runtime_service: Arc<AgentRuntimeService>,
    conversation_service: Arc<ConversationService>,
    attachment_service: Arc<ThreadAttachmentService>,
) -> Router {
    let chat_state = ChatState {
        agent_runtime: agent_runtime_service,
        attachment_service: Arc::clone(&attachment_service),
    };

    let attachment_state = AttachmentState { attachment_service };

    // Chat routes with ChatState
    let chat_routes = Router::new()
        .route("/api/citizen-report-agent/chat", post(chat_stream))
        .route("/api/citizen-report-agent/chat/sync", post(chat_sync))
        .with_state(chat_state);

    // Conversation routes with ConversationService
    let conversation_routes = Router::new()
        .route("/api/citizen-report-agent/threads", get(list_threads))
        .route("/api/citizen-report-agent/threads/{id}", get(get_thread))
        .route(
            "/api/citizen-report-agent/threads/{id}/messages",
            get(list_messages),
        )
        .with_state(conversation_service);

    // Attachment routes with AttachmentState
    let attachment_routes = Router::new()
        .route(
            "/api/citizen-report-agent/threads/{thread_id}/attachments",
            post(upload_attachment).layer(DefaultBodyLimit::max(ATTACHMENT_BODY_LIMIT)),
        )
        .route(
            "/api/citizen-report-agent/threads/{thread_id}/attachments",
            get(list_attachments),
        )
        .route(
            "/api/citizen-report-agent/threads/{thread_id}/attachments/count",
            get(count_attachments),
        )
        .route(
            "/api/citizen-report-agent/threads/{thread_id}/attachments/{attachment_id}",
            delete(delete_attachment),
        )
        .with_state(attachment_state);

    // Merge all routers
    chat_routes
        .merge(conversation_routes)
        .merge(attachment_routes)
}
