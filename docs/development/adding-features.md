# Adding New Features

Step-by-step guide to adding new features to the codebase.

## Overview

Each feature follows a consistent structure (feature names should be plural):
```
src/features/<feature_name_plural>/
├── dtos/
│   ├── <feature_name_singular>_dto.rs       # Request/response DTOs
│   └── mod.rs
├── models/
│   ├── <feature_name_singular>_model.rs     # Database models
│   └── mod.rs
├── services/
│   ├── <feature_name_singular>_service.rs   # Business logic
│   ├── <feature_name_singular>_service_test.rs  # Service tests
│   └── mod.rs
├── handlers/
│   ├── <feature_name_singular>_handler.rs   # HTTP handlers
│   ├── <feature_name_singular>_handler_test.rs  # Handler tests
│   └── mod.rs
├── routes.rs                                 # Route definitions
├── test_utils.rs                             # Test utilities
└── mod.rs                                    # Module exports
```

Example: For a "products" feature, the structure would be:
```
src/features/products/
├── dtos/
│   ├── product_dto.rs
│   └── mod.rs
├── models/
│   ├── product_model.rs
│   └── mod.rs
├── services/
│   ├── product_service.rs
│   ├── product_service_test.rs
│   └── mod.rs
├── handlers/
│   ├── product_handler.rs
│   ├── product_handler_test.rs
│   └── mod.rs
├── routes.rs
├── test_utils.rs
└── mod.rs
```

## Step-by-Step Guide

### Step 1: Create Migration

Create a database migration for your feature.

```bash
sqlx migrate add create_<feature>_table
```

Edit `migrations/<timestamp>_create_<feature>_table.sql`:

```sql
CREATE TABLE products (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id UUID NOT NULL,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    price DECIMAL(10, 2) NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted_at TIMESTAMPTZ,
    CONSTRAINT fk_org_id FOREIGN KEY (org_id) REFERENCES organizations(id)
);

CREATE INDEX idx_products_org_id ON products(org_id);
CREATE INDEX idx_products_deleted_at ON products(deleted_at);
```

Run migration:
```bash
sqlx migrate run
```

### Step 2: Create Feature Directory

```bash
mkdir -p src/features/products/{dtos,models,services,handlers}
touch src/features/products/dtos/{product_dto.rs,mod.rs}
touch src/features/products/models/{product_model.rs,mod.rs}
touch src/features/products/services/{product_service.rs,mod.rs}
touch src/features/products/handlers/{product_handler.rs,mod.rs}
touch src/features/products/{routes.rs,mod.rs}
```

### Step 3: Define Model (`models/product_model.rs`)

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Product {
    pub id: Uuid,
    pub org_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub price: rust_decimal::Decimal,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}
