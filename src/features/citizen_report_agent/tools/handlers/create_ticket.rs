use balungpisah_adk::{ToolContext, ToolResult};
use serde_json::{json, Value};
use sqlx::PgPool;

use crate::features::tickets::services::TicketService;

/// Handle the `create_ticket` tool call
pub async fn handle_create_ticket(args: Value, ctx: ToolContext, pool: &PgPool) -> ToolResult {
    // Extract confidence from arguments
    let confidence = args
        .get("confidence")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.5);

    // Get user_id from thread's external_id (which is the user's sub)
    let user_id = ctx.external_id();
    let thread_id = ctx.thread_id();

    // Create ticket via service
    let ticket_service = TicketService::new(pool.clone());
    match ticket_service
        .create_from_agent(thread_id, user_id, confidence, Some("web"))
        .await
    {
        Ok(ticket) => {
            let response = json!({
                "success": true,
                "reference_number": ticket.reference_number,
                "ticket_id": ticket.ticket_id,
                "message": format!(
                    "Tiket berhasil dibuat dengan nomor referensi {}. \
                     Laporan Anda akan segera diproses.",
                    ticket.reference_number
                )
            });
            ToolResult::success_json(&ctx.tool_call_id, &ctx.tool_name, response)
        }
        Err(e) => {
            tracing::error!("Failed to create ticket: {:?}", e);
            ToolResult::error(
                &ctx.tool_call_id,
                &ctx.tool_name,
                format!("Gagal membuat tiket: {}", e),
            )
        }
    }
}
