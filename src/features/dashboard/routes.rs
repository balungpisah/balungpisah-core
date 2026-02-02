use axum::{routing::get, Router};
use std::sync::Arc;

use crate::features::dashboard::handlers;
use crate::features::dashboard::services::DashboardService;

/// Create public dashboard routes
pub fn routes(dashboard_service: Arc<DashboardService>) -> Router {
    Router::new()
        // Summary
        .route("/api/dashboard/summary", get(handlers::get_summary))
        // Reports listing
        .route("/api/dashboard/reports", get(handlers::list_reports))
        .route("/api/dashboard/reports/{id}", get(handlers::get_report))
        // Grouped views
        .route("/api/dashboard/by-location", get(handlers::get_by_location))
        .route("/api/dashboard/by-category", get(handlers::get_by_category))
        .route("/api/dashboard/by-tag", get(handlers::get_by_tag))
        // Recent and map
        .route("/api/dashboard/recent", get(handlers::get_recent))
        .route("/api/dashboard/map", get(handlers::get_map))
        .route("/api/dashboard/map-data", get(handlers::get_map_data))
        .with_state(dashboard_service)
}
