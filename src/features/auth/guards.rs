//! Role-based authorization guards for the application.
//!
//! These guards extract the authenticated user and verify they have the required roles.
//!
//! Role hierarchy (from highest to lowest):
//! - super_admin: Global admin (existing)
//! - admin_curator: Can curate citizen reports and manage marketplace items
//! - official: Can claim and resolve marketplace problems
//! - citizen: Can report problems and track their reports
//!
//! Each higher role includes all permissions of lower roles:
//! - super_admin can do everything
//! - admin_curator can do everything official and citizen can do
//! - official can do everything citizen can do

use crate::core::error::AppError;
use crate::features::auth::model::AuthenticatedUser;
use axum::{extract::FromRequestParts, http::request::Parts};

/// Guard for checking if user is super admin.
///
/// Only allows users with the "super_admin" role.
///
/// # Example
/// ```ignore
/// pub async fn handler(RequireSuperAdmin(user): RequireSuperAdmin) { ... }
/// ```
#[allow(dead_code)]
pub struct RequireSuperAdmin(pub AuthenticatedUser);

impl<S> FromRequestParts<S> for RequireSuperAdmin
where
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let user = parts
            .extensions
            .get::<AuthenticatedUser>()
            .ok_or_else(|| AppError::Unauthorized("User not authenticated".to_string()))?;

        if !user.is_super_admin() {
            return Err(AppError::Forbidden(
                "Super admin access required".to_string(),
            ));
        }

        Ok(RequireSuperAdmin(user.clone()))
    }
}

/// Guard for checking if user has admin curator level access.
///
/// Allows users with "super_admin" or "admin_curator" roles.
/// Use this for report curation and marketplace management operations.
///
/// # Example
/// ```ignore
/// pub async fn handler(RequireAdminCurator(user): RequireAdminCurator) { ... }
/// ```
#[allow(dead_code)]
pub struct RequireAdminCurator(pub AuthenticatedUser);

impl<S> FromRequestParts<S> for RequireAdminCurator
where
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let user = parts
            .extensions
            .get::<AuthenticatedUser>()
            .ok_or_else(|| AppError::Unauthorized("User not authenticated".to_string()))?;

        if !user.has_admin_access() {
            return Err(AppError::Forbidden(
                "Admin curator access required".to_string(),
            ));
        }

        Ok(RequireAdminCurator(user.clone()))
    }
}

/// Guard for checking if user has official level access.
///
/// Allows users with "super_admin", "admin_curator", or "official" roles.
/// Use this for marketplace claiming and problem resolution operations.
///
/// # Example
/// ```ignore
/// pub async fn handler(RequireOfficial(user): RequireOfficial) { ... }
/// ```
#[allow(dead_code)]
pub struct RequireOfficial(pub AuthenticatedUser);

impl<S> FromRequestParts<S> for RequireOfficial
where
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let user = parts
            .extensions
            .get::<AuthenticatedUser>()
            .ok_or_else(|| AppError::Unauthorized("User not authenticated".to_string()))?;

        if !user.has_official_access() {
            return Err(AppError::Forbidden("Official access required".to_string()));
        }

        Ok(RequireOfficial(user.clone()))
    }
}

/// Guard for checking if user has citizen level access.
///
/// Allows users with "super_admin", "admin_curator", "official", or "citizen" roles.
/// Use this for problem reporting and tracking operations.
///
/// # Example
/// ```ignore
/// pub async fn handler(RequireCitizen(user): RequireCitizen) { ... }
/// ```
#[allow(dead_code)]
pub struct RequireCitizen(pub AuthenticatedUser);

impl<S> FromRequestParts<S> for RequireCitizen
where
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let user = parts
            .extensions
            .get::<AuthenticatedUser>()
            .ok_or_else(|| AppError::Unauthorized("User not authenticated".to_string()))?;

        if !user.has_citizen_access() {
            return Err(AppError::Forbidden("Citizen access required".to_string()));
        }

        Ok(RequireCitizen(user.clone()))
    }
}
