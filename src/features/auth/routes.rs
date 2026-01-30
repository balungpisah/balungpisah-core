use crate::features::auth::handlers;
use crate::features::auth::services::AuthService;
use axum::{
    routing::{get, post},
    Router,
};
use std::sync::Arc;

/// Public auth routes (no authentication required)
pub fn public_routes(service: Arc<AuthService>) -> Router {
    Router::new()
        .route("/api/auth/register", post(handlers::register))
        .route("/api/auth/login", post(handlers::login))
        .route("/api/auth/refresh", post(handlers::refresh_token))
        .with_state(service)
}

/// Protected auth routes (require JWT authentication)
pub fn protected_routes(service: Arc<AuthService>) -> Router {
    Router::new()
        .route("/api/auth/me", get(handlers::get_me))
        .with_state(service)
}
