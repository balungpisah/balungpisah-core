use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

/// Request DTO for creating an expectation
#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateExpectationDto {
    /// Optional name of the submitter
    #[validate(length(max = 255, message = "Name must not exceed 255 characters"))]
    pub name: Option<String>,

    /// Optional email for follow-up
    #[validate(email(message = "Invalid email format"))]
    pub email: Option<String>,

    /// The expectation text (required)
    #[validate(length(min = 1, max = 5000, message = "Expectation must be 1-5000 characters"))]
    pub expectation: String,
}

/// Response DTO for expectation
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ExpectationResponseDto {
    pub id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    pub expectation: String,
    pub created_at: DateTime<Utc>,
}
