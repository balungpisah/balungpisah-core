use sqlx::PgPool;
use std::sync::Arc;

use crate::core::error::{AppError, Result};
use crate::features::expectations::dtos::{CreateExpectationDto, ExpectationResponseDto};
use crate::features::expectations::models::Expectation;
use crate::shared::encryption::EncryptionService;

/// Service for managing user expectations
pub struct ExpectationService {
    pool: PgPool,
    encryption: Arc<EncryptionService>,
}

impl ExpectationService {
    pub fn new(pool: PgPool, encryption: Arc<EncryptionService>) -> Self {
        Self { pool, encryption }
    }

    /// Create a new expectation from landing page submission
    pub async fn create(&self, dto: CreateExpectationDto) -> Result<ExpectationResponseDto> {
        // Encrypt email if provided
        let email_encrypted = self
            .encryption
            .encrypt_opt(dto.email.as_deref())
            .map_err(|e| {
                tracing::error!("Failed to encrypt email: {}", e);
                AppError::Internal(format!("Encryption failed: {}", e))
            })?;

        // Generate blind index for email if provided
        let email_index = self.encryption.blind_index_opt(dto.email.as_deref());

        let expectation = sqlx::query_as!(
            Expectation,
            r#"
            INSERT INTO expectations (name, email, email_index, expectation)
            VALUES ($1, $2, $3, $4)
            RETURNING id, name, email, email_index, expectation, created_at
            "#,
            dto.name,
            email_encrypted,
            email_index,
            dto.expectation
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to create expectation: {:?}", e);
            AppError::Database(e)
        })?;

        tracing::info!(
            "Expectation created: id={}, has_email={}",
            expectation.id,
            email_encrypted.is_some()
        );

        // Decrypt for response
        expectation.to_dto(&self.encryption).map_err(|e| {
            tracing::error!("Failed to decrypt expectation for response: {}", e);
            AppError::Internal(format!("Decryption failed: {}", e))
        })
    }
}
