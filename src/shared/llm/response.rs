use schemars::gen::SchemaGenerator;
use schemars::JsonSchema;
use serde::de::DeserializeOwned;

/// Trait for LLM response types that support fallback behavior
///
/// Types implementing this trait can be parsed with graceful degradation -
/// if parsing fails, a default fallback value is returned with error information.
pub trait LlmResponse: DeserializeOwned + Default + JsonSchema {
    /// Mark this response as a fallback due to parsing failure
    fn mark_as_fallback(&mut self, error_message: String);

    /// Check if this response was successfully parsed
    fn is_success(&self) -> bool;

    /// Generate JSON schema string for use in LLM prompts
    fn json_schema_string() -> String {
        let mut gen = SchemaGenerator::default();
        let schema = gen.root_schema_for::<Self>();
        serde_json::to_string_pretty(&schema).unwrap_or_else(|_| "{}".to_string())
    }
}
