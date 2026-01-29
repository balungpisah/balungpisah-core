use crate::features::users::handlers::profile_handler;
use crate::features::users::services::UserProfileService;
use axum::{
    routing::{get, patch},
    Router,
};
use std::sync::Arc;

pub fn routes(service: Arc<UserProfileService>) -> Router {
    Router::new()
        .route("/api/users/me", get(profile_handler::get_profile))
        .route(
            "/api/users/me",
            patch(profile_handler::update_basic_profile),
        )
        .route(
            "/api/users/me/profile",
            patch(profile_handler::update_extended_profile),
        )
        .with_state(service)
}
