//! Contributor Service - Simple data storage

use sqlx::PgPool;
use std::sync::Arc;

use crate::core::error::{AppError, Result};
use crate::features::contributors::dtos::{ContributorResponseDto, CreateContributorDto};
use crate::features::contributors::models::Contributor;
use crate::shared::encryption::EncryptionService;

/// Service for storing contributor registrations
pub struct ContributorService {
    pool: PgPool,
    encryption: Arc<EncryptionService>,
}

impl ContributorService {
    pub fn new(pool: PgPool, encryption: Arc<EncryptionService>) -> Self {
        Self { pool, encryption }
    }

    /// Register a new contributor - just stores data, no auth or email
    pub async fn register(&self, dto: CreateContributorDto) -> Result<ContributorResponseDto> {
        // Encrypt PII fields
        let email_encrypted = self
            .encryption
            .encrypt_opt(dto.email.as_deref())
            .map_err(|e| {
                tracing::error!("Failed to encrypt email: {}", e);
                AppError::Internal(format!("Encryption failed: {}", e))
            })?;

        let whatsapp_encrypted = self
            .encryption
            .encrypt_opt(dto.whatsapp.as_deref())
            .map_err(|e| {
                tracing::error!("Failed to encrypt whatsapp: {}", e);
                AppError::Internal(format!("Encryption failed: {}", e))
            })?;

        let contact_email_encrypted = self
            .encryption
            .encrypt_opt(dto.contact_email.as_deref())
            .map_err(|e| {
                tracing::error!("Failed to encrypt contact_email: {}", e);
                AppError::Internal(format!("Encryption failed: {}", e))
            })?;

        let contact_whatsapp_encrypted = self
            .encryption
            .encrypt_opt(dto.contact_whatsapp.as_deref())
            .map_err(|e| {
                tracing::error!("Failed to encrypt contact_whatsapp: {}", e);
                AppError::Internal(format!("Encryption failed: {}", e))
            })?;

        // Generate blind indexes
        let email_index = self.encryption.blind_index_opt(dto.email.as_deref());
        let whatsapp_index = self.encryption.blind_index_opt(dto.whatsapp.as_deref());
        let contact_email_index = self
            .encryption
            .blind_index_opt(dto.contact_email.as_deref());
        let contact_whatsapp_index = self
            .encryption
            .blind_index_opt(dto.contact_whatsapp.as_deref());

        let contributor = sqlx::query_as!(
            Contributor,
            r#"
            INSERT INTO contributors (
                submission_type,
                name, email_encrypted, email_index, whatsapp_encrypted, whatsapp_index, city, role, skills, bio, portfolio_url, aspiration,
                organization_name, organization_type, contact_name, contact_position,
                contact_whatsapp_encrypted, contact_whatsapp_index, contact_email_encrypted, contact_email_index, contribution_offer,
                agreed
            ) VALUES (
                $1,
                $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12,
                $13, $14, $15, $16, $17, $18, $19, $20, $21,
                $22
            )
            RETURNING
                id, submission_type,
                name, email_encrypted as email, email_index, whatsapp_encrypted as whatsapp, whatsapp_index,
                city, role, skills, bio, portfolio_url, aspiration,
                organization_name, organization_type, contact_name, contact_position,
                contact_whatsapp_encrypted as contact_whatsapp, contact_whatsapp_index,
                contact_email_encrypted as contact_email, contact_email_index, contribution_offer,
                agreed, created_at, updated_at
            "#,
            dto.submission_type,
            dto.name,
            email_encrypted,
            email_index,
            whatsapp_encrypted,
            whatsapp_index,
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
            contact_whatsapp_encrypted,
            contact_whatsapp_index,
            contact_email_encrypted,
            contact_email_index,
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
            "Contributor registered: id={}, type={}, has_email={}, has_whatsapp={}, has_contact_email={}, has_contact_whatsapp={}",
            contributor.id,
            contributor.submission_type,
            email_encrypted.is_some(),
            whatsapp_encrypted.is_some(),
            contact_email_encrypted.is_some(),
            contact_whatsapp_encrypted.is_some()
        );

        Ok(contributor.into())
    }
}
