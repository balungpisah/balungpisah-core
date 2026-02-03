use balungpisah_adk::ToolDefinition;

/// Create the `create_report` tool definition (new workflow)
pub fn create_report_tool() -> ToolDefinition {
    ToolDefinition::builder("create_report")
        .description(
            "End the conversation by submitting a report or closing without a report. \
             Use 'submit' when the citizen has provided a reportable issue with sufficient details. \
             Use 'close' when there is no valid report (spam, off-topic, inappropriate content, or user abandoned).",
        )
        .string_param(
            "action",
            "What to do: 'submit' to create a report, 'close' to end without a report.",
        )
        .number_param(
            "confidence",
            "Your confidence level (0.0-1.0). For 'submit': how certain this is a complete, actionable report (>=0.7 to process). \
             For 'close': how certain there is no valid report.",
        )
        .build()
}

/// Create the `create_ticket` tool definition (legacy - kept for compatibility)
#[allow(dead_code)]
pub fn create_ticket_tool() -> ToolDefinition {
    ToolDefinition::builder("create_ticket")
        .description(
            "Create a ticket when you have gathered sufficient information from the citizen. \
             Call this ONLY when you are confident the conversation contains enough details \
             about the problem (what happened, where, when). The ticket will be processed \
             in the background to extract structured data. Returns a reference number for \
             the citizen to track their report.",
        )
        .number_param(
            "confidence",
            "Your confidence that the conversation has sufficient information (0.0-1.0). \
             Use 0.7+ when you have clear problem description, location, and timing.",
        )
        .build()
}
