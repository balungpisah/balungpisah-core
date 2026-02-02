use std::sync::Arc;

use axum::{
    extract::State,
    response::{sse::Event, IntoResponse, Response, Sse},
    Json,
};
use balungpisah_adk::{ContentBlock, MessageContent};
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::StreamExt;
use tracing::debug;
use validator::Validate;

use crate::core::error::{AppError, Result};
use crate::features::auth::model::AuthenticatedUser;
use crate::shared::types::ApiResponse;

use super::super::dtos::{ChatRequestDto, ChatResponseDto, ContentBlockInput, MessageContentInput};
use super::super::services::{AgentRuntimeService, ThreadAttachmentService};

/// State for chat handlers
#[derive(Clone)]
pub struct ChatState {
    pub agent_runtime: Arc<AgentRuntimeService>,
    pub attachment_service: Arc<ThreadAttachmentService>,
}

/// Convert MessageContentInput (DTO) to MessageContent (ADK)
fn convert_content(input: MessageContentInput) -> MessageContent {
    match input {
        MessageContentInput::Text(text) => MessageContent::Text(text),
        MessageContentInput::Blocks(blocks) => {
            let content_blocks: Vec<ContentBlock> = blocks
                .into_iter()
                .map(|b| match b {
                    ContentBlockInput::Text { text } => ContentBlock::text(text),
                    ContentBlockInput::File { url } => ContentBlock::file_from_url(url, None),
                    ContentBlockInput::FileData { mime_type, data } => {
                        ContentBlock::file_from_base64(data, mime_type)
                    }
                })
                .collect();
            MessageContent::Blocks(content_blocks)
        }
    }
}

/// Parse a raw SSE string into event type and data.
///
/// Input format:
/// ```text
/// event: block.delta
/// data: {"message_id":"msg_..."}
///
/// ```
///
/// Returns (event_type, data)
fn parse_raw_sse(raw: &str) -> (Option<String>, String) {
    let mut event_type = None;
    let mut data = String::new();

    for line in raw.lines() {
        if let Some(event) = line.strip_prefix("event: ") {
            event_type = Some(event.trim().to_string());
        } else if let Some(d) = line.strip_prefix("data: ") {
            data = d.trim().to_string();
        }
    }

    (event_type, data)
}

/// Send a message and receive a streaming SSE response
#[utoipa::path(
    post,
    path = "/api/citizen-report-agent/chat",
    request_body = ChatRequestDto,
    responses(
        (status = 200, description = "SSE stream of chat events", content_type = "text/event-stream"),
        (status = 400, description = "Validation error"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - Thread belongs to another user"),
        (status = 404, description = "Thread not found")
    ),
    tag = "citizen-report-agent",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn chat_stream(
    user: AuthenticatedUser,
    State(state): State<ChatState>,
    Json(dto): Json<ChatRequestDto>,
) -> Result<Response> {
    // Validate request
    dto.validate()
        .map_err(|e| AppError::Validation(format!("Invalid request: {}", e)))?;

    // Convert DTO content to ADK MessageContent
    let content = convert_content(dto.content);

    // Fetch attachment context if thread_id is provided
    let attachment_context = if let Some(tid) = dto.thread_id {
        match state
            .attachment_service
            .get_attachment_context(tid, &user.account_id)
            .await
        {
            Ok(ctx) => {
                if ctx.is_some() {
                    debug!("Injecting attachment context for thread {}", tid);
                }
                ctx
            }
            Err(e) => {
                // Log but don't fail - attachment context is optional
                debug!("Failed to fetch attachment context: {}", e);
                None
            }
        }
    } else {
        None
    };

    // Start streaming chat - returns raw SSE strings from ADK
    let (_thread_id, rx) = state
        .agent_runtime
        .chat_stream(
            &user.account_id,
            dto.thread_id,
            dto.user_message_id,
            content,
            attachment_context.as_deref(),
        )
        .await?;

    // Convert receiver to SSE stream
    // Each event is a raw SSE string like "event: block.delta\ndata: {...}\n\n"
    let stream = ReceiverStream::new(rx).map(|raw_sse| {
        let (event_type, data) = parse_raw_sse(&raw_sse);

        let mut event = Event::default().data(data);
        if let Some(et) = event_type {
            event = event.event(et);
        }

        Ok::<_, std::convert::Infallible>(event)
    });

    // Return SSE response with keepalive
    let sse = Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(std::time::Duration::from_secs(15))
            .text("ping"),
    );

    Ok(sse.into_response())
}

/// Send a message and receive a synchronous response (non-streaming fallback)
#[utoipa::path(
    post,
    path = "/api/citizen-report-agent/chat/sync",
    request_body = ChatRequestDto,
    responses(
        (status = 200, description = "Chat response", body = ApiResponse<ChatResponseDto>),
        (status = 400, description = "Validation error"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - Thread belongs to another user"),
        (status = 404, description = "Thread not found"),
        (status = 502, description = "AI service error")
    ),
    tag = "citizen-report-agent",
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn chat_sync(
    user: AuthenticatedUser,
    State(state): State<ChatState>,
    Json(dto): Json<ChatRequestDto>,
) -> Result<Json<ApiResponse<ChatResponseDto>>> {
    // Validate request
    dto.validate()
        .map_err(|e| AppError::Validation(format!("Invalid request: {}", e)))?;

    // Convert DTO content to ADK MessageContent
    let content = convert_content(dto.content);

    // Fetch attachment context if thread_id is provided
    let attachment_context = if let Some(tid) = dto.thread_id {
        match state
            .attachment_service
            .get_attachment_context(tid, &user.account_id)
            .await
        {
            Ok(ctx) => {
                if ctx.is_some() {
                    debug!("Injecting attachment context for thread {}", tid);
                }
                ctx
            }
            Err(e) => {
                // Log but don't fail - attachment context is optional
                debug!("Failed to fetch attachment context: {}", e);
                None
            }
        }
    } else {
        None
    };

    // Send chat message
    let (thread_id, response, episode_id) = state
        .agent_runtime
        .chat_sync(
            &user.account_id,
            dto.thread_id,
            content,
            attachment_context.as_deref(),
        )
        .await?;

    let response_dto = ChatResponseDto {
        thread_id,
        response,
        episode_id,
    };

    Ok(Json(ApiResponse::success(Some(response_dto), None, None)))
}
