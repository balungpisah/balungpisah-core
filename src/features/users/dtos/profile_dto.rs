use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

/// Extended profile following OIDC standard claims
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ExtendedProfileDto {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub given_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub family_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nickname: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub birthdate: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locale: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub zoneinfo: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gender: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub website: Option<String>,
}

/// Complete user profile response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UserProfileResponseDto {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub primary_email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub primary_phone: Option<String>,
    pub profile: ExtendedProfileDto,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<i64>,
    pub roles: Vec<String>,
}

/// Request DTO for updating basic profile
#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateBasicProfileDto {
    #[validate(length(max = 128, message = "Name must not exceed 128 characters"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    #[validate(url(message = "Avatar must be a valid URL"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar: Option<String>,

    #[validate(
        length(min = 1, max = 128, message = "Username must be 1-128 characters"),
        regex(
            path = "*crate::shared::validation::USERNAME_REGEX",
            message = "Username must start with letter or underscore and contain only alphanumeric characters and underscores"
        )
    )]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
}

/// Request DTO for updating extended profile
#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateExtendedProfileDto {
    #[validate(length(max = 128, message = "Given name must not exceed 128 characters"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub given_name: Option<String>,

    #[validate(length(max = 128, message = "Family name must not exceed 128 characters"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub family_name: Option<String>,

    #[validate(length(max = 128, message = "Nickname must not exceed 128 characters"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nickname: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub birthdate: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub locale: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub zoneinfo: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub gender: Option<String>,

    #[validate(url(message = "Website must be a valid URL"))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub website: Option<String>,
}
