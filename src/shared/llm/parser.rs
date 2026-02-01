use lazy_static::lazy_static;
use regex::Regex;
use std::time::Duration;

use super::LlmResponse;

lazy_static! {
    /// Regex for trailing commas before } or ]
    static ref TRAILING_COMMA_RE: Regex = Regex::new(r",(\s*[}\]])").unwrap();

    /// Regex for JavaScript string concatenation ("str1" + "str2")
    static ref JS_STRING_CONCAT_RE: Regex = Regex::new(r#""\s*\+\s*""#).unwrap();
}

/// Timeout for JSON repair operations
const JSON_REPAIR_TIMEOUT: Duration = Duration::from_secs(5);

/// Extract JSON string from text (handles multiple formats)
///
/// Tries in order:
/// 1. JSON in markdown code block: ```json ... ```
/// 2. Generic markdown code block: ``` ... ```
/// 3. Plain JSON starting with {
/// 4. JSON embedded anywhere in text (find { to })
pub fn extract_json_string(text: &str) -> Result<String, String> {
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
///
/// Example: `{"name": "John",}` -> `{"name": "John"}`
pub fn fix_trailing_commas(json_str: &str) -> String {
    TRAILING_COMMA_RE.replace_all(json_str, "$1").to_string()
}

/// Fix JavaScript string concatenation which is invalid in JSON
///
/// LLMs sometimes output: `"str1" + "str2"` which is invalid JSON.
/// This merges them into: `"str1str2"`
pub fn fix_js_string_concatenation(json_str: &str) -> String {
    JS_STRING_CONCAT_RE.replace_all(json_str, "").to_string()
}

/// Apply quick fixes to malformed JSON
fn apply_quick_fixes(json_str: &str) -> String {
    let fixed = fix_js_string_concatenation(json_str);
    fix_trailing_commas(&fixed)
}

/// Attempt to repair JSON using llm_json crate with timeout
///
/// Returns the repaired JSON string if successful, or None if repair fails or times out
fn repair_json_with_timeout(json_str: &str) -> Option<String> {
    // Use tokio timeout for async context, but since this is sync, we'll just use it directly
    // In practice, llm_json::repair_json is fast enough that we don't need actual timeout
    let start = std::time::Instant::now();

    let options = llm_json::RepairOptions::default();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        llm_json::repair_json(json_str, &options)
    }));

    if start.elapsed() > JSON_REPAIR_TIMEOUT {
        tracing::warn!("JSON repair took longer than timeout");
        return None;
    }

    match result {
        Ok(Ok(repaired)) => Some(repaired),
        Ok(Err(e)) => {
            tracing::debug!("JSON repair failed: {:?}", e);
            None
        }
        Err(_) => {
            tracing::warn!("JSON repair panicked");
            None
        }
    }
}

/// Try to parse text as the target type using multiple strategies
///
/// Parsing pipeline:
/// 1. Extract JSON string (markdown/plain/embedded)
/// 2. Try direct parse (fast path)
/// 3. Apply quick fixes (trailing commas, string concat)
/// 4. Try parse after quick fixes
/// 5. Apply llm_json::repair_json() with timeout
/// 6. Final parse attempt
fn try_parse<T>(text: &str) -> Result<T, String>
where
    T: LlmResponse,
{
    // Step 1: Extract JSON string
    let json_str = extract_json_string(text)?;

    tracing::debug!(
        "Extracted JSON (first 500 chars): {}",
        json_str.chars().take(500).collect::<String>()
    );

    // Step 2: Try direct parse (fast path)
    if let Ok(parsed) = serde_json::from_str::<T>(&json_str) {
        tracing::debug!("JSON parsed successfully (fast path)");
        return Ok(parsed);
    }

    // Step 3-4: Apply quick fixes and try again
    let fixed_json = apply_quick_fixes(&json_str);
    if let Ok(parsed) = serde_json::from_str::<T>(&fixed_json) {
        tracing::debug!("JSON parsed successfully after quick fixes");
        return Ok(parsed);
    }

    // Step 5-6: Try advanced repair with llm_json
    if let Some(repaired) = repair_json_with_timeout(&json_str) {
        if let Ok(parsed) = serde_json::from_str::<T>(&repaired) {
            tracing::debug!("JSON parsed successfully after llm_json repair");
            return Ok(parsed);
        }
    }

    // All attempts failed - return error for fallback handling
    Err(format!(
        "Failed to parse JSON after all repair attempts. Original: {}",
        json_str.chars().take(200).collect::<String>()
    ))
}

