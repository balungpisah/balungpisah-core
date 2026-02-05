//! Template engine for prompt management using Jinja2 syntax.
//!
//! This module provides a centralized way to manage and render prompt templates
//! for various AI agents and services.

use minijinja::{Environment, Value};
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, OnceLock};
use thiserror::Error;

/// Global template environment
static TEMPLATE_ENV: OnceLock<Environment<'static>> = OnceLock::new();

/// Global PromptService instance (set during app initialization)
static PROMPT_SERVICE: OnceLock<Arc<crate::features::prompts::services::PromptService>> =
    OnceLock::new();

/// Initialize the global PromptService
pub fn init_prompt_service(service: Arc<crate::features::prompts::services::PromptService>) {
    PROMPT_SERVICE
        .set(service)
        .expect("PromptService already initialized");
}

/// Template directory relative to the project root
const TEMPLATE_DIR: &str = "templates/prompts";

/// Errors that can occur during template operations
#[derive(Debug, Error)]
pub enum TemplateError {
    #[error("Template '{0}' not found")]
    NotFound(String),

    #[error("Failed to render template: {0}")]
    RenderError(String),
}

/// Initialize the template environment with all templates from the templates directory.
///
/// This function is called automatically on first use of `render_template`.
/// Templates are loaded from `templates/prompts/` directory.
fn init_environment() -> Environment<'static> {
    let mut env = Environment::new();

    // Get the template directory path
    let template_path = Path::new(TEMPLATE_DIR);

    if template_path.exists() {
        // Load all .jinja files recursively
        load_templates_recursive(&mut env, template_path, template_path);
    }

    env
}

/// Recursively load all .jinja templates from a directory
fn load_templates_recursive(env: &mut Environment<'static>, base_path: &Path, current_path: &Path) {
    if let Ok(entries) = std::fs::read_dir(current_path) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                load_templates_recursive(env, base_path, &path);
            } else if path.extension().is_some_and(|ext| ext == "jinja") {
                // Create template name from relative path
                if let Ok(relative) = path.strip_prefix(base_path) {
                    let template_name = relative.to_string_lossy().to_string();
                    if let Ok(content) = std::fs::read_to_string(&path) {
                        // Convert to 'static str by leaking (safe for long-lived templates)
                        let static_name: &'static str =
                            Box::leak(template_name.clone().into_boxed_str());
                        let static_content: &'static str = Box::leak(content.into_boxed_str());
                        if let Err(e) = env.add_template(static_name, static_content) {
                            tracing::warn!("Failed to load template {}: {}", template_name, e);
                        } else {
                            tracing::debug!("Loaded template: {}", template_name);
                        }
                    }
                }
            }
        }
    }
}

/// Get the global template environment
fn get_environment() -> &'static Environment<'static> {
    TEMPLATE_ENV.get_or_init(init_environment)
}

/// Render a template with the given context.
///
/// This function implements a hybrid approach:
/// 1. First tries to fetch the template from the database (if PromptService is initialized)
/// 2. Falls back to file-based templates if not found in database
///
/// # Arguments
/// * `template_name` - The template path relative to `templates/prompts/` (e.g., "citizen_report_agent/system.jinja")
/// * `context` - A HashMap of variable names to values
///
/// # Example
/// ```ignore
/// use std::collections::HashMap;
/// use crate::shared::prompts::render_template;
///
/// let mut ctx = HashMap::new();
/// ctx.insert("day_name", "Senin");
/// ctx.insert("date", "03-02-2026");
/// ctx.insert("time", "14:30");
///
/// let prompt = render_template("citizen_report_agent/system.jinja", &ctx).await?;
/// ```
pub async fn render_template(
    template_name: &str,
    ctx: &HashMap<&str, Value>,
) -> Result<String, TemplateError> {
    // Step 1: Try database first (if service is initialized)
    // Database keys don't include the .jinja extension
    let db_key = template_name
        .strip_suffix(".jinja")
        .unwrap_or(template_name);
    if let Some(service) = PROMPT_SERVICE.get() {
        match service.get_template_by_key(db_key).await {
            Ok(Some(content)) => {
                // Render database template
                let mut env = Environment::new();
                env.add_template(template_name, &content)
                    .map_err(|e| TemplateError::RenderError(e.to_string()))?;

                let template = env
                    .get_template(template_name)
                    .map_err(|_| TemplateError::NotFound(template_name.to_string()))?;

                let render_ctx = Value::from_iter(ctx.iter().map(|(k, v)| (*k, v.clone())));

                return template
                    .render(render_ctx)
                    .map_err(|e| TemplateError::RenderError(e.to_string()));
            }
            Ok(None) => {
                // Not in database, try file-based
                tracing::debug!(
                    "Template '{}' not found in database, trying file-based",
                    template_name
                );
            }
            Err(e) => {
                // Database error, log but continue to file-based fallback
                tracing::warn!(
                    "Database lookup failed for template '{}': {:?}, falling back to files",
                    template_name,
                    e
                );
            }
        }
    }

    // Step 2: Fall back to file-based templates
    let env = get_environment();

    let template = env
        .get_template(template_name)
        .map_err(|_| TemplateError::NotFound(template_name.to_string()))?;

    let render_ctx = Value::from_iter(ctx.iter().map(|(k, v)| (*k, v.clone())));

    template
        .render(render_ctx)
        .map_err(|e| TemplateError::RenderError(e.to_string()))
}

/// Render a template with a simpler string-only context.
///
/// For templates that only need string variables, this is more convenient.
#[allow(dead_code)]
pub async fn render_template_simple(
    template_name: &str,
    ctx: &HashMap<&str, &str>,
) -> Result<String, TemplateError> {
    let value_ctx: HashMap<&str, Value> = ctx.iter().map(|(k, v)| (*k, Value::from(*v))).collect();

    render_template(template_name, &value_ctx).await
}

/// Check if a template exists
#[allow(dead_code)]
pub fn template_exists(template_name: &str) -> bool {
    get_environment().get_template(template_name).is_ok()
}

/// List all available templates
#[allow(dead_code)]
pub fn list_templates() -> Vec<String> {
    get_environment()
        .templates()
        .map(|(name, _)| name.to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_render_template_simple() {
        // This test will work if the template file exists
        // For now, just test that the function compiles and handles missing templates
        let mut ctx = HashMap::new();
        ctx.insert("test_var", "test_value");

        let result = render_template_simple("nonexistent.jinja", &ctx).await;
        assert!(matches!(result, Err(TemplateError::NotFound(_))));
    }

    #[test]
    fn test_template_exists() {
        // Non-existent template should return false
        assert!(!template_exists("definitely_not_a_real_template.jinja"));
    }
}
