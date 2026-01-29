use axum::{
    extract::{Multipart, State},
    http::StatusCode,
    Json,
};
use std::sync::Arc;
use tracing::debug;
use validator::Validate;

use crate::core::error::AppError;
use crate::features::auth::model::AuthenticatedUser;
use crate::features::files::dtos::{
    is_mime_type_allowed, DeleteFileByUrlDto, DeleteFileResponseDto, FileResponseDto,
    FileVisibilityDto, UploadFileDto, ALLOWED_MIME_TYPES, MAX_FILE_SIZE,
};
use crate::features::files::services::FileService;
use crate::shared::types::ApiResponse;

/// Upload a file
///
/// Accepts multipart/form-data with:
/// - `file`: The file to upload (required)
/// - `visibility`: "public" or "private" (optional, defaults to "public")
/// - `purpose`: Optional purpose/category for the file
#[utoipa::path(
    post,
    path = "/api/files/upload",
    tag = "files",
    request_body(
        content = UploadFileDto,
        content_type = "multipart/form-data",
        description = "File upload form with optional visibility (public/private) and purpose fields",
    ),
    responses(
        (status = 201, description = "File uploaded successfully", body = ApiResponse<FileResponseDto>),
        (status = 400, description = "Invalid file or validation error"),
        (status = 401, description = "Authentication required"),
        (status = 413, description = "File too large")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn upload_file(
    user: AuthenticatedUser,
    State(service): State<Arc<FileService>>,
    mut multipart: Multipart,
) -> Result<(StatusCode, Json<ApiResponse<FileResponseDto>>), AppError> {
    let mut file_data: Option<Vec<u8>> = None;
    let mut file_name: Option<String> = None;
    let mut content_type: Option<String> = None;
    let mut visibility = FileVisibilityDto::Public; // Default to public
    let mut purpose: Option<String> = None;

    // Process multipart fields
    while let Some(field) = multipart.next_field().await.map_err(|e| {
        debug!("Failed to read multipart field: {}", e);
        AppError::BadRequest(format!("Failed to read multipart data: {}", e))
    })? {
        let field_name = field.name().unwrap_or("").to_string();

        match field_name.as_str() {
            "file" => {
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
            }
            "visibility" => {
                let text = field.text().await.map_err(|e| {
                    AppError::BadRequest(format!("Failed to read visibility field: {}", e))
                })?;
                visibility = match text.to_lowercase().as_str() {
                    "private" => FileVisibilityDto::Private,
                    _ => FileVisibilityDto::Public,
                };
            }
            "purpose" => {
                let text = field.text().await.map_err(|e| {
                    AppError::BadRequest(format!("Failed to read purpose field: {}", e))
                })?;
                if !text.is_empty() {
                    purpose = Some(text);
                }
            }
            _ => {
                // Ignore unknown fields
                debug!("Ignoring unknown field: {}", field_name);
            }
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
    if file_data.len() > MAX_FILE_SIZE {
        return Err(AppError::BadRequest(format!(
            "File too large. Maximum size is {} bytes ({} MB)",
            MAX_FILE_SIZE,
            MAX_FILE_SIZE / 1024 / 1024
        )));
    }

    // Validate MIME type
    if !is_mime_type_allowed(&content_type) {
        return Err(AppError::BadRequest(format!(
            "File type '{}' is not allowed. Allowed types: {}",
            content_type,
            ALLOWED_MIME_TYPES.join(", ")
        )));
    }

    // Upload file
    let response = service
        .upload_file(
            file_data,
            &file_name,
            &content_type,
            visibility,
            purpose,
            &user.sub,
        )
        .await?;

    Ok((
        StatusCode::CREATED,
        Json(ApiResponse::success(Some(response), None, None)),
    ))
}

/// Delete a file by its URL
///
/// Only the owner of the file can delete it.
#[utoipa::path(
    delete,
    path = "/api/files",
    tag = "files",
    request_body = DeleteFileByUrlDto,
    responses(
        (status = 200, description = "File deleted successfully", body = ApiResponse<DeleteFileResponseDto>),
        (status = 400, description = "Invalid URL"),
        (status = 401, description = "Authentication required"),
        (status = 403, description = "Not authorized to delete this file"),
        (status = 404, description = "File not found")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
pub async fn delete_file_by_url(
    user: AuthenticatedUser,
    State(service): State<Arc<FileService>>,
    Json(dto): Json<DeleteFileByUrlDto>,
) -> Result<Json<ApiResponse<DeleteFileResponseDto>>, AppError> {
    // Validate DTO
    dto.validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    // Delete file
    service.delete_by_url(&dto.url, &user.sub).await?;

    Ok(Json(ApiResponse::success(
        Some(DeleteFileResponseDto { deleted: true }),
        Some("File deleted successfully".to_string()),
        None,
    )))
}
