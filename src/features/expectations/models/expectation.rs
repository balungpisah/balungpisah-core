use chrono::{DateTime, Utc};
use sqlx::FromRow;
use uuid::Uuid;

use crate::features::expectations::dtos::ExpectationResponseDto;

/// Database model for expectation
#[derive(Debug, Clone, FromRow)]
pub struct Expectation {
    pub id: Uuid,
    pub name: Option<String>,
    pub email: Option<String>,
    pub expectation: String,
    pub created_at: DateTime<Utc>,
}

impl From<Expectation> for ExpectationResponseDto {
    fn from(e: Expectation) -> Self {
        Self {
            id: e.id,
            name: e.name,
            email: e.email,
            expectation: e.expectation,
            created_at: e.created_at,
        }
    }
}
