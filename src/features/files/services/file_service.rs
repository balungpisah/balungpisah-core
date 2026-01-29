use sqlx::PgPool;
use std::sync::Arc;
use tracing::{debug, info};
use uuid::Uuid;

use crate::core::error::{AppError, Result};
use crate::features::files::dtos::{
    get_extension_from_content_type, FileResponseDto, FileVisibilityDto,
};
use crate::features::files::models::File;
use crate::modules::storage::{FileVisibility, MinIOClient};

/// Service for file operations
pub struct FileService {
    pool: PgPool,
    minio_client: Arc<MinIOClient>,
}

impl FileService {
    pub fn new(pool: PgPool, minio_client: Arc<MinIOClient>) -> Self {
        Self { pool, minio_client }
    }

    /// Convert DTO visibility to storage visibility
    fn to_storage_visibility(visibility: FileVisibilityDto) -> FileVisibility {
        match visibility {
            FileVisibilityDto::Public => FileVisibility::Public,
            FileVisibilityDto::Private => FileVisibility::Private,
        }
    }

    /// Convert storage visibility to DTO visibility
    fn to_dto_visibility(visibility: &str) -> FileVisibilityDto {
        match visibility {
            "private" => FileVisibilityDto::Private,
            _ => FileVisibilityDto::Public,
        }
    }

    /// Upload a file to storage and save metadata to database
    ///
    /// # Arguments
    /// * `data` - The file content as bytes
    /// * `original_filename` - The original filename
    /// * `content_type` - The MIME type of the file
    /// * `visibility` - Whether the file is public or private
    /// * `purpose` - Optional purpose/category for the file
    /// * `user_id` - The ID of the user uploading the file
    ///
    /// # Returns
    /// The file response DTO with metadata
    pub async fn upload_file(
        &self,
        data: Vec<u8>,
        original_filename: &str,
        content_type: &str,
        visibility: FileVisibilityDto,
        purpose: Option<String>,
        user_id: &str,
    ) -> Result<FileResponseDto> {
        let file_size = data.len() as i64;

        // Generate unique file key with visibility prefix
        let file_id = Uuid::new_v4();
        let extension = get_extension_from_content_type(content_type)
            .unwrap_or_else(|| original_filename.rsplit('.').next().unwrap_or("bin"));

        // Build path: {purpose}/{user_id}/{file_id}.{extension}
        let purpose_path = purpose.as_deref().unwrap_or("uploads");
        let path = format!("{}/{}/{}.{}", purpose_path, user_id, file_id, extension);

        // Generate key with visibility prefix (e.g., public/uploads/user123/file.pdf)
        let storage_visibility = Self::to_storage_visibility(visibility);
        let file_key = self.minio_client.generate_key(storage_visibility, &path);

        // Upload to MinIO
        self.minio_client
            .upload(&file_key, data, content_type)
            .await?;

        debug!("File uploaded to MinIO: {}", file_key);

        // Generate URL based on visibility
        let url = self.minio_client.get_file_url(&file_key);

        // Visibility string for database
        let visibility_str = match visibility {
            FileVisibilityDto::Public => "public",
            FileVisibilityDto::Private => "private",
        };

        // Save metadata to database
        let file = sqlx::query_as!(
            File,
            r#"
            INSERT INTO files (file_key, original_filename, content_type, file_size, url, visibility, purpose, uploaded_by)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING *
            "#,
            file_key,
            original_filename,
            content_type,
            file_size,
            url,
            visibility_str,
            purpose,
            user_id
        )
        .fetch_one(&self.pool)
        .await?;

        info!(
            "File metadata saved: id={}, key={}, visibility={}, size={}",
            file.id, file.file_key, file.visibility, file.file_size
        );

        Ok(FileResponseDto {
            id: file.id,
            original_filename: file.original_filename,
            content_type: file.content_type,
            file_size: file.file_size,
            url: file.url,
            visibility: Self::to_dto_visibility(&file.visibility),
            purpose: file.purpose,
            created_at: file.created_at,
        })
    }

    /// Get a presigned URL for a private file
    ///
    /// # Arguments
    /// * `file_id` - The ID of the file
    /// * `user_id` - The ID of the user requesting access
    ///
    /// # Returns
    /// A presigned URL for the file
    #[allow(dead_code)]
    pub async fn get_presigned_url(&self, file_id: Uuid, user_id: &str) -> Result<String> {
        // Find the file
        let file = sqlx::query_as!(
            File,
            r#"
            SELECT * FROM files
            WHERE id = $1 AND is_active = TRUE
            "#,
            file_id
        )
        .fetch_optional(&self.pool)
        .await?;

        let file = file.ok_or_else(|| AppError::NotFound("File not found".to_string()))?;

        // Check ownership for private files
        if file.visibility == "private" && file.uploaded_by != user_id {
            return Err(AppError::Forbidden(
                "You do not have permission to access this file".to_string(),
            ));
        }

        // Generate presigned URL
        self.minio_client.get_presigned_url(&file.file_key).await
    }

    /// Delete a file by its URL
    ///
    /// Only the owner of the file can delete it.
    ///
    /// # Arguments
    /// * `url` - The URL of the file to delete
    /// * `user_id` - The ID of the user requesting deletion
    ///
    /// # Returns
    /// Ok(()) if successful
    pub async fn delete_by_url(&self, url: &str, user_id: &str) -> Result<()> {
        // Find the file by URL
        let file = sqlx::query_as!(
            File,
            r#"
            SELECT * FROM files
            WHERE url = $1 AND is_active = TRUE
            "#,
            url
        )
        .fetch_optional(&self.pool)
        .await?;

        let file = file.ok_or_else(|| AppError::NotFound("File not found".to_string()))?;

        // Check ownership
        if file.uploaded_by != user_id {
            return Err(AppError::Forbidden(
                "You do not have permission to delete this file".to_string(),
            ));
        }

        // Delete from MinIO
        self.minio_client.delete(&file.file_key).await?;

        debug!("File deleted from MinIO: {}", file.file_key);

        // Soft delete in database
        sqlx::query!(
            r#"
            UPDATE files
            SET is_active = FALSE, updated_at = NOW()
            WHERE id = $1
            "#,
            file.id
        )
        .execute(&self.pool)
        .await?;

        info!("File soft deleted: id={}, key={}", file.id, file.file_key);

        Ok(())
    }
}
