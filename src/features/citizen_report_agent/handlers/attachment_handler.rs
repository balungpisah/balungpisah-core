use std::sync::Arc;

use axum::{
    extract::{Multipart, Path, State},
    http::StatusCode,
    Json,
};
use tracing::debug;
use uuid::Uuid;

use crate::core::error::AppError;
use crate::features::auth::model::AuthenticatedUser;
use crate::features::citizen_report_agent::dtos::{
    is_attachment_mime_type_allowed, AttachmentCountDto, DeleteAttachmentResponseDto,
    ThreadAttachmentResponseDto, UploadAttachmentDto, ALLOWED_ATTACHMENT_MIME_TYPES,
    MAX_ATTACHMENT_SIZE,
};
use crate::features::citizen_report_agent::services::ThreadAttachmentService;
use crate::shared::types::ApiResponse;

/// State for attachment handlers
#[derive(Clone)]
pub struct AttachmentState {
    pub attachment_service: Arc<ThreadAttachmentService>,
}

/// Upload an attachment to a thread
#[utoipa::path(
    post,
    path = "/api/citizen-report-agent/threads/{thread_id}/attachments",
    tag = "citizen-report-agent",
    params(
        ("thread_id" = Uuid, Path, description = "Thread ID")
    ),
    request_body(
        content = UploadAttachmentDto,
        content_type = "multipart/form-data",
        description = "File to attach to the thread"
    ),
    responses(
        (status = 201, description = "Attachment uploaded successfully", body = ApiResponse<ThreadAttachmentResponseDto>),
        (status = 400, description = "Invalid file, validation error, or max attachments reached"),
        (status = 401, description = "Authentication required"),
        (status = 403, description = "Thread does not belong to user"),
        (status = 404, description = "Thread not found"),
        (status = 413, description = "File too large")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn upload_attachment(
    user: AuthenticatedUser,
    Path(thread_id): Path<Uuid>,
    State(state): State<AttachmentState>,
    mut multipart: Multipart,
) -> Result<(StatusCode, Json<ApiResponse<ThreadAttachmentResponseDto>>), AppError> {
    let mut file_data: Option<Vec<u8>> = None;
    let mut file_name: Option<String> = None;
    let mut content_type: Option<String> = None;

    // Process multipart fields
    while let Some(field) = multipart.next_field().await.map_err(|e| {
        debug!("Failed to read multipart field: {}", e);
        AppError::BadRequest(format!("Failed to read multipart data: {}", e))
    })? {
        let field_name = field.name().unwrap_or("").to_string();

        if field_name == "file" {
            // Get content type
            let ct = field
                .content_type()
                .map(|s| s.to_string())
                .unwrap_or_else(|| "application/octet-stream".to_string());

            // Get filename
            let fname = field
                .file_name()
                .map(|s| s.to_string())
                .unwrap_or_else(|| "unnamed".to_string());

            // Read file data
            let data = field.bytes().await.map_err(|e| {
                debug!("Failed to read file bytes: {}", e);
                AppError::BadRequest(format!("Failed to read file data: {}", e))
            })?;

            file_data = Some(data.to_vec());
            file_name = Some(fname);
            content_type = Some(ct);
        } else {
            debug!("Ignoring unknown field: {}", field_name);
        }
    }

    // Validate required fields
    let file_data =
        file_data.ok_or_else(|| AppError::BadRequest("File is required".to_string()))?;
    let file_name =
        file_name.ok_or_else(|| AppError::BadRequest("Filename is required".to_string()))?;
    let content_type =
        content_type.ok_or_else(|| AppError::BadRequest("Content type is required".to_string()))?;

    // Validate file size
    if file_data.len() > MAX_ATTACHMENT_SIZE {
        return Err(AppError::BadRequest(format!(
            "File too large. Maximum size is {} bytes ({} MB)",
            MAX_ATTACHMENT_SIZE,
            MAX_ATTACHMENT_SIZE / 1024 / 1024
        )));
    }

    // Validate MIME type
    if !is_attachment_mime_type_allowed(&content_type) {
        return Err(AppError::BadRequest(format!(
            "File type '{}' is not allowed. Allowed types: images (image/*), PDF (application/pdf), video (video/*). Examples: {}",
            content_type,
            ALLOWED_ATTACHMENT_MIME_TYPES.iter().take(5).cloned().collect::<Vec<_>>().join(", ")
        )));
    }

    // Upload attachment
    let response = state
        .attachment_service
        .upload_attachment(
            thread_id,
            &user.account_id,
            file_data,
            &file_name,
            &content_type,
        )
        .await?;

    Ok((
        StatusCode::CREATED,
        Json(ApiResponse::success(Some(response), None, None)),
    ))
}

/// List all attachments for a thread
#[utoipa::path(
    get,
    path = "/api/citizen-report-agent/threads/{thread_id}/attachments",
    tag = "citizen-report-agent",
    params(
        ("thread_id" = Uuid, Path, description = "Thread ID")
    ),
    responses(
        (status = 200, description = "List of attachments", body = ApiResponse<Vec<ThreadAttachmentResponseDto>>),
        (status = 401, description = "Authentication required"),
        (status = 403, description = "Thread does not belong to user"),
        (status = 404, description = "Thread not found")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn list_attachments(
    user: AuthenticatedUser,
    Path(thread_id): Path<Uuid>,
    State(state): State<AttachmentState>,
) -> Result<Json<ApiResponse<Vec<ThreadAttachmentResponseDto>>>, AppError> {
    let attachments = state
        .attachment_service
        .list_attachments(thread_id, &user.account_id)
        .await?;

    Ok(Json(ApiResponse::success(Some(attachments), None, None)))
}

/// Get attachment count for a thread
#[utoipa::path(
    get,
    path = "/api/citizen-report-agent/threads/{thread_id}/attachments/count",
    tag = "citizen-report-agent",
    params(
        ("thread_id" = Uuid, Path, description = "Thread ID")
    ),
    responses(
        (status = 200, description = "Attachment count info", body = ApiResponse<AttachmentCountDto>),
        (status = 401, description = "Authentication required"),
        (status = 403, description = "Thread does not belong to user"),
        (status = 404, description = "Thread not found")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn count_attachments(
    user: AuthenticatedUser,
    Path(thread_id): Path<Uuid>,
    State(state): State<AttachmentState>,
) -> Result<Json<ApiResponse<AttachmentCountDto>>, AppError> {
    let count = state
        .attachment_service
        .count_attachments(thread_id, &user.account_id)
        .await?;

    Ok(Json(ApiResponse::success(Some(count), None, None)))
}

/// Delete an attachment from a thread
#[utoipa::path(
    delete,
    path = "/api/citizen-report-agent/threads/{thread_id}/attachments/{attachment_id}",
    tag = "citizen-report-agent",
    params(
        ("thread_id" = Uuid, Path, description = "Thread ID"),
        ("attachment_id" = Uuid, Path, description = "Attachment ID")
    ),
    responses(
        (status = 200, description = "Attachment deleted", body = ApiResponse<DeleteAttachmentResponseDto>),
        (status = 401, description = "Authentication required"),
        (status = 403, description = "Thread does not belong to user"),
        (status = 404, description = "Thread or attachment not found")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn delete_attachment(
    user: AuthenticatedUser,
    Path((thread_id, attachment_id)): Path<(Uuid, Uuid)>,
    State(state): State<AttachmentState>,
) -> Result<Json<ApiResponse<DeleteAttachmentResponseDto>>, AppError> {
    state
        .attachment_service
        .delete_attachment(thread_id, attachment_id, &user.account_id)
        .await?;

    Ok(Json(ApiResponse::success(
        Some(DeleteAttachmentResponseDto { deleted: true }),
        Some("Attachment deleted successfully".to_string()),
        None,
    )))
}
