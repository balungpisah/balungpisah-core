use std::sync::Arc;

use axum::{routing::get, Router};

use crate::features::categories::handlers;
use crate::features::categories::services::CategoryService;

/// Create routes for the categories feature
///
/// Note: This feature is public (no authentication required)
pub fn routes(service: Arc<CategoryService>) -> Router {
    Router::new()
        .route("/api/categories", get(handlers::list_categories))
        .route("/api/categories/{slug}", get(handlers::get_category))
        .with_state(service)
}
