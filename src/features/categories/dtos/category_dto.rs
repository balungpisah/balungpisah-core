use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::features::categories::models::Category;

/// Response DTO for category
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CategoryResponseDto {
    pub id: Uuid,
    pub parent_id: Option<Uuid>,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub display_order: i32,
}

impl From<Category> for CategoryResponseDto {
    fn from(c: Category) -> Self {
        Self {
            id: c.id,
            parent_id: c.parent_id,
            name: c.name,
            slug: c.slug,
            description: c.description,
            icon: c.icon,
            color: c.color,
            display_order: c.display_order,
        }
    }
}

/// Response DTO for category tree (hierarchical structure)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[schema(no_recursion)]
pub struct CategoryTreeDto {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub display_order: i32,
    pub children: Vec<CategoryTreeDto>,
}

impl CategoryTreeDto {
    /// Build tree from flat list of categories
    pub fn build_tree(categories: Vec<Category>) -> Vec<CategoryTreeDto> {
        // Get root categories (parent_id is None)
        let roots: Vec<&Category> = categories
            .iter()
            .filter(|c| c.parent_id.is_none())
            .collect();

        // Build tree recursively
        roots
            .into_iter()
            .map(|root| Self::build_node(root, &categories))
            .collect()
    }

    fn build_node(category: &Category, all_categories: &[Category]) -> CategoryTreeDto {
        let children: Vec<CategoryTreeDto> = all_categories
            .iter()
            .filter(|c| c.parent_id == Some(category.id))
            .map(|child| Self::build_node(child, all_categories))
            .collect();

        CategoryTreeDto {
            id: category.id,
            name: category.name.clone(),
            slug: category.slug.clone(),
            description: category.description.clone(),
            icon: category.icon.clone(),
            color: category.color.clone(),
            display_order: category.display_order,
            children,
        }
    }
}
