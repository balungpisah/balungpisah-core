//! Contributor routes

use std::sync::Arc;

use axum::{routing::post, Router};

use crate::features::contributors::handlers;
use crate::features::contributors::services::ContributorService;

/// Create routes for the contributors feature
///
/// Note: This feature is public (no authentication required) as it's used
/// for the contributor registration form.
pub fn routes(service: Arc<ContributorService>) -> Router {
    Router::new()
        .route(
            "/api/contributors/register",
            post(handlers::register_contributor),
        )
        .with_state(service)
}
