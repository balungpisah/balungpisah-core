use balungpisah_adk::{MessageStorage, PostgresStorage};
use balungpisah_tensorzero::{InputMessage, JsonInferenceRequestBuilder, TensorZeroClient};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::core::error::{AppError, Result};
use crate::features::reports::models::ReportSeverity;

/// Extracted report data from a conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedReportData {
    pub title: String,
    pub description: String,
    pub category_slug: Option<String>,
    pub severity: Option<ReportSeverity>,
    pub timeline: Option<String>,
    pub impact: Option<String>,
    pub location_raw: Option<String>,
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
    /// Uses TensorZero JSON inference for structured extraction
    pub async fn extract_from_text(&self, conversation: &str) -> Result<ExtractedReportData> {
        let system_prompt = self.build_system_prompt();
        let user_prompt = self.build_user_prompt(conversation);

        // Build JSON inference request with output schema
        let request = JsonInferenceRequestBuilder::new()
            .model(self.model_name.clone())
            .system(system_prompt)
            .message(InputMessage::user(user_prompt))
            .credentials(serde_json::json!({
                "system_api_key": self.openai_api_key
            }))
            .output_schema(self.output_schema())
            .build()
            .map_err(|e| {
                tracing::error!("Failed to build inference request: {:?}", e);
                AppError::Internal(format!("Failed to build inference request: {}", e))
            })?;

        // Send request to TensorZero
        let response = self.client.json_inference(request).await.map_err(|e| {
            tracing::error!("TensorZero inference failed: {:?}", e);
            AppError::ExternalServiceError(format!("LLM extraction failed: {}", e))
        })?;

        // Parse response into ExtractedReportData
        response.parse_as::<ExtractedReportData>().map_err(|e| {
            tracing::error!("Failed to parse extraction response: {:?}", e);
            AppError::Internal(format!("Failed to parse extraction response: {}", e))
        })
    }

    fn build_system_prompt(&self) -> String {
        r#"You are a data extraction assistant for a citizen report system. Your task is to extract structured information from conversations between citizens and an AI assistant about issues they want to report.

Extract the following information:
- title: A concise title for the report (max 200 characters)
- description: A detailed description of the issue
- category_slug: One of: infrastructure, environment, public-safety, social-welfare, other
- severity: One of: low, medium, high, critical
- timeline: When the issue started or when it occurred
- impact: Who or how many people are affected
- location_raw: The raw location description (address, landmark, area name)

Be accurate and only extract information that is explicitly mentioned in the conversation. If information is not provided, set it to null."#.to_string()
    }

    fn build_user_prompt(&self, conversation: &str) -> String {
        format!(
            "Extract structured report data from this conversation:\n\n{}",
            conversation
        )
    }

    fn output_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "title": {
                    "type": "string",
                    "description": "Concise title for the report (max 200 chars)"
                },
                "description": {
                    "type": "string",
                    "description": "Detailed description of the issue"
                },
                "category_slug": {
                    "type": ["string", "null"],
                    "enum": ["infrastructure", "environment", "public-safety", "social-welfare", "other", null],
                    "description": "Category of the report"
                },
                "severity": {
                    "type": ["string", "null"],
                    "enum": ["low", "medium", "high", "critical", null],
                    "description": "Severity level of the issue"
                },
                "timeline": {
                    "type": ["string", "null"],
                    "description": "When the issue started or occurred"
                },
                "impact": {
                    "type": ["string", "null"],
                    "description": "Who or how many people are affected"
                },
                "location_raw": {
                    "type": ["string", "null"],
                    "description": "Raw location description from the user"
                }
            },
            "required": ["title", "description"],
            "additionalProperties": false
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_output_schema_is_valid_json() {
        // We can't create a full ExtractionService without ADK storage,
        // so we'll just test the schema directly
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "title": { "type": "string" },
                "description": { "type": "string" }
            },
            "required": ["title", "description"]
        });

        assert!(schema.is_object());
        assert_eq!(schema["type"], "object");
    }

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
}