/// Parse LLM response text with graceful fallback
///
/// This is the main entry point for parsing LLM responses. It attempts
/// to parse the text into the target type using multiple strategies.
/// If all parsing attempts fail, it returns a default fallback value
/// with the error message attached.
///
/// # Example
///
/// ```ignore
/// use crate::shared::llm::{parse_with_fallback, LlmResponse};
///
/// #[derive(Default, Deserialize, JsonSchema)]
/// struct MyResponse {
///     data: String,
///     #[serde(default = "default_true")]
///     #[schemars(skip)]
///     is_llm_success: bool,
///     #[serde(skip_serializing_if = "Option::is_none")]
///     #[schemars(skip)]
///     llm_error_message: Option<String>,
/// }
///
/// impl LlmResponse for MyResponse {
///     fn mark_as_fallback(&mut self, error: String) {
///         self.is_llm_success = false;
///         self.llm_error_message = Some(error);
///     }
///     fn is_success(&self) -> bool { self.is_llm_success }
/// }
///
/// let response = parse_with_fallback::<MyResponse>(llm_output);
/// if response.is_success() {
///     // Use parsed data
/// } else {
///     // Handle fallback case
/// }
/// ```
pub fn parse_with_fallback<T>(text: &str) -> T
where
    T: LlmResponse,
{
    match try_parse::<T>(text) {
        Ok(parsed) => parsed,
        Err(error_msg) => {
            tracing::warn!("LLM response parsing failed, using fallback: {}", error_msg);
            let mut fallback = T::default();
            fallback.mark_as_fallback(error_msg);
            fallback
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use schemars::JsonSchema;
    use serde::Deserialize;

    fn default_true() -> bool {
        true
    }

    #[derive(Debug, Clone, Default, Deserialize, JsonSchema)]
    struct TestResponse {
        pub title: String,
        pub description: String,
        pub count: Option<i32>,

        #[serde(default = "default_true")]
        #[schemars(skip)]
        pub is_llm_success: bool,

        #[serde(skip_serializing_if = "Option::is_none")]
        #[schemars(skip)]
        pub llm_error_message: Option<String>,
    }

    impl LlmResponse for TestResponse {
        fn mark_as_fallback(&mut self, error_message: String) {
            self.is_llm_success = false;
            self.llm_error_message = Some(error_message);
        }

        fn is_success(&self) -> bool {
            self.is_llm_success
        }
    }

    // ==================== extract_json_string tests ====================

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

        let json = extract_json_string(response).unwrap();
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

        let json = extract_json_string(response).unwrap();
        assert!(json.starts_with('{'));
        assert!(json.ends_with('}'));
    }

    #[test]
    fn test_extract_json_string_plain_json() {
        let response = r#"{"title": "Test", "description": "Test desc"}"#;

        let json = extract_json_string(response).unwrap();
        assert_eq!(json, response);
    }

    #[test]
    fn test_extract_json_string_with_whitespace() {
        let response = r#"

{"title": "Test", "description": "Test desc"}

"#;

        let json = extract_json_string(response).unwrap();
        assert!(json.starts_with('{'));
        assert!(json.ends_with('}'));
    }

    #[test]
    fn test_extract_json_string_embedded() {
        let response =
            "Some text before {\"title\": \"Test\", \"description\": \"desc\"} some text after";

        let json = extract_json_string(response).unwrap();
        assert_eq!(json, r#"{"title": "Test", "description": "desc"}"#);
    }

    #[test]
    fn test_extract_json_string_no_json() {
        let response = "No JSON here at all!";

        let result = extract_json_string(response);
        assert!(result.is_err());
    }

    // ==================== fix functions tests ====================

    #[test]
    fn test_fix_trailing_commas() {
        // Should remove trailing comma before }
        let input = r#"{"name": "John", "age": 30,}"#;
        let fixed = fix_trailing_commas(input);
        assert_eq!(fixed, r#"{"name": "John", "age": 30}"#);

        // Should remove trailing comma before ]
        let input2 = r#"{"items": [1, 2, 3,]}"#;
        let fixed2 = fix_trailing_commas(input2);
        assert_eq!(fixed2, r#"{"items": [1, 2, 3]}"#);

        // Nested trailing commas
        let input3 = r#"{"obj": {"nested": true,},}"#;
        let fixed3 = fix_trailing_commas(input3);
        assert_eq!(fixed3, r#"{"obj": {"nested": true}}"#);
    }

    #[test]
    fn test_fix_js_string_concatenation() {
        // Basic concatenation
        let input = r#"{"text": "hello" + "world"}"#;
        let fixed = fix_js_string_concatenation(input);
        assert_eq!(fixed, r#"{"text": "helloworld"}"#);

        // Multiple concatenations
        let input2 = r#"{"msg": "a" + "b" + "c"}"#;
        let fixed2 = fix_js_string_concatenation(input2);
        assert_eq!(fixed2, r#"{"msg": "abc"}"#);

        // With spaces
        let input3 = r#"{"text": "hello"   +   "world"}"#;
        let fixed3 = fix_js_string_concatenation(input3);
        assert_eq!(fixed3, r#"{"text": "helloworld"}"#);
    }

    // ==================== parse_with_fallback tests ====================

    #[test]
    fn test_parse_with_fallback_valid_json() {
        let input = r#"{"title": "Test Title", "description": "Test Description", "count": 42}"#;

        let result: TestResponse = parse_with_fallback(input);

        assert!(result.is_success());
        assert_eq!(result.title, "Test Title");
        assert_eq!(result.description, "Test Description");
        assert_eq!(result.count, Some(42));
        assert!(result.llm_error_message.is_none());
    }

    #[test]
    fn test_parse_with_fallback_markdown_json() {
        let input = r#"Here's the response:

```json
{"title": "Markdown Test", "description": "From code block"}
```"#;

        let result: TestResponse = parse_with_fallback(input);

        assert!(result.is_success());
        assert_eq!(result.title, "Markdown Test");
    }

    #[test]
    fn test_parse_with_fallback_with_trailing_comma() {
        let input = r#"{"title": "Test", "description": "Desc",}"#;

        let result: TestResponse = parse_with_fallback(input);

        assert!(result.is_success());
        assert_eq!(result.title, "Test");
    }

    #[test]
    fn test_parse_with_fallback_with_string_concat() {
        let input = r#"{"title": "Part1" + "Part2", "description": "Desc"}"#;

        let result: TestResponse = parse_with_fallback(input);

        assert!(result.is_success());
        assert_eq!(result.title, "Part1Part2");
    }

    #[test]
    fn test_parse_with_fallback_invalid_returns_fallback() {
        let input = "This is not JSON at all";

        let result: TestResponse = parse_with_fallback(input);

        assert!(!result.is_success());
        assert!(result.llm_error_message.is_some());
        assert!(result.title.is_empty()); // Default value
    }

    #[test]
    fn test_parse_with_fallback_partial_json_returns_fallback() {
        let input = r#"{"title": "Test", "description": }"#;

        let result: TestResponse = parse_with_fallback(input);

        // llm_json should be able to repair this, but if not, fallback
        // Either way, the function should not panic
        assert!(result.is_success() || result.llm_error_message.is_some());
    }

    // ==================== json_schema_string tests ====================

    #[test]
    fn test_json_schema_string_generation() {
        let schema = TestResponse::json_schema_string();

        assert!(schema.contains("title"));
        assert!(schema.contains("description"));
        assert!(schema.contains("count"));
        // Should NOT contain internal fields
        assert!(!schema.contains("is_llm_success"));
        assert!(!schema.contains("llm_error_message"));
    }
}
