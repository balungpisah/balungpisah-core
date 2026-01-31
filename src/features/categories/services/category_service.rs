use sqlx::PgPool;

use crate::core::error::{AppError, Result};
use crate::features::categories::dtos::{CategoryResponseDto, CategoryTreeDto};
use crate::features::categories::models::Category;

/// Service for category operations
pub struct CategoryService {
    pool: PgPool,
}

impl CategoryService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// List all active categories (flat list)
    pub async fn list(&self) -> Result<Vec<CategoryResponseDto>> {
        let categories = sqlx::query_as!(
            Category,
            r#"
            SELECT id, parent_id, name, slug, description, icon, color, display_order, is_active, created_at, updated_at
            FROM categories
            WHERE is_active = TRUE
            ORDER BY display_order, name
            "#
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to list categories: {:?}", e);
            AppError::Database(e)
        })?;

        Ok(categories.into_iter().map(|c| c.into()).collect())
    }

    /// List all active categories as tree structure
    pub async fn list_tree(&self) -> Result<Vec<CategoryTreeDto>> {
        let categories = sqlx::query_as!(
            Category,
            r#"
            SELECT id, parent_id, name, slug, description, icon, color, display_order, is_active, created_at, updated_at
            FROM categories
            WHERE is_active = TRUE
            ORDER BY display_order, name
            "#
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to list categories: {:?}", e);
            AppError::Database(e)
        })?;

        Ok(CategoryTreeDto::build_tree(categories))
    }

    /// Get category by slug
    pub async fn get_by_slug(&self, slug: &str) -> Result<CategoryResponseDto> {
        let category = sqlx::query_as!(
            Category,
            r#"
            SELECT id, parent_id, name, slug, description, icon, color, display_order, is_active, created_at, updated_at
            FROM categories
            WHERE slug = $1 AND is_active = TRUE
            "#,
            slug
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get category by slug: {:?}", e);
            AppError::Database(e)
        })?;

        category
            .map(|c| c.into())
            .ok_or_else(|| AppError::NotFound(format!("Category '{}' not found", slug)))
    }
}