```

### Step 4: Define DTOs (`dtos/product_dto.rs`)

```rust
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CreateProductDto {
    #[validate(length(min = 1, max = 255))]
    pub name: String,
    pub description: Option<String>,
    #[validate(range(min = 0.01))]
    pub price: Decimal,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ProductResponseDto {
    pub id: Uuid,
    pub org_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub price: Decimal,
    pub is_active: bool,
}

impl From<Product> for ProductResponseDto {
    fn from(product: Product) -> Self {
        Self {
            id: product.id,
            org_id: product.org_id,
            name: product.name,
            description: product.description,
            price: product.price,
            is_active: product.is_active,
        }
    }
}
// Add UpdateProductDto similarly
```

### Step 5: Implement Service (`services/product_service.rs`)

```rust
pub struct ProductService {
    pool: PgPool,
}

impl ProductService {
    pub fn new(pool: PgPool) -> Self { Self { pool } }

    pub async fn create(&self, org_id: Uuid, dto: CreateProductDto) -> Result<ProductResponseDto> {
        let product = sqlx::query_as!(
            Product,
            "INSERT INTO products (org_id, name, description, price) VALUES ($1, $2, $3, $4) RETURNING *",
            org_id, dto.name, dto.description, dto.price
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to create: {}", e)))?;
        Ok(product.into())
    }

    pub async fn get_by_id(&self, org_id: Uuid, id: Uuid) -> Result<ProductResponseDto> {
        let product = sqlx::query_as!(
            Product,
            "SELECT * FROM products WHERE id = $1 AND org_id = $2 AND deleted_at IS NULL",
            id, org_id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?
        .ok_or_else(|| AppError::NotFound("Product not found".to_string()))?;
        Ok(product.into())
    }

    // Similar pattern for update() and delete()
}
```

### Step 6: Create Handlers (`handlers/product_handler.rs`)

```rust
#[utoipa::path(post, path = "/api/products", request_body = CreateProductDto)]
pub async fn create_product(
    AuthUser(user): AuthUser,
    State(service): State<Arc<ProductService>>,
    ValidatedJson(dto): ValidatedJson<CreateProductDto>,
) -> Result<Json<ApiResponse<ProductResponseDto>>> {
    let product = service.create(user.org_id, dto).await?;
    Ok(Json(ApiResponse::success(product, None)))
}

#[utoipa::path(get, path = "/api/products/{id}")]
pub async fn get_product(
    AuthUser(user): AuthUser,
    State(service): State<Arc<ProductService>>,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<ProductResponseDto>>> {
    let product = service.get_by_id(user.org_id, id).await?;
    Ok(Json(ApiResponse::success(product, None)))
}
```

### Step 7: Define Routes (`routes.rs`)

```rust
use crate::features::products::handlers;
use crate::features::products::services::ProductService;
use axum::{routing::{get, post}, Router};
use std::sync::Arc;

pub fn routes(service: Arc<ProductService>) -> Router {
    Router::new()
        .route("/api/products", post(handlers::create_product))
        .route("/api/products/:id", get(handlers::get_product))
        .with_state(service)
}
```

### Step 8: Export Modules

Create `mod.rs` in each subfolder to export the files:

**`dtos/mod.rs`:**
```rust
mod product_dto;
pub use product_dto::*;
```

**`models/mod.rs`:**
```rust
mod product_model;
pub use product_model::*;
```

**`services/mod.rs`:**
```rust
mod product_service;
pub use product_service::*;
```

**`handlers/mod.rs`:**
```rust
mod product_handler;
pub use product_handler::*;
```

**Main `mod.rs`:**
```rust
pub mod dtos;
pub mod handlers;
pub mod models;
pub mod routes;
pub mod services;

#[cfg(test)]
pub mod test_utils;
```

### Step 9: Register in `src/features/mod.rs`

```rust
pub mod products;
// ... other modules
```

### Step 10: Register in `main.rs`

```rust
// Initialize service
let products_service = Arc::new(products::services::ProductService::new(pool.clone()));

// Register routes
let app = Router::new()
    .merge(products::routes::routes(products_service.clone()))
    .route_layer(middleware::from_fn_with_state(jwt_validator.clone(), auth_middleware));

// Register in OpenAPI
#[derive(OpenApi)]
#[openapi(
    paths(
        products::handlers::create_product,
        products::handlers::get_product,
    ),
    components(schemas(
        products::dtos::CreateProductDto,
        products::dtos::ProductResponseDto,
    ))
)]
struct ApiDoc;
```

## Testing Your Feature

Create test files in their respective folders:
```bash
touch src/features/products/services/product_service_test.rs
touch src/features/products/handlers/product_handler_test.rs
touch src/features/products/test_utils.rs
```

Update each `mod.rs` to include test modules:
- **`services/mod.rs`:** Add `#[cfg(test)] mod product_service_test;`
- **`handlers/mod.rs`:** Add `#[cfg(test)] mod product_handler_test;`

See [Testing Guide](./testing.md) for testing patterns.

## Best Practices

1. **Use soft deletes** - Add `deleted_at` column for logical deletion
2. **Validate inputs** - Use `validator` crate on DTOs
3. **Scope by organization** - Always filter by `org_id` for multi-tenant data
4. **Return DTOs, not models** - Never expose database models to API
5. **Handle errors properly** - Return appropriate `AppError` variants
6. **Document with OpenAPI** - Use `#[utoipa::path]` on all handlers
7. **Write tests** - Test both success and error cases

## Common Patterns

### Pagination
```rust
pub async fn list(&self, org_id: Uuid, page: i64, per_page: i64) -> Result<Vec<ProductResponseDto>> {
    let offset = (page - 1) * per_page;
    // ... query with LIMIT and OFFSET
}
```

### Search/Filtering
```rust
let mut query = QueryBuilder::new("SELECT * FROM products WHERE org_id = ");
query.push_bind(org_id);

if let Some(search) = filter.search {
    query.push(" AND name ILIKE ");
    query.push_bind(format!("%{}%", search));
}
```

### Transactions
```rust
let mut tx = self.pool.begin().await?;
// ... multiple queries
tx.commit().await?;
```

---

For architecture details, see [Architecture Guide](./architecture.md).
