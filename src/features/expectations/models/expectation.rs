use chrono::{DateTime, Utc};
use sqlx::FromRow;
use uuid::Uuid;

use crate::features::expectations::dtos::ExpectationResponseDto;
use crate::shared::encryption::EncryptionService;

/// Database model for expectation
#[derive(Debug, Clone, FromRow)]
pub struct Expectation {
    pub id: Uuid,
    pub name: Option<String>,
    pub email: Option<String>,
    #[allow(dead_code)]
    pub email_index: Option<String>,
    pub expectation: String,
    pub created_at: DateTime<Utc>,
}

impl Expectation {
    /// Convert to DTO with decrypted PII fields
    ///
    /// # Arguments
    /// * `encryption` - EncryptionService instance for decrypting email
    ///
    /// # Returns
    /// * `Result<ExpectationResponseDto, String>` - DTO or decryption error
    pub fn to_dto(&self, encryption: &EncryptionService) -> Result<ExpectationResponseDto, String> {
        let email = encryption.decrypt_opt(self.email.as_deref())?;

        Ok(ExpectationResponseDto {
            id: self.id,
            name: self.name.clone(),
            email,
            expectation: self.expectation.clone(),
            created_at: self.created_at,
        })
    }
}
