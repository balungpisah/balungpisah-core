use std::sync::Arc;

use balungpisah_adk::{PostgresStorage, ThreadStorage};
use sqlx::PgPool;
use tracing::{debug, info};
use uuid::Uuid;

use crate::core::error::{AppError, Result};
use crate::features::citizen_report_agent::dtos::{
    AttachmentCountDto, ThreadAttachmentResponseDto, MAX_ATTACHMENTS_PER_THREAD,
    MAX_ATTACHMENT_SIZE,
};
use crate::features::citizen_report_agent::models::ThreadAttachment;
use crate::features::files::dtos::get_extension_from_content_type;
use crate::modules::storage::{FileVisibility, MinIOClient};

/// Service for managing thread attachments
pub struct ThreadAttachmentService {
    pool: PgPool,
    minio_client: Arc<MinIOClient>,
    adk_storage: Arc<PostgresStorage>,
}

impl ThreadAttachmentService {
    pub fn new(
        pool: PgPool,
        minio_client: Arc<MinIOClient>,
        adk_storage: Arc<PostgresStorage>,
    ) -> Self {
        Self {
            pool,
            minio_client,
            adk_storage,
        }
    }

    /// Verify thread ownership - returns error if thread doesn't belong to user
    async fn verify_thread_ownership(&self, thread_id: Uuid, owner_id: &str) -> Result<()> {
        let thread = self
            .adk_storage
            .get_thread(thread_id)
            .await
            .map_err(|e| AppError::Internal(format!("Failed to get thread: {}", e)))?;

        match thread {
            Some(t) if t.external_id == owner_id => Ok(()),
            Some(_) => Err(AppError::Forbidden(
                "Thread does not belong to this user".to_string(),
            )),
            None => Err(AppError::NotFound(format!(
                "Thread {} not found",
                thread_id
            ))),
        }
    }

