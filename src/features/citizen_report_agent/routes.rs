use std::sync::Arc;

use axum::{
    routing::{get, post},
    Router,
};

use super::handlers::{
    chat_handler::{chat_stream, chat_sync, ChatState},
    conversation_handler::{get_thread, list_messages, list_threads},
};
use super::services::{AgentRuntimeService, ConversationService};

/// Create routes for the citizen report agent feature
pub fn routes(
    agent_runtime_service: Arc<AgentRuntimeService>,
    conversation_service: Arc<ConversationService>,
) -> Router {
    let chat_state = ChatState {
        agent_runtime: agent_runtime_service,
    };

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

    // Merge both routers
    chat_routes.merge(conversation_routes)
}
