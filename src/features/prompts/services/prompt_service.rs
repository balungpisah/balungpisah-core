use crate::core::error::{AppError, Result};
use crate::features::prompts::dtos::{
    CreatePromptDto, PromptQueryParams, PromptResponseDto, UpdatePromptDto,
};
use crate::features::prompts::models::Prompt;
use minijinja::Environment;
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use uuid::Uuid;

// Cache structure
struct PromptCache {
    templates: HashMap<String, String>, // key -> template_content
    last_fetched: Instant,
}

/// Validate that a template can be compiled by minijinja
fn validate_template_compilation(template_content: &str) -> Result<()> {
    let mut env = Environment::new();
    env.add_template("_validation_", template_content)
        .map_err(|e| AppError::Validation(format!("Template compilation failed: {}", e)))?;
    Ok(())
}

/// Convert database error to more specific AppError with user-friendly messages
fn handle_db_error(e: sqlx::Error) -> AppError {
    if let sqlx::Error::Database(db_err) = &e {
        // Check for unique constraint violation (PostgreSQL error code 23505)
        if db_err.code() == Some(std::borrow::Cow::Borrowed("23505")) {
            // Extract constraint name if available
            if let Some(constraint) = db_err.constraint() {
                if constraint.contains("key_unique_when_active") {
                    return AppError::Conflict(
                        "Prompt with this key already exists. Please use a different key or delete the existing active prompt first.".to_string()
                    );
                }
            }
            // Generic unique constraint error
            return AppError::Conflict(
                "A prompt with this key already exists and is active.".to_string(),
            );
        }

        // Check for foreign key violation (PostgreSQL error code 23503)
        if db_err.code() == Some(std::borrow::Cow::Borrowed("23503")) {
            return AppError::BadRequest("Referenced record does not exist.".to_string());
        }
    }

    // For other database errors, return generic database error
    AppError::Database(e)
}

pub struct PromptService {
    pool: PgPool,
    cache: Arc<RwLock<Option<PromptCache>>>,
    cache_ttl: Duration,
}

impl std::fmt::Debug for PromptService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PromptService")
            .field("pool", &"<PgPool>")
            .field("cache_ttl", &self.cache_ttl)
            .finish()
    }
}

