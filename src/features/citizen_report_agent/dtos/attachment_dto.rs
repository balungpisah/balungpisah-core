use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// Maximum number of attachments allowed per thread
pub const MAX_ATTACHMENTS_PER_THREAD: i64 = 5;

/// Maximum file size in bytes (20MB)
pub const MAX_ATTACHMENT_SIZE: usize = 20 * 1024 * 1024;

/// Allowed MIME types for thread attachments
pub const ALLOWED_ATTACHMENT_MIME_TYPES: &[&str] = &[
    // Images
    "image/jpeg",
    "image/png",
    "image/gif",
    "image/webp",
    "image/heic",
    "image/heif",
    // PDF
    "application/pdf",
    // Video
    "video/mp4",
    "video/mpeg",
    "video/quicktime",
    "video/x-msvideo",
    "video/webm",
];

/// Check if a MIME type is allowed for attachments
pub fn is_attachment_mime_type_allowed(content_type: &str) -> bool {
    // Check exact match first
    if ALLOWED_ATTACHMENT_MIME_TYPES.contains(&content_type) {
        return true;
    }

    // Also allow any image/* or video/* type
    content_type.starts_with("image/") || content_type.starts_with("video/")
}

/// Response DTO for thread attachment
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ThreadAttachmentResponseDto {
    /// Unique identifier for the attachment
    pub id: Uuid,
    /// Thread ID this attachment belongs to
    pub thread_id: Uuid,
    /// File ID reference
    pub file_id: Uuid,
    /// Original filename
    pub original_filename: String,
    /// MIME type of the file
    pub content_type: String,
    /// Size of the file in bytes
    pub file_size: i64,
    /// URL to access the file (presigned URL for private files)
    pub url: String,
    /// Timestamp when the attachment was created
    pub created_at: DateTime<Utc>,
}

/// DTO for attachment count information
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AttachmentCountDto {
    /// Current number of attachments in the thread
    pub count: i64,
    /// Maximum allowed attachments per thread
    pub max_allowed: i64,
    /// Whether more attachments can be uploaded
    pub can_upload: bool,
}

/// Upload attachment request DTO for OpenAPI documentation
/// Note: This struct is for Swagger UI documentation only.
/// The actual handler uses axum's Multipart extractor directly.
#[derive(Debug, ToSchema)]
#[allow(dead_code)]
pub struct UploadAttachmentDto {
    /// The file to upload
    #[schema(format = Binary, content_media_type = "application/octet-stream")]
    pub file: String,
}

/// Response DTO for delete attachment operations
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct DeleteAttachmentResponseDto {
    /// Confirmation that the attachment was deleted
    pub deleted: bool,
}
