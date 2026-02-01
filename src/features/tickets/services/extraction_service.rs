use balungpisah_adk::{MessageStorage, PostgresStorage};
use balungpisah_tensorzero::{InferenceRequestBuilder, InputMessage, TensorZeroClient};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::core::error::{AppError, Result};
use crate::features::reports::models::ReportSeverity;
use crate::shared::llm::{parse_with_fallback, LlmResponse};

fn default_true() -> bool {
    true
}

/// Extracted report data from a conversation
#[derive(Debug, Clone, Serialize, Deserialize, Default, JsonSchema)]
#[schemars(title = "ExtractedReportData")]
pub struct ExtractedReportData {
    #[schemars(description = "Concise title for the report (max 200 characters)")]
    pub title: String,

    #[schemars(description = "Detailed description of the issue")]
    pub description: String,

    #[schemars(
        description = "Category slug: infrastructure, environment, public-safety, social-welfare, or other"
    )]
    pub category_slug: Option<String>,

    #[schemars(description = "Severity level: low, medium, high, or critical")]
    pub severity: Option<ReportSeverity>,

    #[schemars(description = "When the issue started or occurred")]
    pub timeline: Option<String>,

    #[schemars(description = "Who or how many people are affected")]
    pub impact: Option<String>,

    #[schemars(
        description = "Raw location description from the user (address, landmark, area name)"
    )]
    pub location_raw: Option<String>,

    /// Whether the LLM extraction was successful
    #[serde(default = "default_true")]
    #[schemars(skip)]
    pub is_llm_success: bool,

    /// Error message if LLM extraction failed
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(skip)]
    pub llm_error_message: Option<String>,
}

impl LlmResponse for ExtractedReportData {
    fn mark_as_fallback(&mut self, error_message: String) {
        self.is_llm_success = false;
        self.llm_error_message = Some(error_message);
        // Set reasonable defaults for required fields
        if self.title.is_empty() {
            self.title = "Laporan Warga".to_string();
        }
        if self.description.is_empty() {
            self.description = "Deskripsi tidak tersedia - ekstraksi gagal".to_string();
        }
    }

    fn is_success(&self) -> bool {
        self.is_llm_success
    }
}

/// Service for extracting structured data from conversations using TensorZero
pub struct ExtractionService {
    client: TensorZeroClient,
    openai_api_key: String,
    model_name: String,
    adk_storage: Arc<PostgresStorage>,
}

impl ExtractionService {
    pub fn new(
        tensorzero_url: &str,
        openai_api_key: String,
        model_name: String,
        adk_storage: Arc<PostgresStorage>,
    ) -> Result<Self> {
        let client = TensorZeroClient::new(tensorzero_url).map_err(|e| {
            tracing::error!("Failed to create TensorZero client: {:?}", e);
            AppError::Internal(format!("Failed to create TensorZero client: {}", e))
        })?;

        Ok(Self {
            client,
            openai_api_key,
            model_name,
            adk_storage,
        })
    }

    /// Extract structured data from a conversation thread
    ///
    /// Fetches conversation from ADK storage and extracts structured data using LLM
    pub async fn extract_from_thread(&self, thread_id: Uuid) -> Result<ExtractedReportData> {
        // Fetch messages from ADK storage
        let messages = self
            .adk_storage
            .get_thread_messages(thread_id)
            .await
            .map_err(|e| {
                tracing::error!("Failed to fetch thread messages: {:?}", e);
                AppError::Internal(format!("Failed to fetch conversation: {}", e))
            })?;

        if messages.is_empty() {
            return Err(AppError::BadRequest(
                "Cannot extract from empty conversation".to_string(),
            ));
        }

        // Convert messages to text format for extraction
        let conversation_text = self.format_conversation(&messages);

        tracing::debug!(
            "Extracting from thread {} ({} messages, {} chars)",
            thread_id,
            messages.len(),
            conversation_text.len()
        );

        // Extract structured data
        self.extract_from_text(&conversation_text).await
    }

    /// Format messages into a conversation transcript
    fn format_conversation(&self, messages: &[balungpisah_adk::Message]) -> String {
        messages
            .iter()
            .map(|m| {
                let role = match m.role {
                    balungpisah_adk::Role::User => "User",
                    balungpisah_adk::Role::Assistant => "Assistant",
                };
                format!("{}: {}", role, m.text())
            })
            .collect::<Vec<_>>()
            .join("\n\n")
    }

    /// Extract structured data from raw conversation text
    ///
    /// Uses TensorZero inference with JSON schema embedded in system prompt.
    /// Uses graceful fallback parsing - never fails, returns default values on parse errors.
    pub async fn extract_from_text(&self, conversation: &str) -> Result<ExtractedReportData> {
        let system_prompt = Self::build_system_prompt();
        let user_prompt = Self::build_user_prompt(conversation);

        // Build inference request with schema in system prompt (avoiding output_schema bug)
        let request = InferenceRequestBuilder::new()
            .model(self.model_name.clone())
            .system(system_prompt)
            .message(InputMessage::user(user_prompt))
            .credentials(serde_json::json!({
                "system_api_key": self.openai_api_key
            }))
            .build()
            .map_err(|e| {
                tracing::error!("Failed to build inference request: {:?}", e);
                AppError::Internal(format!("Failed to build inference request: {}", e))
            })?;

        // Send request to TensorZero
        let response = self.client.inference(request).await.map_err(|e| {
            tracing::error!("TensorZero inference failed: {:?}", e);
            AppError::ExternalServiceError(format!("LLM extraction failed: {}", e))
        })?;

        // Get text content and parse with fallback
        let text = response.text();

        tracing::debug!(
            "Raw LLM response (first 500 chars): {}",
            text.chars().take(500).collect::<String>()
        );

        // Use the reusable parser with graceful fallback
        let extracted: ExtractedReportData = parse_with_fallback(&text);

        if !extracted.is_success() {
            tracing::warn!(
                "LLM extraction used fallback: {:?}",
                extracted.llm_error_message
            );
        }

        Ok(extracted)
    }

