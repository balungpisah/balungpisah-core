use std::sync::Arc;

use axum::{routing::get, Router};

use crate::features::tickets::handlers;
use crate::features::tickets::services::TicketService;

/// Create routes for the tickets feature
///
/// Note: This feature requires authentication
pub fn routes(service: Arc<TicketService>) -> Router {
    Router::new()
        .route("/api/tickets", get(handlers::list_tickets))
        .route("/api/tickets/{id}", get(handlers::get_ticket))
        .route(
            "/api/tickets/ref/{reference}",
            get(handlers::get_ticket_by_reference),
        )
        .with_state(service)
}
