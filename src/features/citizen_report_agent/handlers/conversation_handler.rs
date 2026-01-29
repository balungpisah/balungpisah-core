use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    Json,
};
use uuid::Uuid;

use crate::core::error::Result;
use crate::features::auth::model::AuthenticatedUser;
use crate::shared::types::{ApiResponse, Meta};

use super::super::dtos::{
    ListMessagesQuery, ListThreadsQuery, MessageResponseDto, ThreadDetailDto, ThreadResponseDto,
};
use super::super::services::ConversationService;

/// GET /api/citizen-report-agent/threads
/// List user's conversation threads
#[utoipa::path(
    get,
    path = "/api/citizen-report-agent/threads",
    params(ListThreadsQuery),
    responses(
        (status = 200, description = "List of threads", body = ApiResponse<Vec<ThreadResponseDto>>),
        (status = 401, description = "Unauthorized")
    ),
    tag = "citizen-report-agent",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn list_threads(
    user: AuthenticatedUser,
    State(service): State<Arc<ConversationService>>,
    Query(query): Query<ListThreadsQuery>,
) -> Result<Json<ApiResponse<Vec<ThreadResponseDto>>>> {
    let (threads, total) = service.list_threads(&user.account_id, &query).await?;

    Ok(Json(ApiResponse::success(
        Some(threads),
        None,
        Some(Meta { total }),
    )))
}

/// GET /api/citizen-report-agent/threads/{id}
/// Get thread details
#[utoipa::path(
    get,
    path = "/api/citizen-report-agent/threads/{id}",
    params(
        ("id" = Uuid, Path, description = "Thread ID")
    ),
    responses(
        (status = 200, description = "Thread details", body = ApiResponse<ThreadDetailDto>),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - Thread belongs to another user"),
        (status = 404, description = "Thread not found")
    ),
    tag = "citizen-report-agent",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn get_thread(
    user: AuthenticatedUser,
    State(service): State<Arc<ConversationService>>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<ThreadDetailDto>>> {
    let thread = service.get_thread(&user.account_id, id).await?;

    Ok(Json(ApiResponse::success(Some(thread), None, None)))
}

/// GET /api/citizen-report-agent/threads/{id}/messages
/// List messages in a thread
#[utoipa::path(
    get,
    path = "/api/citizen-report-agent/threads/{id}/messages",
    params(
        ("id" = Uuid, Path, description = "Thread ID"),
        ListMessagesQuery
    ),
    responses(
        (status = 200, description = "List of messages", body = ApiResponse<Vec<MessageResponseDto>>),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - Thread belongs to another user"),
        (status = 404, description = "Thread not found")
    ),
    tag = "citizen-report-agent",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn list_messages(
    user: AuthenticatedUser,
    State(service): State<Arc<ConversationService>>,
    Path(id): Path<Uuid>,
    Query(query): Query<ListMessagesQuery>,
) -> Result<Json<ApiResponse<Vec<MessageResponseDto>>>> {
    let (messages, total) = service.list_messages(&user.account_id, id, &query).await?;

    Ok(Json(ApiResponse::success(
        Some(messages),
        None,
        Some(Meta { total }),
    )))
}
