use std::sync::Arc;

use axum::{routing::post, Router};

use crate::features::expectations::handlers;
use crate::features::expectations::services::ExpectationService;

/// Create routes for the expectations feature
///
/// Note: This feature is public (no authentication required) as it's used
/// for the landing page form.
pub fn routes(service: Arc<ExpectationService>) -> Router {
    Router::new()
        .route("/api/expectations", post(handlers::create_expectation))
        .with_state(service)
}
