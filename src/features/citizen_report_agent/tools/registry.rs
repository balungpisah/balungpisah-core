use balungpisah_adk::{FnToolExecutor, ToolContext, ToolRegistry};
use serde_json::Value;
use sqlx::PgPool;
use std::sync::Arc;

use super::definitions::create_report_tool;
use super::handlers::handle_create_report;

/// Create a tool registry with all citizen report agent tools
pub fn create_tool_registry(pool: Arc<PgPool>) -> ToolRegistry {
    let mut registry = ToolRegistry::new();

    // Register create_report tool (new workflow)
    let pool_clone = Arc::clone(&pool);
    registry.register(FnToolExecutor::new(
        create_report_tool(),
        move |args: Value, ctx: ToolContext| {
            let pool = Arc::clone(&pool_clone);
            async move { handle_create_report(args, ctx, &pool).await }
        },
    ));

    registry
}
