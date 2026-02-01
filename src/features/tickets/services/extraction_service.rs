use balungpisah_adk::{MessageStorage, PostgresStorage};
use balungpisah_tensorzero::{InferenceRequestBuilder, InputMessage, TensorZeroClient};
use regex::Regex;
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
    /// Uses TensorZero inference with JSON schema embedded in system prompt
    pub async fn extract_from_text(&self, conversation: &str) -> Result<ExtractedReportData> {
        let system_prompt = self.build_system_prompt();
        let user_prompt = self.build_user_prompt(conversation);

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

        // Get text content and parse as JSON
        let text = response.text();

        tracing::debug!(
            "Raw LLM response (first 500 chars): {}",
            text.chars().take(500).collect::<String>()
        );

        // Try to extract JSON from the response (handle markdown code blocks)
        let json_str = match Self::extract_json_string(&text) {
            Ok(s) => s,
            Err(e) => {
                tracing::error!(
                    "Failed to extract JSON from response: {}, raw text: {}",
                    e,
                    text
                );
                return Err(AppError::Internal(format!(
                    "Failed to extract JSON from response: {}",
                    e
                )));
            }
        };

        tracing::debug!(
            "Extracted JSON (first 500 chars): {}",
            json_str.chars().take(500).collect::<String>()
        );

        // Try parsing directly first (fast path)
        match serde_json::from_str::<ExtractedReportData>(&json_str) {
            Ok(parsed) => {
                tracing::debug!("JSON parsed successfully (fast path)");
                Ok(parsed)
            }
            Err(direct_err) => {
                tracing::debug!(
                    "Direct JSON parsing failed: {:?}, attempting quick fixes",
                    direct_err
                );

                // Apply quick fixes
                let mut fixed_json = Self::fix_js_string_concatenation(&json_str);
                fixed_json = Self::fix_trailing_commas(&fixed_json);

                // Try parsing after fixes
                serde_json::from_str::<ExtractedReportData>(&fixed_json).map_err(|e| {
                    tracing::error!(
                        "Failed to parse extraction response after fixes: {:?}, raw text: {}",
                        e,
                        text
                    );
                    AppError::Internal(format!("Failed to parse extraction response: {}", e))
                })
            }
        }
    }

    /// Extract JSON string from text (handles multiple formats)
    ///
    /// Tries in order:
    /// 1. JSON in markdown code block: ```json ... ```
    /// 2. Plain JSON starting with {
    /// 3. JSON embedded anywhere in text (find { to })
    fn extract_json_string(text: &str) -> std::result::Result<String, String> {
        // Try 1: Markdown code block with json
        if text.contains("```json") {
            return text
                .split("```json")
                .nth(1)
                .and_then(|s| s.split("```").next())
                .map(|s| s.trim().to_string())
                .ok_or_else(|| "Failed to extract JSON from markdown code block".to_string());
        }

        // Try 2: Generic markdown code block
        if text.contains("```") {
            if let Some(start) = text.find("```") {
                let block_start = start + 3;
                // Skip optional language identifier on the same line
                if let Some(newline_offset) = text[block_start..].find('\n') {
                    let json_start = block_start + newline_offset + 1;
                    if let Some(end_offset) = text[json_start..].find("```") {
                        return Ok(text[json_start..json_start + end_offset].trim().to_string());
                    }
                }
            }
        }

        // Try 3: Plain JSON starting with {
        let trimmed = text.trim();
        if trimmed.starts_with('{') {
            return Ok(trimmed.to_string());
        }

        // Try 4: Embedded JSON (find first { to last })
        let start = text
            .find('{')
            .ok_or_else(|| "No JSON object found in response".to_string())?;

        let end = text
            .rfind('}')
            .ok_or_else(|| "Incomplete JSON object in response".to_string())?;

        if start < end {
            Ok(text[start..=end].to_string())
        } else {
            Err("Invalid JSON boundaries in response".to_string())
        }
    }

    /// Fix trailing commas in JSON (common LLM mistake)
    fn fix_trailing_commas(json_str: &str) -> String {
        let re = Regex::new(r",(\s*[}\]])").unwrap();
        re.replace_all(json_str, "$1").to_string()
    }

    /// Fix JavaScript string concatenation which is invalid in JSON
    ///
    /// LLMs sometimes output: `"str1" + "str2"` which is invalid JSON.
    /// This merges them into: `"str1str2"`
    fn fix_js_string_concatenation(json_str: &str) -> String {
        let re = Regex::new(r#""\s*\+\s*""#).unwrap();
        re.replace_all(json_str, "").to_string()
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

Be accurate and only extract information that is explicitly mentioned in the conversation. If information is not provided, set it to null.

You MUST respond with valid JSON that conforms to this schema:
```json
{
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
}
```

Respond ONLY with the JSON object, no additional text or explanation."#.to_string()
    }

    fn build_user_prompt(&self, conversation: &str) -> String {
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
    fn test_extract_json_string_with_json_code_block() {
        let response = r#"Here is the extracted data:

```json
{
    "title": "Test",
    "description": "Test desc"
}
```

That's the result."#;

        let json = ExtractionService::extract_json_string(response).unwrap();
        assert!(json.starts_with('{'));
        assert!(json.ends_with('}'));
        assert!(json.contains("\"title\""));
    }

    #[test]
    fn test_extract_json_string_with_generic_code_block() {
        let response = r#"```
{
    "title": "Test",
    "description": "Test desc"
}
```"#;

        let json = ExtractionService::extract_json_string(response).unwrap();
        assert!(json.starts_with('{'));
        assert!(json.ends_with('}'));
    }

    #[test]
    fn test_extract_json_string_plain_json() {
        let response = r#"{"title": "Test", "description": "Test desc"}"#;

        let json = ExtractionService::extract_json_string(response).unwrap();
        assert_eq!(json, response);
    }

    #[test]
    fn test_extract_json_string_with_whitespace() {
        let response = r#"

{"title": "Test", "description": "Test desc"}

"#;

        let json = ExtractionService::extract_json_string(response).unwrap();
        assert!(json.starts_with('{'));
        assert!(json.ends_with('}'));
    }

    #[test]
    fn test_extract_json_string_embedded() {
        let response =
            "Some text before {\"title\": \"Test\", \"description\": \"desc\"} some text after";

        let json = ExtractionService::extract_json_string(response).unwrap();
        assert_eq!(json, r#"{"title": "Test", "description": "desc"}"#);
    }

    #[test]
    fn test_extract_json_string_no_json() {
        let response = "No JSON here at all!";

        let result = ExtractionService::extract_json_string(response);
        assert!(result.is_err());
    }

    #[test]
    fn test_fix_trailing_commas() {
        // Should remove trailing comma before }
        let input = r#"{"name": "John", "age": 30,}"#;
        let fixed = ExtractionService::fix_trailing_commas(input);
        assert_eq!(fixed, r#"{"name": "John", "age": 30}"#);

        // Should remove trailing comma before ]
        let input2 = r#"{"items": [1, 2, 3,]}"#;
        let fixed2 = ExtractionService::fix_trailing_commas(input2);
        assert_eq!(fixed2, r#"{"items": [1, 2, 3]}"#);
    }

    #[test]
    fn test_fix_js_string_concatenation() {
        // Basic concatenation
        let input = r#"{"text": "hello" + "world"}"#;
        let fixed = ExtractionService::fix_js_string_concatenation(input);
        assert_eq!(fixed, r#"{"text": "helloworld"}"#);

        // Multiple concatenations
        let input2 = r#"{"msg": "a" + "b" + "c"}"#;
        let fixed2 = ExtractionService::fix_js_string_concatenation(input2);
        assert_eq!(fixed2, r#"{"msg": "abc"}"#);
    }
}