    fn build_system_prompt() -> String {
        format!(
            r#"You are a data extraction assistant for a citizen report system. Your task is to extract structured information from conversations between citizens and an AI assistant about issues they want to report.

Extract the following information:
- title: A concise title for the report (max 200 characters)
- description: A detailed description of the issue
- category_slug: One of: infrastructure, environment, public-safety, social-welfare, other
- severity: One of: low, medium, high, critical
- timeline: When the issue started or when it occurred
- impact: Who or how many people are affected
- location_raw: The raw location description (address, landmark, area name)

Be accurate and only extract information that is explicitly mentioned in the conversation. If information is not provided, set it to null.

You MUST respond with valid JSON that conforms to this schema:
```json
{}
```

Respond ONLY with the JSON object, no additional text or explanation."#,
            ExtractedReportData::json_schema_string()
        )
    }

    fn build_user_prompt(conversation: &str) -> String {
        format!(
            "Extract structured report data from this conversation:\n\n{}",
            conversation
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extracted_report_data_deserialize() {
        let json = r#"{
            "title": "Pothole on Main Street",
            "description": "Large pothole causing traffic issues",
            "category_slug": "infrastructure",
            "severity": "medium",
            "timeline": "Started last week",
            "impact": "Affects daily commuters",
            "location_raw": "Jl. Sudirman No. 123"
        }"#;

        let data: ExtractedReportData = serde_json::from_str(json).unwrap();
        assert_eq!(data.title, "Pothole on Main Street");
        assert_eq!(data.category_slug, Some("infrastructure".to_string()));
        assert!(data.is_success()); // Default is true
    }

    #[test]
    fn test_extracted_report_data_with_nulls() {
        let json = r#"{
            "title": "Test Report",
            "description": "Test description",
            "category_slug": null,
            "severity": null,
            "timeline": null,
            "impact": null,
            "location_raw": null
        }"#;

        let data: ExtractedReportData = serde_json::from_str(json).unwrap();
        assert_eq!(data.title, "Test Report");
        assert!(data.category_slug.is_none());
        assert!(data.severity.is_none());
    }

    #[test]
    fn test_parse_with_fallback_valid_json() {
        let input = r#"{"title": "Test", "description": "Test desc"}"#;

        let result: ExtractedReportData = parse_with_fallback(input);

        assert!(result.is_success());
        assert_eq!(result.title, "Test");
        assert_eq!(result.description, "Test desc");
    }

    #[test]
    fn test_parse_with_fallback_markdown_code_block() {
        let input = r#"Here's the result:

```json
{
    "title": "Markdown Test",
    "description": "From code block",
    "category_slug": "infrastructure"
}
```"#;

        let result: ExtractedReportData = parse_with_fallback(input);

        assert!(result.is_success());
        assert_eq!(result.title, "Markdown Test");
        assert_eq!(result.category_slug, Some("infrastructure".to_string()));
    }

    #[test]
    fn test_parse_with_fallback_with_trailing_comma() {
        let input = r#"{"title": "Test", "description": "Desc",}"#;

        let result: ExtractedReportData = parse_with_fallback(input);

        assert!(result.is_success());
        assert_eq!(result.title, "Test");
    }

    #[test]
    fn test_parse_with_fallback_invalid_returns_fallback() {
        let input = "This is not JSON at all";

        let result: ExtractedReportData = parse_with_fallback(input);

        assert!(!result.is_success());
        assert!(result.llm_error_message.is_some());
        // Fallback sets default values
        assert_eq!(result.title, "Laporan Warga");
        assert_eq!(
            result.description,
            "Deskripsi tidak tersedia - ekstraksi gagal"
        );
    }

    #[test]
    fn test_llm_response_trait_mark_as_fallback() {
        let mut data = ExtractedReportData::default();
        data.mark_as_fallback("Test error".to_string());

        assert!(!data.is_success());
        assert_eq!(data.llm_error_message, Some("Test error".to_string()));
        assert_eq!(data.title, "Laporan Warga"); // Default fallback title
    }

    #[test]
    fn test_json_schema_string_generation() {
        let schema = ExtractedReportData::json_schema_string();

        // Should contain field descriptions from schemars attributes
        assert!(schema.contains("title"));
        assert!(schema.contains("description"));
        assert!(schema.contains("category_slug"));
        assert!(schema.contains("severity"));

        // Should NOT contain internal fields (marked with #[schemars(skip)])
        assert!(!schema.contains("is_llm_success"));
        assert!(!schema.contains("llm_error_message"));
    }

    #[test]
    fn test_build_system_prompt_contains_schema() {
        let prompt = ExtractionService::build_system_prompt();

        // Should contain the schema
        assert!(prompt.contains("title"));
        assert!(prompt.contains("description"));
        assert!(prompt.contains("category_slug"));

        // Should contain instructions
        assert!(prompt.contains("data extraction assistant"));
        assert!(prompt.contains("JSON"));
    }
}
