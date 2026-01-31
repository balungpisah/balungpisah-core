use std::sync::Arc;

use axum::{
    extract::{Path, State},
    Json,
};
use uuid::Uuid;

use crate::core::error::Result;
use crate::features::auth::model::AuthenticatedUser;
use crate::features::tickets::dtos::TicketResponseDto;
use crate::features::tickets::services::TicketService;
use crate::shared::types::ApiResponse;

/// List user's tickets
#[utoipa::path(
    get,
    path = "/api/tickets",
    responses(
        (status = 200, description = "List of user's tickets", body = ApiResponse<Vec<TicketResponseDto>>),
    ),
    security(("bearer_auth" = [])),
    tag = "tickets"
)]
pub async fn list_tickets(
    user: AuthenticatedUser,
    State(service): State<Arc<TicketService>>,
) -> Result<Json<ApiResponse<Vec<TicketResponseDto>>>> {
    let tickets = service.list_by_user(&user.sub).await?;
    Ok(Json(ApiResponse::success(Some(tickets), None, None)))
}

/// Get ticket by ID
#[utoipa::path(
    get,
    path = "/api/tickets/{id}",
    params(
        ("id" = Uuid, Path, description = "Ticket ID")
    ),
    responses(
        (status = 200, description = "Ticket found", body = ApiResponse<TicketResponseDto>),
        (status = 404, description = "Ticket not found")
    ),
    security(("bearer_auth" = [])),
    tag = "tickets"
)]
pub async fn get_ticket(
    user: AuthenticatedUser,
    State(service): State<Arc<TicketService>>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<TicketResponseDto>>> {
    let ticket = service.get_by_id(id, &user.sub).await?;
    Ok(Json(ApiResponse::success(Some(ticket), None, None)))
}

/// Get ticket by reference number
#[utoipa::path(
    get,
    path = "/api/tickets/ref/{reference}",
    params(
        ("reference" = String, Path, description = "Ticket reference number (e.g., TKT-2026-0000001)")
    ),
    responses(
        (status = 200, description = "Ticket found", body = ApiResponse<TicketResponseDto>),
        (status = 404, description = "Ticket not found")
    ),
    security(("bearer_auth" = [])),
    tag = "tickets"
)]
pub async fn get_ticket_by_reference(
    user: AuthenticatedUser,
    State(service): State<Arc<TicketService>>,
    Path(reference): Path<String>,
) -> Result<Json<ApiResponse<TicketResponseDto>>> {
    let ticket = service.get_by_reference(&reference, &user.sub).await?;
    Ok(Json(ApiResponse::success(Some(ticket), None, None)))
}
