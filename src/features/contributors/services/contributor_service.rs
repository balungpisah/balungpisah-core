//! Contributor Service - Simple data storage

use sqlx::PgPool;

use crate::core::error::{AppError, Result};
use crate::features::contributors::dtos::{ContributorResponseDto, CreateContributorDto};
use crate::features::contributors::models::Contributor;

/// Service for storing contributor registrations
pub struct ContributorService {
    pool: PgPool,
}

impl ContributorService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Register a new contributor - just stores data, no auth or email
    pub async fn register(&self, dto: CreateContributorDto) -> Result<ContributorResponseDto> {
        let contributor = sqlx::query_as!(
            Contributor,
            r#"
            INSERT INTO contributors (
                submission_type,
                name, email, whatsapp, city, role, skills, bio, portfolio_url, aspiration,
                organization_name, organization_type, contact_name, contact_position,
                contact_whatsapp, contact_email, contribution_offer,
                agreed
            ) VALUES (
                $1,
                $2, $3, $4, $5, $6, $7, $8, $9, $10,
                $11, $12, $13, $14, $15, $16, $17,
                $18
            )
            RETURNING *
            "#,
            dto.submission_type,
            dto.name,
            dto.email,
            dto.whatsapp,
            dto.city,
            dto.role,
            dto.skills,
            dto.bio,
            dto.portfolio_url,
            dto.aspiration,
            dto.organization_name,
            dto.organization_type,
            dto.contact_name,
            dto.contact_position,
            dto.contact_whatsapp,
            dto.contact_email,
            dto.contribution_offer,
            dto.agreed
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to insert contributor: {:?}", e);
            AppError::Database(e)
        })?;

        tracing::info!(
            "Contributor registered: id={}, type={}",
            contributor.id,
            contributor.submission_type
        );

        Ok(contributor.into())
    }
}
