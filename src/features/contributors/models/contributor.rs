use chrono::{DateTime, Utc};
use sqlx::FromRow;
use uuid::Uuid;

use crate::features::contributors::dtos::ContributorResponseDto;

/// Database model for contributor
#[derive(Debug, Clone, FromRow)]
#[allow(dead_code)]
pub struct Contributor {
    pub id: Uuid,
    pub submission_type: String,

    // Personal fields
    pub name: Option<String>,
    pub email: Option<String>,
    pub whatsapp: Option<String>,
    pub city: Option<String>,
    pub role: Option<String>,
    pub skills: Option<String>,
    pub bio: Option<String>,
    pub portfolio_url: Option<String>,
    pub aspiration: Option<String>,

    // Organization fields
    pub organization_name: Option<String>,
    pub organization_type: Option<String>,
    pub contact_name: Option<String>,
    pub contact_position: Option<String>,
    pub contact_whatsapp: Option<String>,
    pub contact_email: Option<String>,
    pub contribution_offer: Option<String>,

    // Common
    pub agreed: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<Contributor> for ContributorResponseDto {
    fn from(c: Contributor) -> Self {
        Self {
            id: c.id,
            submission_type: c.submission_type,
        }
    }
}
