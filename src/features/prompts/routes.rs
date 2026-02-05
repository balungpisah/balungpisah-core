use axum::{
    routing::{get, post},
    Router,
};
use std::sync::Arc;

use crate::features::prompts::{handlers, services::PromptService};

/// Create admin routes for prompt management (super admin only)
pub fn admin_routes(service: Arc<PromptService>) -> Router {
    Router::new()
        .route("/api/admin/prompts", post(handlers::create_prompt))
        .route("/api/admin/prompts", get(handlers::list_prompts))
        .route(
            "/api/admin/prompts/{id}",
            get(handlers::get_prompt)
                .put(handlers::update_prompt)
                .delete(handlers::delete_prompt),
        )
        .with_state(service)
}
