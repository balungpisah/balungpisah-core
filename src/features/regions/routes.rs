use std::sync::Arc;

use axum::{routing::get, Router};

use crate::features::regions::handlers;
use crate::features::regions::services::RegionService;

/// Create routes for the regions feature
pub fn routes(service: Arc<RegionService>) -> Router {
    Router::new()
        // Province routes
        .route("/api/regions/provinces", get(handlers::list_provinces))
        .route("/api/regions/provinces/{code}", get(handlers::get_province))
        .route(
            "/api/regions/provinces/{code}/regencies",
            get(handlers::list_regencies_by_province),
        )
        // Regency routes (search endpoint must come before {code} route)
        .route("/api/regions/regencies", get(handlers::search_regencies))
        .route("/api/regions/regencies/{code}", get(handlers::get_regency))
        .route(
            "/api/regions/regencies/{code}/districts",
            get(handlers::list_districts_by_regency),
        )
        // District routes (search endpoint must come before {code} route)
        .route("/api/regions/districts", get(handlers::search_districts))
        .route("/api/regions/districts/{code}", get(handlers::get_district))
        .route(
            "/api/regions/districts/{code}/villages",
            get(handlers::list_villages_by_district),
        )
        // Village routes (search endpoint must come before {code} route)
        .route("/api/regions/villages", get(handlers::search_villages))
        .route("/api/regions/villages/{code}", get(handlers::get_village))
        .with_state(service)
}
