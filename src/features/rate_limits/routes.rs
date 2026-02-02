use std::sync::Arc;

use axum::{routing::get, Router};

use super::handlers::{get_rate_limit_config, list_rate_limit_configs, update_rate_limit_config};
use super::services::RateLimitConfigService;

/// Create admin routes for rate limit configuration (super admin only)
pub fn admin_routes(config_service: Arc<RateLimitConfigService>) -> Router {
    Router::new()
        .route("/api/admin/rate-limits", get(list_rate_limit_configs))
        .route(
            "/api/admin/rate-limits/{key}",
            get(get_rate_limit_config).put(update_rate_limit_config),
        )
        .with_state(config_service)
}
