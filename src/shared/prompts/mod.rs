//! Prompt template management module.
//!
//! This module provides a centralized way to manage prompt templates for AI agents.
//! Templates are stored in `templates/prompts/` and use Jinja2 syntax.
//!
//! # Usage
//!
//! ```ignore
//! use std::collections::HashMap;
//! use crate::shared::prompts::{render_template_simple, PromptContext};
//!
//! // Simple string context
//! let mut ctx = HashMap::new();
//! ctx.insert("day_name", "Senin");
//! ctx.insert("date", "03-02-2026");
//!
//! let prompt = render_template_simple("citizen_report_agent/system.jinja", &ctx)?;
//! ```

pub mod engine;

pub use engine::{render_template, TemplateError};

use chrono::{Datelike, Local, Weekday};
use minijinja::Value;
use std::collections::HashMap;

/// Convert weekday to Indonesian day name
fn weekday_to_indonesian(weekday: Weekday) -> &'static str {
    match weekday {
        Weekday::Mon => "Senin",
        Weekday::Tue => "Selasa",
        Weekday::Wed => "Rabu",
        Weekday::Thu => "Kamis",
        Weekday::Fri => "Jumat",
        Weekday::Sat => "Sabtu",
        Weekday::Sun => "Minggu",
    }
}

/// Build a context with current datetime information.
///
/// Returns a HashMap with:
/// - `day_name`: Indonesian day name (e.g., "Senin")
/// - `date`: Date in dd-mm-yyyy format
/// - `time`: Time in HH:MM format
pub fn datetime_context() -> HashMap<&'static str, String> {
    let now = Local::now();
    let mut ctx = HashMap::new();
    ctx.insert("day_name", weekday_to_indonesian(now.weekday()).to_string());
    ctx.insert("date", now.format("%d-%m-%Y").to_string());
    ctx.insert("time", now.format("%H:%M").to_string());
    ctx
}

/// Render the citizen report agent system prompt.
///
/// # Arguments
/// * `attachments` - Optional attachment context to include
///
/// # Returns
/// The rendered system prompt with current datetime and optional attachments.
pub async fn render_citizen_report_agent_prompt(
    attachments: Option<&str>,
) -> Result<String, TemplateError> {
    let datetime = datetime_context();

    let mut ctx: HashMap<&str, Value> = HashMap::new();
    ctx.insert(
        "day_name",
        Value::from(datetime.get("day_name").unwrap().as_str()),
    );
    ctx.insert("date", Value::from(datetime.get("date").unwrap().as_str()));
    ctx.insert("time", Value::from(datetime.get("time").unwrap().as_str()));
    ctx.insert("attachments", Value::from(attachments.unwrap_or("")));
    ctx.insert("has_attachments", Value::from(attachments.is_some()));

    render_template("citizen_report_agent/system.jinja", &ctx).await
}

/// Render the extraction service system prompt.
///
/// # Arguments
/// * `category_list` - Formatted list of categories for extraction
/// * `json_schema` - JSON schema string for the expected output
///
/// # Returns
/// The rendered system prompt with dynamic categories and schema.
pub async fn render_extraction_prompt(
    category_list: &str,
    json_schema: &str,
) -> Result<String, TemplateError> {
    let mut ctx: HashMap<&str, Value> = HashMap::new();
    ctx.insert("category_list", Value::from(category_list));
    ctx.insert("json_schema", Value::from(json_schema));

    render_template("extraction/system.jinja", &ctx).await
}
