use balungpisah_adk::{MessageStorage, PostgresStorage};
use balungpisah_tensorzero::{InferenceRequestBuilder, InputMessage, TensorZeroClient};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

use crate::core::error::{AppError, Result};
use crate::features::reports::models::{ReportSeverity, ReportTagType};
use crate::shared::llm::{parse_with_fallback, LlmResponse};
use crate::shared::prompts::render_extraction_prompt;

fn default_true() -> bool {
    true
}

/// Category information for LLM context
#[derive(Debug, Clone)]
struct CategoryInfo {
    name: String,
    slug: String,
    description: Option<String>,
}

/// Extracted category with severity
#[derive(Debug, Clone, Serialize, Deserialize, Default, JsonSchema)]
pub struct ExtractedCategory {
    #[schemars(
        description = "Category slug: infrastructure, environment, public-safety, social-welfare, or other"
    )]
    pub slug: String,

    #[schemars(description = "Severity level for this category: low, medium, high, or critical")]
    pub severity: ReportSeverity,
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
        description = "List of categories with their severities. A report can belong to multiple categories."
    )]
    #[serde(default)]
    pub categories: Vec<ExtractedCategory>,

    #[schemars(
        description = "Type of report: report (observation), proposal (improvement suggestion), complaint (issue complaint), inquiry (question), or appreciation (positive feedback)"
    )]
    pub tag_type: Option<ReportTagType>,

    #[schemars(description = "When the issue started or occurred")]
    pub timeline: Option<String>,

    #[schemars(description = "Who or how many people are affected")]
    pub impact: Option<String>,

    #[schemars(
        description = "Raw location description from the user (address, landmark, area name)"
    )]
    pub location_raw: Option<String>,

    #[schemars(
        description = "Structured query for geocoding - combines street name with nearby landmark (e.g., 'Jalan Tunjungan dekat Tugu Pahlawan, Surabaya')"
    )]
    pub location_query: Option<String>,

    #[schemars(
        description = "Street name extracted from location (e.g., 'Jalan Pemuda', 'Gang Melati')"
    )]
    pub location_street: Option<String>,

    #[schemars(
        description = "City or kabupaten name (e.g., 'Surabaya', 'Kabupaten Sidoarjo', 'Kota Malang')"
    )]
    pub location_city: Option<String>,

    #[schemars(
        description = "Province/state name (e.g., 'Jawa Timur', 'DKI Jakarta', 'Jawa Barat')"
    )]
    pub location_state: Option<String>,

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
    pool: PgPool,
    client: TensorZeroClient,
    openai_api_key: String,
    model_name: String,
    adk_storage: Arc<PostgresStorage>,
}

