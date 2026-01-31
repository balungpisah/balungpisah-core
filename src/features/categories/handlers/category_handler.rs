use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;

use crate::core::error::Result;
use crate::features::categories::dtos::CategoryResponseDto;
use crate::features::categories::services::CategoryService;
use crate::shared::types::ApiResponse;

/// Query params for listing categories
#[derive(Debug, Deserialize)]
pub struct ListCategoriesQuery {
    /// If true, return tree structure. Default: false (flat list)
    #[serde(default)]
    pub tree: bool,
}

/// List all active categories
///
/// Returns categories as flat list or tree structure based on `tree` query param.
#[utoipa::path(
    get,
    path = "/api/categories",
    params(
        ("tree" = Option<bool>, Query, description = "Return tree structure if true")
    ),
    responses(
        (status = 200, description = "List of categories", body = ApiResponse<Vec<CategoryResponseDto>>),
    ),
    tag = "categories"
)]
pub async fn list_categories(
    State(service): State<Arc<CategoryService>>,
    Query(query): Query<ListCategoriesQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>> {
    if query.tree {
        let tree = service.list_tree().await?;
        let value = serde_json::to_value(tree).unwrap();
        Ok(Json(ApiResponse::success(Some(value), None, None)))
    } else {
        let categories = service.list().await?;
        let value = serde_json::to_value(categories).unwrap();
        Ok(Json(ApiResponse::success(Some(value), None, None)))
    }
}

/// Get category by slug
#[utoipa::path(
    get,
    path = "/api/categories/{slug}",
    params(
        ("slug" = String, Path, description = "Category slug")
    ),
    responses(
        (status = 200, description = "Category found", body = ApiResponse<CategoryResponseDto>),
        (status = 404, description = "Category not found")
    ),
    tag = "categories"
)]
pub async fn get_category(
    State(service): State<Arc<CategoryService>>,
    Path(slug): Path<String>,
) -> Result<Json<ApiResponse<CategoryResponseDto>>> {
    let category = service.get_by_slug(&slug).await?;
    Ok(Json(ApiResponse::success(Some(category), None, None)))
}
