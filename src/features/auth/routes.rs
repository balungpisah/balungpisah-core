use crate::features::auth::handler;
use crate::features::auth::service::AuthService;
use axum::{routing::get, Router};
use std::sync::Arc;

pub fn routes(service: Arc<AuthService>) -> Router {
    Router::new()
        .route("/api/auth/me", get(handler::get_me))
        .with_state(service)
}
