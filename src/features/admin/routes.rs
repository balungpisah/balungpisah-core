use std::sync::Arc;

use axum::{routing::get, Router};

use crate::features::admin::handlers;
use crate::features::admin::services::AdminService;

/// Create admin routes (all require super admin access)
pub fn routes(admin_service: Arc<AdminService>) -> Router {
    Router::new()
        // Expectations
        .route("/expectations", get(handlers::list_expectations))
        .route("/expectations/{id}", get(handlers::get_expectation))
        // Reports
        .route("/reports", get(handlers::list_reports))
        .route("/reports/{id}", get(handlers::get_report))
        // Contributors
        .route("/contributors", get(handlers::list_contributors))
        .route("/contributors/{id}", get(handlers::get_contributor))
        // Tickets
        .route("/tickets", get(handlers::list_tickets))
        .route("/tickets/{id}", get(handlers::get_ticket))
        .with_state(admin_service)
}
