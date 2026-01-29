use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// Request DTO for contributor registration
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateContributorDto {
    /// Submission type: "personal" or "organization"
    pub submission_type: String,

    // Personal contributor fields
    pub name: Option<String>,
    pub email: Option<String>,
    pub whatsapp: Option<String>,
    pub city: Option<String>,
    pub role: Option<String>,
    pub skills: Option<String>,
    pub bio: Option<String>,
    pub portfolio_url: Option<String>,
    pub aspiration: Option<String>,

    // Organization contributor fields
    pub organization_name: Option<String>,
    pub organization_type: Option<String>,
    pub contact_name: Option<String>,
    pub contact_position: Option<String>,
    pub contact_whatsapp: Option<String>,
    pub contact_email: Option<String>,
    pub contribution_offer: Option<String>,

    // Common
    #[serde(default)]
    pub agreed: bool,
}

/// Response DTO for contributor registration
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ContributorResponseDto {
    pub id: Uuid,
    pub submission_type: String,
}
