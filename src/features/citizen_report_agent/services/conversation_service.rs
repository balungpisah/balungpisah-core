use std::sync::Arc;

use balungpisah_adk::{MessageStorage, PostgresStorage, ThreadStorage};
use uuid::Uuid;

use crate::core::error::{AppError, Result};

use super::super::dtos::{
    ListMessagesQuery, ListThreadsQuery, MessageResponseDto, ThreadDetailDto, ThreadResponseDto,
};

/// Service for conversation thread and message operations
pub struct ConversationService {
    storage: Arc<PostgresStorage>,
}

impl ConversationService {
    /// Create a new ConversationService
    pub fn new(storage: Arc<PostgresStorage>) -> Self {
        Self { storage }
    }

    /// List threads for a user
    pub async fn list_threads(
        &self,
        external_id: &str,
        query: &ListThreadsQuery,
    ) -> Result<(Vec<ThreadResponseDto>, i64)> {
        // List threads owned by this user
        // Note: ADK's list_threads uses prefix matching with LIKE
        // Since external_id is a UUID (account_id), prefix matching is effectively exact matching
        let threads = self
            .storage
            .list_threads(
                Some(external_id),
                query.limit() as usize,
                query.offset() as usize,
            )
            .await
            .map_err(|e| AppError::Internal(format!("Failed to list threads: {}", e)))?;

        // Filter threads to ensure exact match (extra safety for UUID account_ids)
        let filtered_threads: Vec<_> = threads
            .into_iter()
            .filter(|t| t.external_id == external_id)
            .collect();

        let total = filtered_threads.len() as i64;

        let dtos: Vec<ThreadResponseDto> = filtered_threads
            .into_iter()
            .map(|t| ThreadResponseDto {
                id: t.id,
                title: t.title,
                created_at: t.created_at,
                updated_at: t.updated_at,
            })
            .collect();

        Ok((dtos, total))
    }

    /// Get thread details with message count
    pub async fn get_thread(&self, external_id: &str, thread_id: Uuid) -> Result<ThreadDetailDto> {
        // Get thread and verify ownership
        let thread = self
            .storage
            .get_thread(thread_id)
            .await
            .map_err(|e| AppError::Internal(format!("Failed to get thread: {}", e)))?
            .ok_or_else(|| AppError::NotFound(format!("Thread {} not found", thread_id)))?;

        if thread.external_id != external_id {
            return Err(AppError::Forbidden(
                "Thread does not belong to this user".to_string(),
            ));
        }

        // Count messages
        let message_count = self
            .storage
            .count_thread_messages(thread_id)
            .await
            .map_err(|e| AppError::Internal(format!("Failed to count messages: {}", e)))?;

        Ok(ThreadDetailDto {
            id: thread.id,
            title: thread.title,
            message_count: message_count as i64,
            created_at: thread.created_at,
            updated_at: thread.updated_at,
        })
    }

    /// List messages in a thread
    pub async fn list_messages(
        &self,
        external_id: &str,
        thread_id: Uuid,
        query: &ListMessagesQuery,
    ) -> Result<(Vec<MessageResponseDto>, i64)> {
        // Verify thread ownership first
        let thread = self
            .storage
            .get_thread(thread_id)
            .await
            .map_err(|e| AppError::Internal(format!("Failed to get thread: {}", e)))?
            .ok_or_else(|| AppError::NotFound(format!("Thread {} not found", thread_id)))?;

        if thread.external_id != external_id {
            return Err(AppError::Forbidden(
                "Thread does not belong to this user".to_string(),
            ));
        }

        // Get messages
        let messages = self
            .storage
            .get_thread_messages(thread_id)
            .await
            .map_err(|e| AppError::Internal(format!("Failed to get messages: {}", e)))?;

        let total = messages.len() as i64;

        // Apply pagination manually since ADK doesn't have pagination for messages
        let offset = query.offset() as usize;
        let limit = query.limit() as usize;
        let paginated: Vec<MessageResponseDto> = messages
            .into_iter()
            .skip(offset)
            .take(limit)
            .map(|m| MessageResponseDto {
                id: m.id,
                thread_id: m.thread_id,
                role: format!("{:?}", m.role).to_lowercase(),
                content: serde_json::to_value(&m.content).unwrap_or_default(),
                episode_id: m.episode_id,
                created_at: m.created_at,
            })
            .collect();

        Ok((paginated, total))
    }
}
