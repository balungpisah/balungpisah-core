use chrono::Utc;
use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;

use crate::core::error::{AppError, Result};
use crate::features::tickets::dtos::{TicketCreatedDto, TicketResponseDto};
use crate::features::tickets::models::{Ticket, TicketStatus};

/// Service for ticket operations
pub struct TicketService {
    pool: PgPool,
}

impl TicketService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Generate a reference number in format: TKT-YYYY-NNNNNNN
    async fn generate_reference_number(&self) -> Result<String> {
        let year = Utc::now().format("%Y").to_string();

        // Get next value from sequence
        let seq: i64 = sqlx::query_scalar!("SELECT nextval('ticket_reference_seq')")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| {
                tracing::error!("Failed to get next sequence value: {:?}", e);
                AppError::Database(e)
            })?
            .unwrap_or(1);

        Ok(format!("TKT-{}-{:07}", year, seq))
    }

    /// Create a ticket from agent tool call
    pub async fn create_from_agent(
        &self,
        adk_thread_id: Uuid,
        user_id: &str,
        confidence: f64,
        platform: Option<&str>,
    ) -> Result<TicketCreatedDto> {
        let reference_number = self.generate_reference_number().await?;
        let platform = platform.unwrap_or("web");

        // Clamp confidence to valid range
        let confidence = confidence.clamp(0.0, 1.0);
        let confidence_decimal =
            Decimal::try_from(confidence).unwrap_or_else(|_| Decimal::new(50, 2));

        let ticket = sqlx::query_as!(
            Ticket,
            r#"
            INSERT INTO tickets (
                adk_thread_id, user_id, reference_number, platform, confidence_score, status
            ) VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING
                id, adk_thread_id, user_id, reference_number, platform,
                confidence_score, completeness_score, missing_fields, preliminary_data,
                status as "status: TicketStatus",
                submitted_at, processed_at, error_message, retry_count,
                created_at, updated_at
            "#,
            adk_thread_id,
            user_id,
            &reference_number,
            platform,
            confidence_decimal,
            TicketStatus::Submitted as TicketStatus
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to create ticket: {:?}", e);
            AppError::Database(e)
        })?;

        tracing::info!(
            "Ticket created: id={}, ref={}, user={}",
            ticket.id,
            ticket.reference_number,
            user_id
        );

        Ok(TicketCreatedDto {
            reference_number: ticket.reference_number,
            ticket_id: ticket.id,
        })
    }

    /// Get ticket by ID
    pub async fn get_by_id(&self, id: Uuid, user_id: &str) -> Result<TicketResponseDto> {
        let ticket = sqlx::query_as!(
            Ticket,
            r#"
            SELECT
                id, adk_thread_id, user_id, reference_number, platform,
                confidence_score, completeness_score, missing_fields, preliminary_data,
                status as "status: TicketStatus",
                submitted_at, processed_at, error_message, retry_count,
                created_at, updated_at
            FROM tickets
            WHERE id = $1 AND user_id = $2
            "#,
            id,
            user_id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get ticket by ID: {:?}", e);
            AppError::Database(e)
        })?;

        ticket
            .map(|t| t.into())
            .ok_or_else(|| AppError::NotFound(format!("Ticket '{}' not found", id)))
    }

    /// Get ticket by reference number
    pub async fn get_by_reference(
        &self,
        reference: &str,
        user_id: &str,
    ) -> Result<TicketResponseDto> {
        let ticket = sqlx::query_as!(
            Ticket,
            r#"
            SELECT
                id, adk_thread_id, user_id, reference_number, platform,
                confidence_score, completeness_score, missing_fields, preliminary_data,
                status as "status: TicketStatus",
                submitted_at, processed_at, error_message, retry_count,
                created_at, updated_at
            FROM tickets
            WHERE reference_number = $1 AND user_id = $2
            "#,
            reference,
            user_id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get ticket by reference: {:?}", e);
            AppError::Database(e)
        })?;

        ticket
            .map(|t| t.into())
            .ok_or_else(|| AppError::NotFound(format!("Ticket '{}' not found", reference)))
    }

    /// List tickets for a user
    pub async fn list_by_user(&self, user_id: &str) -> Result<Vec<TicketResponseDto>> {
        let tickets = sqlx::query_as!(
            Ticket,
            r#"
            SELECT
                id, adk_thread_id, user_id, reference_number, platform,
                confidence_score, completeness_score, missing_fields, preliminary_data,
                status as "status: TicketStatus",
                submitted_at, processed_at, error_message, retry_count,
                created_at, updated_at
            FROM tickets
            WHERE user_id = $1
            ORDER BY created_at DESC
            "#,
            user_id
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to list tickets by user: {:?}", e);
            AppError::Database(e)
        })?;

        Ok(tickets.into_iter().map(|t| t.into()).collect())
    }
}
