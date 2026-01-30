use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AuthenticatedUser {
    pub account_id: String,
    pub sub: String,
    /// Session UID (only present for interactive OIDC flows, not for token exchange)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_uid: Option<String>,
    pub roles: Vec<String>,
}

impl AuthenticatedUser {
    /// Check if user has a specific role
    pub fn has_role(&self, role: &str) -> bool {
        self.roles.iter().any(|r| r == role)
    }

    /// Check if user is super admin
    pub fn is_super_admin(&self) -> bool {
        self.has_role("super_admin")
    }

    /// Check if user is admin curator (can curate citizen reports, manage marketplace)
    #[allow(dead_code)]
    pub fn is_admin_curator(&self) -> bool {
        self.has_role("admin_curator")
    }

    /// Check if user is citizen (can report problems, track reports)
    #[allow(dead_code)]
    pub fn is_citizen(&self) -> bool {
        self.has_role("citizen")
    }

    /// Check if user is official (can claim and resolve problems)
    #[allow(dead_code)]
    pub fn is_official(&self) -> bool {
        self.has_role("official")
    }

    /// Check if user has admin-level access (super_admin or admin_curator)
    #[allow(dead_code)]
    pub fn has_admin_access(&self) -> bool {
        self.is_super_admin() || self.is_admin_curator()
    }

    /// Check if user has official-level access (super_admin, admin_curator, or official)
    #[allow(dead_code)]
    pub fn has_official_access(&self) -> bool {
        self.has_admin_access() || self.is_official()
    }

    /// Check if user has citizen-level access (any authenticated user)
    /// All roles can access citizen features
    #[allow(dead_code)]
    pub fn has_citizen_access(&self) -> bool {
        self.has_admin_access() || self.is_official() || self.is_citizen()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomClaims {
    #[serde(rename = "type")]
    pub token_type: String,
    #[serde(default)]
    pub permissions: Vec<String>,
    pub roles: Vec<String>,
}
