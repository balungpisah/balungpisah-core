use balungpisah_adk::{ToolContext, ToolResult};
use serde_json::{json, Value};
use sqlx::PgPool;

use crate::features::reports::models::CreateReportJob;
use crate::features::reports::services::{ReportJobService, ReportService};

/// Minimum confidence score required for a report to be processed
const MIN_CONFIDENCE_FOR_PROCESSING: f64 = 0.7;

/// Handle the `create_report` tool call
/// Creates a report submission and queues it for background processing,
/// or closes the conversation without creating a report
pub async fn handle_create_report(args: Value, ctx: ToolContext, pool: &PgPool) -> ToolResult {
    // Extract action and confidence from arguments
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("submit");

    let confidence = args
        .get("confidence")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.5);

    // Handle "close" action - end conversation without creating a report
    if action == "close" {
        tracing::info!(
            "Conversation closed without report: user={}, confidence={}",
            ctx.external_id(),
            confidence
        );

        return ToolResult::success_json(
            &ctx.tool_call_id,
            &ctx.tool_name,
            json!({
                "success": true,
                "action": "closed",
                "message": "Percakapan ditutup. Terima kasih sudah menghubungi BalungPisah."
            }),
        );
    }

    // Handle "submit" action - create a report
    let user_id = ctx.external_id();
    let thread_id = ctx.thread_id();

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

    let reference_number = report.reference_number.as_deref().unwrap_or("UNKNOWN");

    // Check if report meets criteria for processing
    if confidence >= MIN_CONFIDENCE_FOR_PROCESSING {
        // High confidence - create job for background processing
        let job_data = CreateReportJob {
            report_id: report.id,
            confidence_score: Some(confidence),
        };

        if let Err(e) = job_service.create(&job_data).await {
            tracing::error!("Failed to create report job: {:?}", e);
            tracing::warn!(
                "Report {} created but job creation failed. Manual processing may be needed.",
                report.id
            );
        }

        tracing::info!(
            "Report submitted for processing: id={}, ref={}, user={}, confidence={}",
            report.id,
            reference_number,
            user_id,
            confidence
        );

        ToolResult::success_json(
            &ctx.tool_call_id,
            &ctx.tool_name,
            json!({
                "success": true,
                "action": "submitted",
                "reference_number": reference_number,
                "report_id": report.id,
                "will_be_processed": true,
                "message": format!(
                    "Laporan berhasil dibuat dengan nomor referensi {}. \
                     Laporan Anda akan segera diproses dan dapat dilacak menggunakan nomor referensi tersebut.",
                    reference_number
                )
            }),
        )
    } else {
        // Low confidence - reject the report without creating a job
        let reject_reason = format!(
            "Low confidence score: {:.2} (minimum required: {:.2})",
            confidence, MIN_CONFIDENCE_FOR_PROCESSING
        );

        if let Err(e) = report_service.reject(report.id, Some(&reject_reason)).await {
            tracing::error!("Failed to reject report: {:?}", e);
        }

        tracing::info!(
            "Report rejected due to low confidence: id={}, ref={}, user={}, confidence={}",
            report.id,
            reference_number,
            user_id,
            confidence
        );

        ToolResult::success_json(
            &ctx.tool_call_id,
            &ctx.tool_name,
            json!({
                "success": true,
                "action": "submitted",
                "reference_number": reference_number,
                "report_id": report.id,
                "will_be_processed": false,
                "message": format!(
                    "Laporan berhasil dibuat dengan nomor referensi {}. \
                     Laporan Anda telah dicatat namun memerlukan informasi tambahan untuk dapat diproses lebih lanjut.",
                    reference_number
                )
            }),
        )
    }
}
