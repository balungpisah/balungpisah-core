use balungpisah_adk::ToolDefinition;

/// Create the `create_report` tool definition
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