impl ExtractionService {
    pub fn new(
        pool: PgPool,
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
            pool,
            client,
            openai_api_key,
            model_name,
            adk_storage,
        })
    }

    /// Fetch active categories from the database
    async fn fetch_active_categories(&self) -> Result<Vec<CategoryInfo>> {
        let categories = sqlx::query_as!(
            CategoryInfo,
            r#"
            SELECT name, slug, description
            FROM categories
            WHERE is_active = true
            ORDER BY display_order ASC
            "#
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch categories: {:?}", e);
            AppError::Database(e)
        })?;

        tracing::debug!(
            "Fetched {} active categories for extraction",
            categories.len()
        );
        Ok(categories)
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
        // Fetch categories from database for dynamic prompt
        let categories = self.fetch_active_categories().await?;

        let system_prompt = Self::build_system_prompt(&categories)?;
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

    fn build_system_prompt(categories: &[CategoryInfo]) -> Result<String> {
        // Build dynamic category list from database
        let category_list = if categories.is_empty() {
            // Fallback to default categories if database is empty
            r#"- `infrastructure` - Roads, bridges, drainage, public facilities, street lights, buildings
- `environment` - Garbage, pollution, flooding, green spaces, cleanliness, sanitation
- `public-safety` - Crime, dangerous conditions, accidents, poor lighting, security concerns
- `social-welfare` - Poverty, health, education, community needs, social issues
- `other` - Issues that don't fit the above categories"#.to_string()
        } else {
            categories
                .iter()
                .map(|c| {
                    let desc = c.description.as_deref().unwrap_or("General category");
                    format!("- `{}` - {} ({})", c.slug, c.name, desc)
                })
                .collect::<Vec<_>>()
                .join("\n")
        };

        let json_schema = ExtractedReportData::json_schema_string();

        render_extraction_prompt(&category_list, &json_schema)
            .map_err(|e| AppError::Internal(format!("Failed to render extraction prompt: {}", e)))
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
    fn test_extracted_report_data_deserialize_with_categories() {
        let json = r#"{
            "title": "Pothole on Main Street",
            "description": "Large pothole causing traffic issues",
            "categories": [
                {"slug": "infrastructure", "severity": "high"},
                {"slug": "public-safety", "severity": "medium"}
            ],
            "tag_type": "complaint",
            "timeline": "Started last week",
            "impact": "Affects daily commuters",
            "location_raw": "Jl. Sudirman No. 123"
        }"#;

        let data: ExtractedReportData = serde_json::from_str(json).unwrap();
        assert_eq!(data.title, "Pothole on Main Street");
        assert_eq!(data.categories.len(), 2);
        assert_eq!(data.categories[0].slug, "infrastructure");
        assert_eq!(data.categories[0].severity, ReportSeverity::High);
        assert_eq!(data.categories[1].slug, "public-safety");
        assert_eq!(data.tag_type, Some(ReportTagType::Complaint));
        assert!(data.is_success());
    }

    #[test]
    fn test_extracted_report_data_with_structured_location() {
        let json = r#"{
            "title": "Jalan Berlubang",
            "description": "Lubang besar di tengah jalan",
            "categories": [{"slug": "infrastructure", "severity": "high"}],
            "tag_type": "report",
            "timeline": "Sudah 2 minggu",
            "impact": "Banyak pengendara motor jatuh",
            "location_raw": "di depan warung Bu Sri, Jalan Tunjungan, Surabaya",
            "location_query": "Jalan Tunjungan dekat warung Bu Sri, Surabaya",
            "location_street": "Jalan Tunjungan",
            "location_city": "Surabaya",
            "location_state": "Jawa Timur"
        }"#;

        let data: ExtractedReportData = serde_json::from_str(json).unwrap();
        assert_eq!(
            data.location_raw,
            Some("di depan warung Bu Sri, Jalan Tunjungan, Surabaya".to_string())
        );
        assert_eq!(
            data.location_query,
            Some("Jalan Tunjungan dekat warung Bu Sri, Surabaya".to_string())
        );
        assert_eq!(data.location_street, Some("Jalan Tunjungan".to_string()));
        assert_eq!(data.location_city, Some("Surabaya".to_string()));
        assert_eq!(data.location_state, Some("Jawa Timur".to_string()));
    }

    #[test]
    fn test_extracted_report_data_with_nulls() {
        let json = r#"{
            "title": "Test Report",
            "description": "Test description",
            "categories": [],
            "tag_type": null,
            "timeline": null,
            "impact": null,
            "location_raw": null,
            "location_query": null,
            "location_street": null,
            "location_city": null,
            "location_state": null
        }"#;

        let data: ExtractedReportData = serde_json::from_str(json).unwrap();
        assert_eq!(data.title, "Test Report");
        assert!(data.categories.is_empty());
        assert!(data.tag_type.is_none());
        assert!(data.location_query.is_none());
        assert!(data.location_street.is_none());
        assert!(data.location_city.is_none());
        assert!(data.location_state.is_none());
    }

    #[test]
    fn test_parse_with_fallback_valid_json() {
        let input = r#"{"title": "Test", "description": "Test desc", "categories": []}"#;

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
    "categories": [{"slug": "infrastructure", "severity": "medium"}]
}
```"#;

        let result: ExtractedReportData = parse_with_fallback(input);

        assert!(result.is_success());
        assert_eq!(result.title, "Markdown Test");
        assert_eq!(result.categories.len(), 1);
        assert_eq!(result.categories[0].slug, "infrastructure");
    }

    #[test]
    fn test_parse_with_fallback_with_trailing_comma() {
        let input = r#"{"title": "Test", "description": "Desc", "categories": [],}"#;

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
        assert!(schema.contains("categories"));
        assert!(schema.contains("tag_type"));

        // Should contain new location fields
        assert!(schema.contains("location_raw"));
        assert!(schema.contains("location_query"));
        assert!(schema.contains("location_street"));
        assert!(schema.contains("location_city"));
        assert!(schema.contains("location_state"));

        // Should NOT contain internal fields (marked with #[schemars(skip)])
        assert!(!schema.contains("is_llm_success"));
        assert!(!schema.contains("llm_error_message"));
    }

    #[test]
    fn test_build_system_prompt_contains_schema() {
        // Test with empty categories (uses fallback)
        let prompt = ExtractionService::build_system_prompt(&[]).unwrap();

        // Should contain the schema
        assert!(prompt.contains("title"));
        assert!(prompt.contains("description"));
        assert!(prompt.contains("categories"));

        // Should contain instructions
        assert!(prompt.contains("data extraction assistant"));
        assert!(prompt.contains("JSON"));

        // Should contain multi-category instructions
        assert!(prompt.contains("Multi-Category Support"));
        assert!(prompt.contains("slug"));
        assert!(prompt.contains("severity"));

        // Should contain tag_type instructions
        assert!(prompt.contains("tag_type"));
        assert!(prompt.contains("report"));
        assert!(prompt.contains("proposal"));
        assert!(prompt.contains("complaint"));

        // Should contain location extraction instructions
        assert!(prompt.contains("Location Extraction"));
        assert!(prompt.contains("location_query"));
        assert!(prompt.contains("location_street"));
        assert!(prompt.contains("location_city"));
        assert!(prompt.contains("location_state"));
        assert!(prompt.contains("Jawa Timur")); // Example province
    }

    #[test]
    fn test_build_system_prompt_with_dynamic_categories() {
        let categories = vec![
            CategoryInfo {
                name: "Infrastructure".to_string(),
                slug: "infrastructure".to_string(),
                description: Some("Roads, bridges, and public facilities".to_string()),
            },
            CategoryInfo {
                name: "Environment".to_string(),
                slug: "environment".to_string(),
                description: Some("Garbage, pollution, and green spaces".to_string()),
            },
        ];

        let prompt = ExtractionService::build_system_prompt(&categories).unwrap();

        // Should contain the dynamic categories in the Available Category Slugs section
        assert!(prompt.contains("`infrastructure` - Infrastructure"));
        assert!(prompt.contains("Roads, bridges, and public facilities"));
        assert!(prompt.contains("`environment` - Environment"));
        assert!(prompt.contains("Garbage, pollution, and green spaces"));

        // Should NOT contain fallback categories in the Available Category Slugs section
        // (note: the examples section will still contain hardcoded examples)
        assert!(!prompt.contains("- `public-safety` - Crime"));
        assert!(!prompt.contains("- `social-welfare` - Poverty"));
    }

    #[test]
    fn test_tag_types() {
        let json = r#"{
            "title": "Thank you",
            "description": "Great service",
            "categories": [],
            "tag_type": "appreciation"
        }"#;

        let data: ExtractedReportData = serde_json::from_str(json).unwrap();
        assert_eq!(data.tag_type, Some(ReportTagType::Appreciation));

        let json2 = r#"{
            "title": "How do I...",
            "description": "Question about...",
            "categories": [],
            "tag_type": "inquiry"
        }"#;

        let data2: ExtractedReportData = serde_json::from_str(json2).unwrap();
        assert_eq!(data2.tag_type, Some(ReportTagType::Inquiry));
    }
}
