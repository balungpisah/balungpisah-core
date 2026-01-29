use sqlx::PgPool;

use crate::core::error::{AppError, Result};
use crate::features::expectations::dtos::{CreateExpectationDto, ExpectationResponseDto};
use crate::features::expectations::models::Expectation;

/// Service for managing user expectations
pub struct ExpectationService {
    pool: PgPool,
}

impl ExpectationService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Create a new expectation from landing page submission
    pub async fn create(&self, dto: CreateExpectationDto) -> Result<ExpectationResponseDto> {
        let expectation = sqlx::query_as!(
            Expectation,
            r#"
            INSERT INTO expectations (name, email, expectation)
            VALUES ($1, $2, $3)
            RETURNING id, name, email, expectation, created_at
            "#,
            dto.name,
            dto.email,
            dto.expectation
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to create expectation: {:?}", e);
            AppError::Database(e)
        })?;

        tracing::info!(
            "Expectation created: id={}, email={:?}",
            expectation.id,
            expectation.email
        );

        Ok(expectation.into())
    }
}
