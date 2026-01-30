use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::model::AuthenticatedUser;

/// DTO for /auth/me response
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct MeResponseDto {
    pub account_id: String,
    pub sub: String,
    /// Session UID (only present for interactive OIDC flows, not for token exchange)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_uid: Option<String>,
    pub roles: Vec<String>,
}

// Conversion from AuthenticatedUser to MeResponseDto
impl From<AuthenticatedUser> for MeResponseDto {
    fn from(user: AuthenticatedUser) -> Self {
        Self {
            account_id: user.account_id,
            sub: user.sub,
            session_uid: user.session_uid,
            roles: user.roles,
        }
    }
}
