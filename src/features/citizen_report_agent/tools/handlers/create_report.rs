use balungpisah_adk::{ToolContext, ToolResult};
use serde_json::{json, Value};
use sqlx::PgPool;

use crate::features::reports::models::CreateReportJob;
use crate::features::reports::services::{ReportJobService, ReportService};

/// Handle the `create_report` tool call
/// Creates a report submission and queues it for background processing
pub async fn handle_create_report(args: Value, ctx: ToolContext, pool: &PgPool) -> ToolResult {
    // Extract confidence from arguments
    let confidence = args
        .get("confidence")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.5);

    // Get user_id from thread's external_id (which is the user's sub)
    let user_id = ctx.external_id();
    let thread_id = ctx.thread_id();

    // Create services
    let report_service = ReportService::new(pool.clone());
    let job_service = ReportJobService::new(pool.clone());

    // Create report submission with auto-generated reference number
    let report = match report_service
        .create_submission_auto_ref(thread_id, user_id, Some("web"))
        .await
    {
        Ok(r) => r,
        Err(e) => {
            tracing::error!("Failed to create report submission: {:?}", e);
            return ToolResult::error(
                &ctx.tool_call_id,
                &ctx.tool_name,
                format!("Gagal membuat laporan: {}", e),
            );
        }
    };

    // Create job for background processing
    let job_data = CreateReportJob {
        report_id: report.id,
        confidence_score: Some(confidence),
    };

    if let Err(e) = job_service.create(&job_data).await {
        tracing::error!("Failed to create report job: {:?}", e);
        // Report was created but job failed - still return success to user
        // The report can be manually processed later
        tracing::warn!(
            "Report {} created but job creation failed. Manual processing may be needed.",
            report.id
        );
    }

    let reference_number = report.reference_number.as_deref().unwrap_or("UNKNOWN");

    let response = json!({
        "success": true,
        "reference_number": reference_number,
        "report_id": report.id,
        "message": format!(
            "Laporan berhasil dibuat dengan nomor referensi {}. \
             Laporan Anda akan segera diproses dan dapat dilacak menggunakan nomor referensi tersebut.",
            reference_number
        )
    });

    tracing::info!(
        "Report created: id={}, ref={}, user={}",
        report.id,
        reference_number,
        user_id
    );

    ToolResult::success_json(&ctx.tool_call_id, &ctx.tool_name, response)
}