    /// Get attachment count for a thread
    pub async fn count_attachments(
        &self,
        thread_id: Uuid,
        owner_id: &str,
    ) -> Result<AttachmentCountDto> {
        // Verify thread ownership
        self.verify_thread_ownership(thread_id, owner_id).await?;

        let count = sqlx::query_scalar!(
            r#"SELECT COUNT(*) as "count!" FROM thread_attachments WHERE thread_id = $1"#,
            thread_id
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(AttachmentCountDto {
            count,
            max_allowed: MAX_ATTACHMENTS_PER_THREAD,
            can_upload: count < MAX_ATTACHMENTS_PER_THREAD,
        })
    }

    /// Upload an attachment to a thread
    pub async fn upload_attachment(
        &self,
        thread_id: Uuid,
        owner_id: &str,
        data: Vec<u8>,
        original_filename: &str,
        content_type: &str,
    ) -> Result<ThreadAttachmentResponseDto> {
        // Verify thread ownership
        self.verify_thread_ownership(thread_id, owner_id).await?;

        // Check file size
        if data.len() > MAX_ATTACHMENT_SIZE {
            return Err(AppError::BadRequest(format!(
                "File too large. Maximum size is {} bytes ({} MB)",
                MAX_ATTACHMENT_SIZE,
                MAX_ATTACHMENT_SIZE / 1024 / 1024
            )));
        }

        // Check attachment count
        let count_info = self.count_attachments(thread_id, owner_id).await?;
        if !count_info.can_upload {
            return Err(AppError::BadRequest(format!(
                "Maximum number of attachments ({}) reached for this thread",
                MAX_ATTACHMENTS_PER_THREAD
            )));
        }

        let file_size = data.len() as i64;

        // Generate unique file key
        let file_id = Uuid::new_v4();
        let extension = get_extension_from_content_type(content_type)
            .or_else(|| get_extension_from_video_type(content_type))
            .unwrap_or_else(|| original_filename.rsplit('.').next().unwrap_or("bin"));

        // Build path: thread-attachments/{owner_id}/{thread_id}/{file_id}.{extension}
        let path = format!(
            "thread-attachments/{}/{}/{}.{}",
            owner_id, thread_id, file_id, extension
        );

        // Private visibility for all attachments
        let file_key = self
            .minio_client
            .generate_key(FileVisibility::Private, &path);

        // Upload to MinIO
        self.minio_client
            .upload(&file_key, data, content_type)
            .await?;

        debug!("Attachment uploaded to MinIO: {}", file_key);

        // Get URL for the file
        let url = self.minio_client.get_file_url(&file_key);

        // Save file metadata to database
        let file = sqlx::query_as!(
            crate::features::files::models::File,
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
            "private",
            Some("thread-attachment".to_string()),
            owner_id
        )
        .fetch_one(&self.pool)
        .await?;

        // Create thread attachment record
        let attachment = sqlx::query_as!(
            ThreadAttachment,
            r#"
            INSERT INTO thread_attachments (thread_id, file_id, owner_id)
            VALUES ($1, $2, $3)
            RETURNING id, thread_id, file_id, owner_id, created_at
            "#,
            thread_id,
            file.id,
            owner_id
        )
        .fetch_one(&self.pool)
        .await?;

        info!(
            "Thread attachment created: id={}, thread_id={}, file_id={}",
            attachment.id, thread_id, file.id
        );

        // Get presigned URL for response
        let presigned_url = self.minio_client.get_presigned_url(&file_key).await?;

        Ok(ThreadAttachmentResponseDto {
            id: attachment.id,
            thread_id: attachment.thread_id,
            file_id: file.id,
            original_filename: file.original_filename,
            content_type: file.content_type,
            file_size: file.file_size,
            url: presigned_url,
            created_at: attachment.created_at,
        })
    }

    /// List all attachments for a thread
    pub async fn list_attachments(
        &self,
        thread_id: Uuid,
        owner_id: &str,
    ) -> Result<Vec<ThreadAttachmentResponseDto>> {
        // Verify thread ownership
        self.verify_thread_ownership(thread_id, owner_id).await?;

        // Query attachments with file info
        let rows = sqlx::query!(
            r#"
            SELECT
                ta.id as attachment_id,
                ta.thread_id,
                ta.created_at as attachment_created_at,
                f.id as file_id,
                f.file_key,
                f.original_filename,
                f.content_type,
                f.file_size
            FROM thread_attachments ta
            JOIN files f ON ta.file_id = f.id
            WHERE ta.thread_id = $1 AND f.is_active = TRUE
            ORDER BY ta.created_at ASC
            "#,
            thread_id
        )
        .fetch_all(&self.pool)
        .await?;

        let mut attachments = Vec::with_capacity(rows.len());

        for row in rows {
            // Get presigned URL for each file
            let presigned_url = self.minio_client.get_presigned_url(&row.file_key).await?;

            attachments.push(ThreadAttachmentResponseDto {
                id: row.attachment_id,
                thread_id: row.thread_id,
                file_id: row.file_id,
                original_filename: row.original_filename,
                content_type: row.content_type,
                file_size: row.file_size,
                url: presigned_url,
                created_at: row.attachment_created_at,
            });
        }

        Ok(attachments)
    }

    /// Delete an attachment
    pub async fn delete_attachment(
        &self,
        thread_id: Uuid,
        attachment_id: Uuid,
        owner_id: &str,
    ) -> Result<()> {
        // Verify thread ownership
        self.verify_thread_ownership(thread_id, owner_id).await?;

        // Get attachment and verify it belongs to the thread
        let attachment = sqlx::query!(
            r#"
            SELECT ta.id, ta.file_id, f.file_key
            FROM thread_attachments ta
            JOIN files f ON ta.file_id = f.id
            WHERE ta.id = $1 AND ta.thread_id = $2
            "#,
            attachment_id,
            thread_id
        )
        .fetch_optional(&self.pool)
        .await?;

        let attachment = attachment.ok_or_else(|| {
            AppError::NotFound(format!(
                "Attachment {} not found in thread {}",
                attachment_id, thread_id
            ))
        })?;

        // Delete from MinIO
        self.minio_client.delete(&attachment.file_key).await?;

        debug!("Attachment deleted from MinIO: {}", attachment.file_key);

        // Soft delete file record
        sqlx::query!(
            r#"
            UPDATE files
            SET is_active = FALSE, updated_at = NOW()
            WHERE id = $1
            "#,
            attachment.file_id
        )
        .execute(&self.pool)
        .await?;

        // Delete attachment record
        sqlx::query!(
            r#"DELETE FROM thread_attachments WHERE id = $1"#,
            attachment_id
        )
        .execute(&self.pool)
        .await?;

        info!(
            "Thread attachment deleted: id={}, thread_id={}",
            attachment_id, thread_id
        );

        Ok(())
    }

    /// Get attachment context string for agent injection
    /// Returns None if no attachments exist
    pub async fn get_attachment_context(
        &self,
        thread_id: Uuid,
        owner_id: &str,
    ) -> Result<Option<String>> {
        // Don't verify ownership here as this is called internally
        // and the thread ownership is already verified by the chat endpoint

        let rows = sqlx::query!(
            r#"
            SELECT
                f.original_filename,
                f.content_type,
                f.file_size
            FROM thread_attachments ta
            JOIN files f ON ta.file_id = f.id
            WHERE ta.thread_id = $1 AND ta.owner_id = $2 AND f.is_active = TRUE
            ORDER BY ta.created_at ASC
            "#,
            thread_id,
            owner_id
        )
        .fetch_all(&self.pool)
        .await?;

        if rows.is_empty() {
            return Ok(None);
        }

        let mut context = format!(
            "The user has uploaded {} supporting document{}:\n",
            rows.len(),
            if rows.len() == 1 { "" } else { "s" }
        );

        for (i, row) in rows.iter().enumerate() {
            let size_str = format_file_size(row.file_size);
            context.push_str(&format!(
                "{}. {} ({}, {})\n",
                i + 1,
                row.original_filename,
                row.content_type,
                size_str
            ));
        }

        context.push_str(
            "\nNote: You cannot view these files directly. Acknowledge that the user has provided supporting documentation.",
        );

        Ok(Some(context))
    }
}

/// Format file size in human readable format
fn format_file_size(bytes: i64) -> String {
    const KB: i64 = 1024;
    const MB: i64 = KB * 1024;

    if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.0} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} bytes", bytes)
    }
}

/// Get file extension from video content type
fn get_extension_from_video_type(content_type: &str) -> Option<&'static str> {
    match content_type {
        "video/mp4" => Some("mp4"),
        "video/mpeg" => Some("mpeg"),
        "video/quicktime" => Some("mov"),
        "video/x-msvideo" => Some("avi"),
        "video/webm" => Some("webm"),
        _ => None,
    }
}
