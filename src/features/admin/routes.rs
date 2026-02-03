use std::sync::Arc;

use axum::{routing::get, Router};

use crate::features::admin::handlers;
use crate::features::admin::services::AdminService;

/// Create admin routes (all require super admin access)
pub fn routes(admin_service: Arc<AdminService>) -> Router {
    Router::new()
        .route("/expectations", get(handlers::list_expectations))
        .route("/reports", get(handlers::list_reports))
        .route("/contributors", get(handlers::list_contributors))
        .route("/tickets", get(handlers::list_tickets))
        .with_state(admin_service)
}
