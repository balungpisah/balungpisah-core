use std::sync::Arc;

use axum::{routing::get, Router};

use crate::features::reports::handlers::{self, ReportState};
use crate::features::reports::services::{ClusteringService, ReportService};

/// Create routes for the reports feature
///
/// Protected routes require authentication
/// Cluster routes are public
pub fn routes(
    report_service: Arc<ReportService>,
    clustering_service: Arc<ClusteringService>,
) -> Router {
    let state = ReportState {
        report_service,
        clustering_service,
    };

    Router::new()
        // Protected routes (require auth middleware to be applied by caller)
        .route("/api/reports", get(handlers::list_reports))
        .route("/api/reports/{id}", get(handlers::get_report))
        .route(
            "/api/reports/{id}/status",
            axum::routing::patch(handlers::update_report_status),
        )
        // Public routes
        .route("/api/reports/clusters", get(handlers::list_clusters))
        .route("/api/reports/clusters/{id}", get(handlers::get_cluster))
        .with_state(state)
}
