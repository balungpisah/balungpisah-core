use std::sync::Arc;

use axum::{routing::get, Router};

use crate::features::reports::handlers::{self, ReportState};
use crate::features::reports::services::ReportService;

/// Create routes for the reports feature
///
/// Protected routes require authentication
pub fn routes(report_service: Arc<ReportService>) -> Router {
    let state = ReportState { report_service };

    Router::new()
        // Protected routes (require auth middleware to be applied by caller)
        .route("/api/reports", get(handlers::list_reports))
        .route("/api/reports/{id}", get(handlers::get_report))
        .route(
            "/api/reports/{id}/status",
            axum::routing::patch(handlers::update_report_status),
        )
        .with_state(state)
}
