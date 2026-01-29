use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

/// File visibility enum for API requests/responses
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema, Default)]
#[serde(rename_all = "lowercase")]
pub enum FileVisibilityDto {
    /// Public files are accessible via direct URL
    #[default]
    Public,
    /// Private files require presigned URLs for access
    Private,
}

/// Upload file request DTO for OpenAPI documentation
/// Note: This struct is for Swagger UI documentation only.
/// The actual handler uses axum's Multipart extractor directly.
#[derive(Debug, ToSchema)]
#[allow(dead_code)]
pub struct UploadFileDto {
    /// The file to upload
    #[schema(format = Binary, content_media_type = "application/octet-stream")]
    pub file: String,
    /// File visibility: "public" (default) or "private"
    #[schema(example = "public")]
    pub visibility: Option<String>,
    /// Optional purpose/category for the file
    #[schema(example = "profile_picture")]
    pub purpose: Option<String>,
}

/// Response DTO for file operations
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct FileResponseDto {
    /// Unique identifier for the file
    pub id: Uuid,
    /// Original filename as uploaded
    pub original_filename: String,
    /// MIME type of the file
    pub content_type: String,
    /// Size of the file in bytes
    pub file_size: i64,
    /// URL to access the file (public URL for public files, presigned URL for private files)
    pub url: String,
    /// File visibility (public or private)
    pub visibility: FileVisibilityDto,
    /// Purpose/category of the file
    pub purpose: Option<String>,
    /// Timestamp when the file was uploaded
    pub created_at: DateTime<Utc>,
}

/// Request DTO for deleting a file by URL
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct DeleteFileByUrlDto {
    /// The URL of the file to delete
    #[validate(url(message = "Invalid URL format"))]
    #[validate(length(min = 1, message = "url is required"))]
    pub url: String,
}

/// Response DTO for delete operations
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct DeleteFileResponseDto {
    /// Confirmation that the file was deleted
    pub deleted: bool,
}

/// Allowed MIME types for file uploads
pub const ALLOWED_MIME_TYPES: &[&str] = &[
    "image/jpeg",
    "image/png",
    "image/gif",
    "image/webp",
    "application/pdf",
];

/// Maximum file size in bytes (10MB)
pub const MAX_FILE_SIZE: usize = 10 * 1024 * 1024;

/// Check if a MIME type is allowed
pub fn is_mime_type_allowed(content_type: &str) -> bool {
    ALLOWED_MIME_TYPES.contains(&content_type)
}

/// Get file extension from content type
pub fn get_extension_from_content_type(content_type: &str) -> Option<&'static str> {
    match content_type {
        "image/jpeg" => Some("jpg"),
        "image/png" => Some("png"),
        "image/gif" => Some("gif"),
        "image/webp" => Some("webp"),
        "application/pdf" => Some("pdf"),
        _ => None,
    }
}
