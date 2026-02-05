use serde::Serialize;
use utoipa::ToSchema;

/// A prompt key definition from the backend-managed registry.
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PromptKeyDefinition {
    /// The unique prompt key (e.g. "citizen_report_agent/system")
    pub key: &'static str,
    /// Human-readable description of what this prompt is used for
    pub description: &'static str,
}

/// All valid prompt keys. This is the single source of truth.
const PROMPT_KEY_REGISTRY: &[PromptKeyDefinition] = &[
    PromptKeyDefinition {
        key: "citizen_report_agent/system",
        description: "System prompt for the citizen report AI agent",
    },
    PromptKeyDefinition {
        key: "citizen_report_extraction/system",
        description: "System prompt for extracting structured data from citizen reports",
    },
];

/// Check whether the given key is in the registry.
pub fn is_valid_prompt_key(key: &str) -> bool {
    PROMPT_KEY_REGISTRY.iter().any(|def| def.key == key)
}

/// Return all registered prompt keys.
pub fn get_all_prompt_keys() -> Vec<PromptKeyDefinition> {
    PROMPT_KEY_REGISTRY.to_vec()
}