impl PromptService {
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            cache: Arc::new(RwLock::new(None)),
            cache_ttl: Duration::from_secs(300), // 5 minutes default
        }
    }

    /// Get template content by key (used by rendering engine)
    pub async fn get_template_by_key(&self, key: &str) -> Result<Option<String>> {
        // Check cache first
        {
            let cache_read = self.cache.read().await;
            if let Some(ref cached) = *cache_read {
                if cached.last_fetched.elapsed() < self.cache_ttl {
                    if let Some(template) = cached.templates.get(key) {
                        return Ok(Some(template.clone()));
                    }
                }
            }
        }

        // Cache miss or expired - fetch from database
        let prompt = sqlx::query_as!(
            Prompt,
            r#"
            SELECT id, key, name, description, template_content,
                   variables as "variables: serde_json::Value",
                   version, is_active, created_at, updated_at, created_by, updated_by
            FROM prompts
            WHERE key = $1 AND is_active = true
            "#,
            key
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::Database)?;

        if let Some(p) = prompt {
            // Update cache
            let mut cache_write = self.cache.write().await;
            let mut templates = HashMap::new();
            templates.insert(key.to_string(), p.template_content.clone());
            *cache_write = Some(PromptCache {
                templates,
                last_fetched: Instant::now(),
            });

            Ok(Some(p.template_content))
        } else {
            Ok(None)
        }
    }

    /// Create a new prompt
    pub async fn create(&self, dto: CreatePromptDto) -> Result<PromptResponseDto> {
        // Validate template compilation before saving
        validate_template_compilation(&dto.template_content)?;

        let prompt = sqlx::query_as!(
            Prompt,
            r#"
            INSERT INTO prompts (key, name, description, template_content, variables)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id, key, name, description, template_content,
                      variables as "variables: serde_json::Value",
                      version, is_active, created_at, updated_at, created_by, updated_by
            "#,
            dto.key,
            dto.name,
            dto.description,
            dto.template_content,
            dto.variables as Option<serde_json::Value>
        )
        .fetch_one(&self.pool)
        .await
        .map_err(handle_db_error)?;

        // Invalidate cache
        self.invalidate_cache().await;

        Ok(PromptResponseDto::from(prompt))
    }

    /// Get prompt by ID
    pub async fn get_by_id(&self, id: Uuid) -> Result<PromptResponseDto> {
        let prompt = sqlx::query_as!(
            Prompt,
            r#"
            SELECT id, key, name, description, template_content,
                   variables as "variables: serde_json::Value",
                   version, is_active, created_at, updated_at, created_by, updated_by
            FROM prompts
            WHERE id = $1
            "#,
            id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound(format!("Prompt with id {} not found", id)))?;

        Ok(PromptResponseDto::from(prompt))
    }

    /// List prompts with pagination and filters
    pub async fn list(&self, params: &PromptQueryParams) -> Result<(Vec<PromptResponseDto>, i64)> {
        // Build WHERE conditions
        let mut conditions = Vec::new();

        // Filter by is_active
        if let Some(is_active) = params.is_active {
            conditions.push(format!("is_active = {}", is_active));
        }

        // Prepare search pattern
        let search_pattern = params.search.as_ref().map(|s| format!("%{}%", s));

        // Handle queries based on whether search is present
        if let Some(ref search) = search_pattern {
            // WITH SEARCH: Use $1 for search, $2 for limit, $3 for offset
            conditions.push("(key ILIKE $1 OR name ILIKE $1 OR description ILIKE $1)".to_string());

            let where_clause = format!("WHERE {}", conditions.join(" AND "));

            // Get total count
            let count_query = format!("SELECT COUNT(*) FROM prompts {}", where_clause);
            let total: i64 = sqlx::query_scalar(&count_query)
                .bind(search)
                .fetch_one(&self.pool)
                .await
                .map_err(AppError::Database)?;

            // Get paginated results
            let query = format!(
                r#"
                SELECT id, key, name, description, template_content,
                       variables, version, is_active, created_at, updated_at, created_by, updated_by
                FROM prompts
                {}
                ORDER BY created_at {}
                LIMIT $2 OFFSET $3
                "#,
                where_clause,
                params.sort.as_sql()
            );

            let prompts: Vec<Prompt> = sqlx::query_as(&query)
                .bind(search)
                .bind(params.limit())
                .bind(params.offset())
                .fetch_all(&self.pool)
                .await
                .map_err(AppError::Database)?;

            Ok((
                prompts.into_iter().map(PromptResponseDto::from).collect(),
                total,
            ))
        } else {
            // WITHOUT SEARCH: Use $1 for limit, $2 for offset
            let where_clause = if conditions.is_empty() {
                String::new()
            } else {
                format!("WHERE {}", conditions.join(" AND "))
            };

            // Get total count
            let count_query = format!("SELECT COUNT(*) FROM prompts {}", where_clause);
            let total: i64 = sqlx::query_scalar(&count_query)
                .fetch_one(&self.pool)
                .await
                .map_err(AppError::Database)?;

            // Get paginated results
            let query = format!(
                r#"
                SELECT id, key, name, description, template_content,
                       variables, version, is_active, created_at, updated_at, created_by, updated_by
                FROM prompts
                {}
                ORDER BY created_at {}
                LIMIT $1 OFFSET $2
                "#,
                where_clause,
                params.sort.as_sql()
            );

            let prompts: Vec<Prompt> = sqlx::query_as(&query)
                .bind(params.limit())
                .bind(params.offset())
                .fetch_all(&self.pool)
                .await
                .map_err(AppError::Database)?;

            Ok((
                prompts.into_iter().map(PromptResponseDto::from).collect(),
                total,
            ))
        }
    }

    /// Update prompt (increments version)
    pub async fn update(&self, id: Uuid, dto: UpdatePromptDto) -> Result<PromptResponseDto> {
        // Validate template compilation if template_content is being updated
        if let Some(ref template_content) = dto.template_content {
            validate_template_compilation(template_content)?;
        }

        let prompt = sqlx::query_as!(
            Prompt,
            r#"
            UPDATE prompts
            SET name = COALESCE($1, name),
                description = COALESCE($2, description),
                template_content = COALESCE($3, template_content),
                variables = COALESCE($4, variables),
                version = version + 1,
                updated_at = NOW()
            WHERE id = $5
            RETURNING id, key, name, description, template_content,
                      variables as "variables: serde_json::Value",
                      version, is_active, created_at, updated_at, created_by, updated_by
            "#,
            dto.name,
            dto.description,
            dto.template_content,
            dto.variables as Option<serde_json::Value>,
            id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(handle_db_error)?
        .ok_or_else(|| AppError::NotFound(format!("Prompt with id {} not found", id)))?;

        // Invalidate cache
        self.invalidate_cache().await;

        Ok(PromptResponseDto::from(prompt))
    }

    /// Delete (soft delete)
    pub async fn delete(&self, id: Uuid) -> Result<()> {
        let result = sqlx::query!(
            r#"
            UPDATE prompts
            SET is_active = false, updated_at = NOW()
            WHERE id = $1
            "#,
            id
        )
        .execute(&self.pool)
        .await
        .map_err(AppError::Database)?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!(
                "Prompt with id {} not found",
                id
            )));
        }

        // Invalidate cache
        self.invalidate_cache().await;

        Ok(())
    }

    /// Clear the cache
    async fn invalidate_cache(&self) {
        let mut cache_write = self.cache.write().await;
        *cache_write = None;
    }
}
